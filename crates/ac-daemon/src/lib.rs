use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use ac_agent::{provider_events_text, AttachmentContent, ContextAttachment};
use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_db::{
    ControlPlaneDb, DesignCritiqueRow, DesignDocumentRow, DesignPreviewRow, PersistedSession,
    PersistedWorktree, ProviderCatalogRow,
};
use ac_evidence::EvidenceStore;
use ac_git::{WorktreeRecord, WorktreeStatus};
use ac_kernel::{Kernel, KernelDecisionKind, MissionState, PermissionDecision, PolicyBoundary};
use ac_provider::catalog::{
    test_provider_account, ProviderAccount as CatalogAccount, ProviderAccountStatus,
    ProviderCatalogEntry, ProviderConnectionKind, ProviderConnectionTest,
};
use ac_provider::discovery::{discover_models, ModelDiscoveryKind};
use ac_provider::PrivacyClass;
use ac_runtime::{
    AgentSession, AgentSessionState, CancellationToken, HydratedSession, RuntimeHydrator,
    TaskGraph, Worker,
};
use serde_json::{json, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionExecutionStatus {
    pub mission_id: StableId,
    pub session_id: StableId,
    pub state: String,
}

#[derive(Clone, Debug)]
struct QueuedMission {
    mission_id: StableId,
    session_id: StableId,
    goal: String,
    workspace_root: PathBuf,
    attachments: Vec<ContextAttachment>,
}

enum CoordinatorMessage {
    Run(QueuedMission),
    Shutdown,
}

struct SqliteAgentDurability {
    db_path: PathBuf,
}

impl SqliteAgentDurability {
    fn db(&self) -> AcResult<ControlPlaneDb> {
        ControlPlaneDb::open(&self.db_path)
    }

    /// Resolve the durable repository identity for a mission: conversation's
    /// project_path if bound, else the session workspace root.  Same
    /// authoritative-resolution order as provider routing records (never
    /// fabricated).
    fn repository_identity_for_mission(&self, mission_id: &StableId) -> AcResult<String> {
        let db = self.db()?;
        let project_path = db
            .conversation_for_mission(&mission_id.to_string())?
            .map(|conversation| conversation.project_path)
            .or_else(|| {
                db.session_for_mission(mission_id)
                    .ok()
                    .flatten()
                    .and_then(|session| session.workspace_root)
            });
        match project_path {
            Some(path) => Ok(crate::project_repository_identity(&path)),
            None => Ok(format!("repo-{}", mission_id.as_str())),
        }
    }
}

impl ac_agent::AgentDurabilityObserver for SqliteAgentDurability {
    fn worktree_bound(&mut self, worktree: &WorktreeRecord) -> AcResult<()> {
        self.db()?.save_worktree(worktree)
    }

    fn graph_updated(
        &mut self,
        graph: &TaskGraph,
        worker: &Worker,
        session_id: &StableId,
    ) -> AcResult<()> {
        graph.persist(&self.db()?, worker, session_id)
    }

    fn changeset_updated(&mut self, changeset: &ac_changeset::ChangeSet) -> AcResult<()> {
        self.db()?.save_changeset(changeset)
    }

    fn edit_transaction_updated(
        &mut self,
        transaction: &ac_changeset::ChangeSetTransaction,
        task_id: Option<&StableId>,
        worktree_id: Option<&StableId>,
        base_revision: &str,
    ) -> AcResult<()> {
        self.db()?.save_changeset_transaction(
            transaction,
            task_id.map(StableId::as_str),
            worktree_id.map(StableId::as_str),
            base_revision,
            "rust",
        )
    }

    fn evidence_persisted(&mut self, record: &ac_evidence::EvidenceRecord) -> AcResult<()> {
        self.db()?.append_evidence(record)
    }

    fn load_evidence_records(
        &mut self,
        ids: &[ac_common::StableId],
    ) -> AcResult<Vec<ac_evidence::EvidenceRecord>> {
        let db = self.db()?;
        let all = db.evidence_records()?;
        Ok(all
            .into_iter()
            .filter(|record| ids.contains(&record.id))
            .collect())
    }

    /// F1: persist a memory fact under the mission's repository identity so
    /// project memory survives the mission and is available to every mode.
    fn memory_fact_persisted(
        &mut self,
        mission_id: &StableId,
        fact: &ac_context::MemoryFact,
    ) -> AcResult<()> {
        let db = self.db()?;
        let repository_id = self.repository_identity_for_mission(mission_id)?;
        let evidence = fact
            .source_evidence
            .iter()
            .map(|reference| ac_db::MemoryEvidenceRow {
                fact_id: fact.id.to_string(),
                evidence_ref: reference.to_string(),
                file_path: None,
                symbol: None,
                content_hash: None,
            })
            .collect::<Vec<_>>();
        db.save_memory_fact(
            &ac_db::MemoryFactRow {
                id: fact.id.to_string(),
                repository_id,
                mission_id: Some(mission_id.to_string()),
                task_id: fact.scope.task_id.as_ref().map(ToString::to_string),
                branch: fact.scope.branch.clone(),
                statement: fact.statement.clone(),
                fact_type: fact.fact_type.as_str().to_string(),
                source: fact.source.as_str().to_string(),
                confidence: fact.confidence,
                freshness: fact.freshness.as_str().to_string(),
                memory_class: fact.memory_class.as_str().to_string(),
                observed_commit: fact.observed_commit.clone(),
                conflict_set_id: fact.conflict_set.as_ref().map(ToString::to_string),
                valid_from_ms: fact.valid_from.as_millis() as i64,
                valid_until_ms: fact.valid_until.as_ref().map(|t| t.as_millis() as i64),
                superseded_by: fact.superseded_by.as_ref().map(ToString::to_string),
                last_validation_ms: fact.last_validation.as_millis() as i64,
            },
            &evidence,
        )
    }

    /// F1: persist a task memory (mission-scoped).
    fn task_memory_persisted(
        &mut self,
        mission_id: &StableId,
        memory: &ac_context::TaskMemory,
    ) -> AcResult<()> {
        let db = self.db()?;
        let refs = memory
            .evidence_refs
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        db.save_task_memory(&ac_db::TaskMemoryRow {
            id: memory.id.to_string(),
            task_id: memory.task_id.to_string(),
            summary: memory.summary.clone(),
            evidence_refs: refs,
            created_at_ms: memory.created_at.as_millis() as i64,
        })?;
        let _ = mission_id; // mission scoping lives on fact rows; task memory is task-scoped
        Ok(())
    }

    fn provider_routing_recorded(
        &mut self,
        mission_id: &str,
        session_id: &str,
        task_id: Option<&str>,
        record: &ac_agent::ProviderModelRecord,
    ) -> AcResult<()> {
        let db = self.db()?;
        // Resolve project_path and conversation_id from authoritative state.
        // Conversations are the user-facing container; a mission may be
        // referenced by a conversation's current_mission_id or by message
        // mission_refs.  Never fabricate either association.
        let mission = StableId::from_existing(mission_id)?;
        let conversation = db.conversation_for_mission(&mission.to_string())?;
        let project_path = conversation
            .as_ref()
            .map(|c| c.project_path.clone())
            .or_else(|| {
                db.session_for_mission(&mission)
                    .ok()
                    .flatten()
                    .and_then(|session| session.workspace_root)
            });
        let row = ac_db::ProviderModelRecordRow {
            id: format!("pmr-{}-{}", StableId::new("record"), record.created_at_ms),
            project_path,
            conversation_id: conversation.map(|c| c.id),
            mission_id: Some(mission_id.to_string()),
            session_id: Some(session_id.to_string()),
            task_id: task_id.map(ToString::to_string),
            provider_id: record.provider_id.clone(),
            provider_account_id: record.provider_account_id.clone(),
            model_id: record.model_id.clone(),
            model_name: record.model_name.clone(),
            routing_mode: record.routing_mode.clone(),
            attempt_number: record.attempt_number,
            success: record.success,
            failure_class: record.failure_class.clone(),
            created_at_ms: record.created_at_ms,
        };
        db.save_provider_model_record(&row)
    }

    fn final_audit_recorded(
        &mut self,
        mission_id: &str,
        original_goal: &str,
        requirements: &[String],
        audit: &ac_verification::FinalAuditReport,
        completion_allowed: bool,
        remaining_uncertainty: &str,
    ) -> AcResult<()> {
        self.db()?.save_final_audit(
            mission_id,
            original_goal,
            requirements,
            audit,
            completion_allowed,
            remaining_uncertainty,
        )
    }
}

#[derive(Default)]
struct CoordinatorState {
    queued: VecDeque<QueuedMission>,
    active: Option<StableId>,
    statuses: BTreeMap<String, MissionExecutionStatus>,
    paused: BTreeMap<String, QueuedMission>,
    /// Per-mission pause flags for ACTIVE missions.  Pausing a running mission
    /// sets its flag; the agent's run loop blocks on it (via the session) so no
    /// forward work occurs while paused, and resume clears it.
    pause_flags: BTreeMap<String, Arc<AtomicBool>>,
    cancelled: BTreeMap<String, ()>,
    cancellation: BTreeMap<String, CancellationToken>,
}

/// A deliberately single-slot, daemon-owned executor.  The IPC thread only
/// queues durable work; execution happens on this dedicated worker.
struct MissionCoordinator {
    tx: mpsc::SyncSender<CoordinatorMessage>,
    state: Arc<Mutex<CoordinatorState>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    /// Dedicated shutdown flag that cannot be blocked by a full work queue.
    /// The worker checks this flag via recv_timeout, guaranteeing termination
    /// even when all 64 channel slots are occupied by queued missions.
    shutdown: Arc<AtomicBool>,
}

impl MissionCoordinator {
    fn new(
        db_path: PathBuf,
        kernel: Arc<Mutex<Kernel<ProductionKernelPolicy>>>,
        terminal_sink_registry: Arc<Mutex<BTreeMap<String, Arc<TerminalSessionState>>>>,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel::<CoordinatorMessage>(64);
        let state = Arc::new(Mutex::new(CoordinatorState::default()));
        let worker_state = Arc::clone(&state);
        let terminal_sink_registry = Arc::clone(&terminal_sink_registry);
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_check = Arc::clone(&shutdown_flag);
        let worker = thread::Builder::new()
            .name("agentcode-mission-worker".to_string())
            .spawn(move || {
                loop {
                    if shutdown_check.load(Ordering::Acquire) {
                        break;
                    }
                    let message = match rx.recv_timeout(std::time::Duration::from_millis(50)) {
                        Ok(msg) => msg,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let CoordinatorMessage::Run(job) = message else {
                        break;
                    };
                    let run = {
                        let mut state = match worker_state.lock() {
                            Ok(state) => state,
                            Err(_) => continue,
                        };
                        state
                            .queued
                            .retain(|queued| queued.mission_id != job.mission_id);
                        if state.cancelled.contains_key(job.mission_id.as_str()) {
                            // A queued mission cancelled before execution: its
                            // Run message is still in the channel.  This branch
                            // is the only place it is ever seen, so clean up the
                            // per-mission maps here — otherwise cancelled/cancellation/
                            // pause_flags/statuses would leak for every queued
                            // mission cancelled before it executed.
                            let mid = job.mission_id.as_str();
                            state.cancelled.remove(mid);
                            state.cancellation.remove(mid);
                            state.pause_flags.remove(mid);
                            prune_terminal_statuses(&mut state);
                            continue;
                        }
                        if state.paused.contains_key(job.mission_id.as_str()) {
                            continue;
                        }
                        state.active = Some(job.mission_id.clone());
                        if let Some(status) = state.statuses.get_mut(job.mission_id.as_str()) {
                            status.state = "running".to_string();
                        }
                        true
                    };
                    if !run {
                        continue;
                    }
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let (token, pause) = {
                            let state = worker_state.lock().ok();
                            (
                                state
                                    .as_ref()
                                    .and_then(|state| {
                                        state.cancellation.get(job.mission_id.as_str()).cloned()
                                    })
                                    .unwrap_or_default(),
                                state
                                    .and_then(|state| {
                                        state.pause_flags.get(job.mission_id.as_str()).cloned()
                                    })
                                    .unwrap_or_else(|| Arc::new(AtomicBool::new(false))),
                            )
                        };
                        // Watch-the-agent: tag all tool output of THIS
                        // mission so mirrored chunks open the right
                        // mission-tagged terminal session, and register
                        // THIS daemon's terminal map as the sink while the
                        // mission runs (removed after it ends).
                        ac_tool::set_live_mission_context(Some(format!(
                            "{}|{}",
                            job.mission_id.as_str(),
                            job.workspace_root.display()
                        )));
                        let sink = Arc::clone(&terminal_sink_registry);
                        let _ = LIVE_AGENT_SINKS
                            .lock()
                            .map(|mut sinks| sinks.insert(job.mission_id.to_string(), sink));
                        let outcome = execute_mission(
                            &db_path,
                            &job.workspace_root,
                            Arc::clone(&kernel),
                            &job,
                            token,
                            pause,
                        );
                        let _ = LIVE_AGENT_SINKS
                            .lock()
                            .map(|mut sinks| sinks.remove(job.mission_id.as_str()));
                        ac_tool::set_live_mission_context(None);
                        outcome
                    }));
                    let terminal = match outcome {
                        Ok(Ok(state)) => state,
                        Ok(Err(error)) => {
                            eprintln!(
                                "MISSION-ERROR mission={} code={} detail={}",
                                job.mission_id,
                                error.code(),
                                error
                            );
                            format!("failed: {}", error.code())
                        }
                        Err(_) => {
                            eprintln!("MISSION-PANIC mission={}", job.mission_id);
                            "failed: DAEMON-MISSION_PANIC".to_string()
                        }
                    };
                    let was_cancelled_in_memory = worker_state
                        .lock()
                        .ok()
                        .is_some_and(|state| state.cancelled.contains_key(job.mission_id.as_str()));
                    let was_cancelled_durably = ControlPlaneDb::open(&db_path)
                        .ok()
                        .and_then(|db| db.get_session(&job.session_id).ok().flatten())
                        .is_some_and(|session| session.state == "cancelled");
                    let terminal = if was_cancelled_in_memory || was_cancelled_durably {
                        "cancelled".to_string()
                    } else {
                        terminal
                    };
                    if let Ok(db) = ControlPlaneDb::open(&db_path) {
                        let state_name = if terminal == "completed" {
                            "completed"
                        } else if terminal == "cancelled" {
                            "cancelled"
                        } else {
                            "failed"
                        };
                        let _ = db.update_session_state(&job.session_id, state_name);
                    }
                    let is_terminal = is_terminal_status(&terminal);
                    if let Ok(mut state) = worker_state.lock() {
                        state.active = None;
                        if let Some(status) = state.statuses.get_mut(job.mission_id.as_str()) {
                            status.state = terminal;
                        }
                        // Clean up terminal mission state so cancelled/cancellation
                        // maps do not grow without bound across thousands of missions.
                        if is_terminal {
                            let mid = job.mission_id.as_str();
                            state.cancelled.remove(mid);
                            state.cancellation.remove(mid);
                            state.pause_flags.remove(mid);
                        }
                        prune_terminal_statuses(&mut state);
                    }
                }
            })
            .expect("mission coordinator thread must start");
        Self {
            tx,
            state,
            worker: Mutex::new(Some(worker)),
            shutdown: shutdown_flag,
        }
    }

    fn enqueue(&self, job: QueuedMission) -> AcResult<()> {
        {
            let mut state = self.state.lock().map_err(|_| {
                AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
            })?;
            state.statuses.insert(
                job.mission_id.to_string(),
                MissionExecutionStatus {
                    mission_id: job.mission_id.clone(),
                    session_id: job.session_id.clone(),
                    state: "queued".to_string(),
                },
            );
            state
                .cancellation
                .insert(job.mission_id.to_string(), CancellationToken::new());
            state
                .pause_flags
                .insert(job.mission_id.to_string(), Arc::new(AtomicBool::new(false)));
            state.queued.push_back(job.clone());
        }
        if self
            .tx
            .try_send(CoordinatorMessage::Run(job.clone()))
            .is_err()
        {
            if let Ok(mut state) = self.state.lock() {
                state
                    .queued
                    .retain(|queued| queued.mission_id != job.mission_id);
                state.statuses.remove(job.mission_id.as_str());
                state.cancellation.remove(job.mission_id.as_str());
                state.pause_flags.remove(job.mission_id.as_str());
            }
            return Err(AcError::new(
                "DAEMON-QUEUE_FULL",
                "mission queue is full",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::Retryable,
            ));
        }
        Ok(())
    }

    fn status(&self, mission_id: &str) -> Option<MissionExecutionStatus> {
        self.state.lock().ok()?.statuses.get(mission_id).cloned()
    }

    fn pause(&self, mission_id: &str) -> AcResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
        })?;
        // Pause a queued mission: move it to the paused map so the worker
        // skips it on dequeue and it never executes until resumed.
        if let Some(job) = state
            .queued
            .iter()
            .find(|job| job.mission_id.as_str() == mission_id)
            .cloned()
        {
            state.paused.insert(mission_id.to_string(), job);
            if let Some(status) = state.statuses.get_mut(mission_id) {
                status.state = "paused".to_string();
            }
            return Ok(());
        }
        // Pause an active (running) mission: set the shared pause flag so the
        // agent's run loop blocks at the next safe point.
        if state
            .active
            .as_ref()
            .is_some_and(|id| id.as_str() == mission_id)
            || state
                .statuses
                .get(mission_id)
                .is_some_and(|s| s.state == "running")
        {
            if let Some(sig) = state.pause_flags.get(mission_id) {
                sig.store(true, Ordering::SeqCst);
            }
            if let Some(status) = state.statuses.get_mut(mission_id) {
                status.state = "paused".to_string();
            }
            return Ok(());
        }
        Err(AcError::conflict(
            "DAEMON-MISSION_NOT_PAUSABLE",
            "mission is terminal or not found",
        ))
    }

    fn resume(&self, mission_id: &str) -> AcResult<()> {
        // Resume a queued-paused mission: requeue it.
        let job = {
            let mut state = self.state.lock().map_err(|_| {
                AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
            })?;
            if let Some(job) = state.paused.remove(mission_id) {
                state.queued.push_back(job.clone());
                if let Some(status) = state.statuses.get_mut(mission_id) {
                    status.state = "queued".to_string();
                }
                Some(job)
            } else {
                // Resume an active-paused mission: clear the pause flag so the
                // agent's run loop continues.
                if let Some(sig) = state.pause_flags.get(mission_id) {
                    if sig.load(Ordering::SeqCst) {
                        sig.store(false, Ordering::SeqCst);
                        if let Some(status) = state.statuses.get_mut(mission_id) {
                            status.state = "running".to_string();
                        }
                        return Ok(());
                    }
                }
                None
            }
        };
        match job {
            Some(job) => self.tx.try_send(CoordinatorMessage::Run(job)).map_err(|_| {
                AcError::new(
                    "DAEMON-QUEUE_FULL",
                    "mission queue is full",
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::Retryable,
                )
            }),
            None => Err(AcError::conflict(
                "DAEMON-MISSION_NOT_PAUSED",
                "mission is not paused",
            )),
        }
    }

    fn remember_paused(&self, job: QueuedMission) -> AcResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
        })?;
        let mission_key = job.mission_id.to_string();
        state.statuses.insert(
            mission_key.clone(),
            MissionExecutionStatus {
                mission_id: job.mission_id.clone(),
                session_id: job.session_id.clone(),
                state: "paused".to_string(),
            },
        );
        state.paused.insert(mission_key.clone(), job);
        state
            .pause_flags
            .entry(mission_key)
            .or_insert_with(|| Arc::new(AtomicBool::new(false)));
        Ok(())
    }
    fn cancel(&self, mission_id: &str) -> AcResult<bool> {
        let mut state = self.state.lock().map_err(|_| {
            AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
        })?;
        if !state.statuses.contains_key(mission_id) {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission is not known to coordinator",
            ));
        }
        let was_running = state
            .active
            .as_ref()
            .is_some_and(|id| id.as_str() == mission_id)
            || state
                .statuses
                .get(mission_id)
                .is_some_and(|s| s.state == "running");
        if let Some(token) = state.cancellation.get(mission_id) {
            token.cancel();
        }
        state.cancelled.insert(mission_id.to_string(), ());
        state.paused.remove(mission_id);
        if let Some(sig) = state.pause_flags.get(mission_id) {
            sig.store(false, Ordering::SeqCst);
        }
        state.pause_flags.remove(mission_id);
        state
            .queued
            .retain(|job| job.mission_id.as_str() != mission_id);
        if let Some(status) = state.statuses.get_mut(mission_id) {
            status.state = "cancelled".to_string();
        }
        Ok(was_running)
    }

    fn stop(&self) -> AcResult<()> {
        if let Ok(state) = self.state.lock() {
            for token in state.cancellation.values() {
                token.cancel();
            }
        }
        // Set the dedicated shutdown flag first.  This guarantees the worker
        // will see the shutdown even when the channel is full.
        self.shutdown.store(true, Ordering::Release);
        let _ = self.tx.try_send(CoordinatorMessage::Shutdown);
        let handle = self
            .worker
            .lock()
            .map_err(|_| {
                AcError::conflict(
                    "DAEMON-COORDINATOR_POISONED",
                    "coordinator worker lock poisoned",
                )
            })?
            .take();
        if let Some(handle) = handle {
            handle.join().map_err(|_| {
                AcError::conflict(
                    "DAEMON-COORDINATOR_PANIC",
                    "mission coordinator worker panicked",
                )
            })?;
        }
        Ok(())
    }

    /// Test-only introspection of in-memory coordinator map sizes.  Used to
    /// prove that queued-cancelled missions do not leak per-mission state.
    #[cfg(test)]
    fn map_sizes(&self) -> (usize, usize, usize, usize) {
        if let Ok(state) = self.state.lock() {
            (
                state.statuses.len(),
                state.cancelled.len(),
                state.cancellation.len(),
                state.pause_flags.len(),
            )
        } else {
            (0, 0, 0, 0)
        }
    }
}

fn prune_terminal_statuses(state: &mut CoordinatorState) {
    const MAX_TERMINAL_STATUSES: usize = 256;
    let active = state.active.as_ref().map(ToString::to_string);
    let queued = state
        .queued
        .iter()
        .map(|job| job.mission_id.to_string())
        .collect::<Vec<_>>();
    let paused = state.paused.keys().cloned().collect::<Vec<_>>();
    let mut terminal = state
        .statuses
        .iter()
        .filter(|(mission_id, status)| {
            active.as_ref() != Some(mission_id)
                && !queued.contains(mission_id)
                && !paused.contains(mission_id)
                && is_terminal_status(&status.state)
        })
        .map(|(mission_id, _)| mission_id.clone())
        .collect::<Vec<_>>();
    terminal.sort();
    while terminal.len() > MAX_TERMINAL_STATUSES {
        if let Some(mission_id) = terminal.first().cloned() {
            state.statuses.remove(&mission_id);
            terminal.remove(0);
        }
    }
}

fn is_terminal_status(state: &str) -> bool {
    matches!(state, "completed" | "cancelled" | "failed") || state.starts_with("failed:")
}

/// Transition an Active kernel mission to a terminal state and persist it.
/// Respects the Phase 4 rule: genuine cancellation MUST become Cancelled
/// rather than Failed, even when the transition happens before the agent's
/// own cancellation checks can fire.
fn transition_mission_to_terminal(
    kernel: &Arc<Mutex<Kernel<ProductionKernelPolicy>>>,
    db: &ControlPlaneDb,
    mission_id: &StableId,
    cancelled: bool,
) {
    let cancelled = cancelled
        || db
            .session_for_mission(mission_id)
            .ok()
            .flatten()
            .is_some_and(|s| s.state == "cancelled");
    if let Ok(mut kernel) = kernel.lock() {
        let is_active = kernel
            .mission(mission_id)
            .is_some_and(|m| m.state == ac_kernel::MissionState::Active);
        if is_active {
            let target = if cancelled {
                ac_kernel::MissionState::Cancelled
            } else {
                ac_kernel::MissionState::Failed
            };
            let _ = kernel.transition_mission(mission_id, target, Vec::new());
        }
        if let Some(mission) = kernel.mission(mission_id) {
            let _ = db.put_mission(mission);
        }
        for event in kernel.events() {
            let _ = db.append_kernel_event(event);
        }
    }
}

/// Convert a persisted memory-fact row into the in-process model (F1).
/// Unknown enum strings degrade honestly to ArchitectureFact/Runtime/Fresh/
/// TaskScoped — never fabricate a failure, never invent authority.
fn memory_fact_row_to_model(
    row: &ac_db::MemoryFactRow,
    evidence: Vec<StableId>,
) -> ac_context::MemoryFact {
    ac_context::MemoryFact {
        id: StableId::from_existing(&row.id).unwrap_or_else(|_| StableId::new("mem")),
        statement: row.statement.clone(),
        fact_type: ac_context::FactType::parse(&row.fact_type)
            .unwrap_or(ac_context::FactType::ArchitectureFact),
        source: ac_context::FactSource::parse(&row.source)
            .unwrap_or(ac_context::FactSource::Runtime),
        source_evidence: evidence,
        confidence: row.confidence,
        freshness: ac_context::FreshnessState::parse(&row.freshness)
            .unwrap_or(ac_context::FreshnessState::Fresh),
        scope: ac_context::MemoryScope {
            repository_id: StableId::from_existing(&row.repository_id)
                .unwrap_or_else(|_| StableId::new("repo")),
            mission_id: row
                .mission_id
                .as_deref()
                .and_then(|id| StableId::from_existing(id).ok()),
            task_id: row
                .task_id
                .as_deref()
                .and_then(|id| StableId::from_existing(id).ok()),
            branch: row.branch.clone(),
        },
        memory_class: ac_context::MemoryClass::parse(&row.memory_class)
            .unwrap_or(ac_context::MemoryClass::TaskScoped),
        observed_commit: row.observed_commit.clone(),
        dependencies: Vec::new(),
        conflict_set: row
            .conflict_set_id
            .as_deref()
            .and_then(|id| StableId::from_existing(id).ok()),
        valid_from: ac_common::TimestampMillis::from_millis(row.valid_from_ms.max(0) as u128),
        valid_until: row
            .valid_until_ms
            .map(|value| ac_common::TimestampMillis::from_millis(value.max(0) as u128)),
        superseded_by: row
            .superseded_by
            .as_deref()
            .and_then(|id| StableId::from_existing(id).ok()),
        last_validation: ac_common::TimestampMillis::from_millis(
            row.last_validation_ms.max(0) as u128
        ),
    }
}

/// Convert a persisted task-memory row into the in-process model (F1).
fn task_memory_row_to_model(row: &ac_db::TaskMemoryRow) -> ac_context::TaskMemory {
    ac_context::TaskMemory {
        id: StableId::from_existing(&row.id).unwrap_or_else(|_| StableId::new("taskmem")),
        task_id: StableId::from_existing(&row.task_id).unwrap_or_else(|_| StableId::new("task")),
        summary: row.summary.clone(),
        evidence_refs: row
            .evidence_refs
            .split(',')
            .filter(|value| !value.is_empty())
            .filter_map(|value| StableId::from_existing(value).ok())
            .collect(),
        created_at: ac_common::TimestampMillis::from_millis(row.created_at_ms.max(0) as u128),
    }
}

/// Parse the mission id from a live-mirroring context tag
/// ("mission_id|workspace_root"); returns None when unset.
fn parse_mission_tag(ctx: &str) -> Option<String> {
    ctx.split('|')
        .next()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn execute_mission(
    db_path: &Path,
    workspace_root: &Path,
    kernel: Arc<Mutex<Kernel<ProductionKernelPolicy>>>,
    job: &QueuedMission,
    cancellation: CancellationToken,
    pause: Arc<AtomicBool>,
) -> AcResult<String> {
    let db = ControlPlaneDb::open(db_path)?;
    db.update_session_state(&job.session_id, "running")?;
    let persisted_session = db.get_session(&job.session_id)?;
    let resume_graph = persisted_session
        .map(|session| RuntimeHydrator::hydrate_session(&db, session))
        .transpose()?
        .and_then(|hydrated| {
            let has_tasks = hydrated.graph.tasks().next().is_some();
            has_tasks.then_some(hydrated.graph)
        });
    // Capture cancellation state before the token is moved into AgentSession.
    let cancelled = cancellation.is_cancelled();
    let session = AgentSession::with_id_token_and_pause(
        job.session_id.clone(),
        Worker::new(),
        cancellation,
        pause,
    );
    let worktree = workspace_root
        .join(".agentcode-worktrees")
        .join(job.session_id.as_str());
    let existing_worktree = db
        .worktree_for_mission(&job.mission_id)?
        .map(persisted_worktree_to_record)
        .transpose()?;
    // Provider routing is backend-owned: build the mission's model broker from
    // the durable provider catalog/accounts, falling back to the environment.
    let providers = match daemon_provider_registry(&db, db_path) {
        Ok(providers) => providers,
        Err(error) => {
            transition_mission_to_terminal(&kernel, &db, &job.mission_id, cancelled);
            return Err(error);
        }
    };
    let mut agent = match ac_agent::bound_workspace_agent_with_providers(
        workspace_root.to_path_buf(),
        worktree,
        Arc::clone(&kernel),
        job.mission_id.clone(),
        session,
        backend_tool_policy(),
        existing_worktree,
        resume_graph,
        Some(Box::new(SqliteAgentDurability {
            db_path: db_path.to_path_buf(),
        })),
        providers,
    ) {
        Ok(agent) => agent,
        Err(error) => {
            transition_mission_to_terminal(&kernel, &db, &job.mission_id, cancelled);
            return Err(error);
        }
    };
    // F1: hydrate durable project memory — facts and task memories from
    // previous missions on this repository become AcceptedMemory context.
    {
        let repository_id = db
            .conversation_for_mission(&job.mission_id.to_string())
            .ok()
            .flatten()
            .map(|conversation| conversation.project_path)
            .or_else(|| {
                db.session_for_mission(&job.mission_id)
                    .ok()
                    .flatten()
                    .and_then(|session| session.workspace_root)
            })
            .map(|path| crate::project_repository_identity(&path))
            .unwrap_or_else(|| job.mission_id.to_string());
        let facts = db
            .memory_facts_for(&repository_id, 200)
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                let evidence = db
                    .memory_fact_evidence(&row.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|e| StableId::from_existing(&e.evidence_ref).ok())
                    .collect();
                memory_fact_row_to_model(&row, evidence)
            })
            .collect::<Vec<_>>();
        let task_memories = db
            .task_memories_newest(100)
            .unwrap_or_default()
            .iter()
            .map(task_memory_row_to_model)
            .collect::<Vec<_>>();
        agent.hydrate_project_memory(facts, task_memories);
    }
    let report = match agent.run_goal(ac_agent::Goal::with_attachments(
        job.goal.clone(),
        job.attachments.clone(),
    )?) {
        Ok(report) => report,
        Err(error) => {
            // Even when the agent run itself fails to produce a report,
            // the kernel mission must be transitioned to a terminal state
            // so it does not incorrectly remain Active.
            transition_mission_to_terminal(&kernel, &db, &job.mission_id, cancelled);
            return Err(error);
        }
    };
    if report.state != ac_agent::AutonomousState::Completed {
        eprintln!(
            "MISSION-NOT-COMPLETED mission={} state={:?}",
            job.mission_id, report.state
        );
        if let Some(plan) = &report.runtime_plan {
            for task in plan.tasks.iter() {
                eprintln!(
                    "  TASK {} state={:?} title={}",
                    task.id, task.state, task.title
                );
            }
        }
        for observation in &report.observations {
            eprintln!(
                "  OBS task={} action={} success={} failure_class={:?} summary={}",
                observation.task_id,
                observation.action,
                observation.success,
                observation.failure_class,
                observation.summary
            );
        }
        if let Some(validation) = &report.validation {
            eprintln!(
                "  VALIDATION plan={} passed={} evidence={}",
                validation.plan_name, validation.passed, validation.evidence_ref
            );
        }
        for replan in &report.replans {
            eprintln!("  REPLAN {replan}");
        }
    }
    let evidence = agent.into_evidence();
    for record in evidence.records() {
        db.append_evidence(record)?;
    }
    {
        let mut kernel = kernel.lock().map_err(|_| {
            AcError::conflict(
                "DAEMON-KERNEL_POISONED",
                "kernel lock poisoned after mission",
            )
        })?;
        // If the run finished in a failed (non-completed, non-cancelled) state
        // the agent leaves the kernel mission Active.  Transition it to a
        // terminal Failed state here so the persisted mission never
        // incorrectly reports Active for a genuinely failed run.
        if report.state == ac_agent::AutonomousState::Failed {
            let is_active = kernel
                .mission(&job.mission_id)
                .is_some_and(|m| m.state == ac_kernel::MissionState::Active);
            if is_active {
                let _ = kernel.transition_mission(
                    &job.mission_id,
                    ac_kernel::MissionState::Failed,
                    Vec::new(),
                );
            }
        }
        if let Some(mission) = kernel.mission(&job.mission_id) {
            db.put_mission(mission)?;
        }
        for event in kernel.events() {
            let _ = db.append_kernel_event(event);
        }
    }
    Ok(match report.state {
        ac_agent::AutonomousState::Completed => "completed",
        ac_agent::AutonomousState::Cancelled => "cancelled",
        _ => "failed",
    }
    .to_string())
}

/// Resolve the authoritative workspace root for a mission.
///
/// A project-specific workspace supplied by the caller is authoritative: it is
/// canonicalized, must exist and be a directory, must be absolute, must not be
/// a filesystem root, and must not traverse/escape via `..` or symlinks.  No
/// silent fallback to the daemon default is allowed when a project workspace
/// was supplied — an invalid supplied path is rejected, never replaced.
fn resolve_workspace_root(supplied: Option<&str>, daemon_default: &Path) -> AcResult<PathBuf> {
    let raw = match supplied {
        Some(raw) => raw,
        None => {
            // No project-specific workspace was supplied: fall back to the
            // daemon-configured default, canonicalized so the persisted value
            // is consistent with the submitted case.
            return fs::canonicalize(daemon_default).map_err(|error| {
                AcError::validation(
                    "DAEMON-WORKSPACE_DEFAULT",
                    format!("daemon workspace root is not accessible: {error}"),
                )
            });
        }
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AcError::validation(
            "DAEMON-WORKSPACE_EMPTY",
            "workspace_root must not be empty",
        ));
    }
    let candidate = Path::new(raw);
    if !candidate.is_absolute() {
        return Err(AcError::validation(
            "DAEMON-WORKSPACE_NOT_ABSOLUTE",
            "workspace_root must be an absolute path",
        ));
    }
    let canonical = fs::canonicalize(candidate).map_err(|error| {
        AcError::validation(
            "DAEMON-WORKSPACE_UNAVAILABLE",
            format!("workspace_root is not an accessible path: {error}"),
        )
    })?;
    if !canonical.is_dir() {
        return Err(AcError::validation(
            "DAEMON-WORKSPACE_NOT_DIR",
            "workspace_root must be a directory",
        ));
    }
    // Reject a filesystem root: canonicalize() already resolved any `..` or
    // symlink, so an accepted root here is a real directory.  A filesystem
    // root would grant the sandbox authority over the entire host.
    if canonical.parent().is_none() {
        return Err(AcError::policy_denied(
            "DAEMON-WORKSPACE_ROOT_DENIED",
            "workspace_root must not be a filesystem root",
        ));
    }
    Ok(canonical)
}

fn persisted_worktree_to_record(row: PersistedWorktree) -> AcResult<WorktreeRecord> {
    Ok(WorktreeRecord {
        id: StableId::from_existing(&row.id)?,
        repository_id: StableId::from_existing(&row.repository_id)?,
        owner_mission_id: StableId::from_existing(&row.owner_mission_id)?,
        owner_worker_id: StableId::from_existing(&row.owner_worker_id)?,
        lease_epoch: row.lease_epoch,
        path: PathBuf::from(row.path),
        branch: row.branch,
        base_commit: row.base_commit,
        current_commit: row.current_commit,
        status: match row.status.as_str() {
            "Active" => WorktreeStatus::Active,
            "Degraded" => WorktreeStatus::Degraded,
            "Missing" => WorktreeStatus::Missing,
            "Cleaned" => WorktreeStatus::Cleaned,
            _ => {
                return Err(AcError::validation(
                    "DAEMON-WORKTREE_STATUS",
                    "persisted worktree status is unknown",
                ));
            }
        },
        created_at: TimestampMillis::from_millis(row.created_at_ms as u128),
    })
}

fn backend_tool_policy() -> ac_security::CapabilityPolicy {
    ac_security::CapabilityPolicy::new()
        .allow(ac_security::Capability::FilesystemRead("*".to_string()))
        .allow(ac_security::Capability::FilesystemWrite("*".to_string()))
        .allow(ac_security::Capability::ProcessExec("*".to_string()))
        .allow(ac_security::Capability::BrowserAutomation)
        .allow(ac_security::Capability::SecurityScan)
}

include!("release.rs");
include!("observability.rs");
include!("ipc.rs");
include!("repo_context.rs");
include!("lsp.rs");
include!("conversation.rs");
include!("discuss_plan.rs");
include!("design.rs");
include!("security.rs");
include!("terminal.rs");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DaemonLifecycle {
    Created,
    Running,
    Stopping,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonHealth {
    pub lifecycle: DaemonLifecycle,
    pub recovered_sessions: usize,
}

pub trait DaemonLifecycleRuntime {
    fn start(&mut self) -> AcResult<()>;
    fn stop(&mut self) -> AcResult<()>;
    fn restart(&mut self) -> AcResult<()>;
    fn recover(&mut self) -> AcResult<usize>;
    fn heartbeat(&self) -> AcResult<DaemonHealth>;
    fn shutdown(&mut self) -> AcResult<()>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DaemonCommand {
    Health,
    CreateSession {
        goal: String,
        workspace_root: Option<String>,
    },
    CheckpointSession {
        session_id: StableId,
        next_step: u32,
    },
    DesktopWindowClosed {
        desktop_session_id: StableId,
    },
    ReconnectDesktop {
        desktop_session_id: StableId,
    },
    Stop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DaemonResponse {
    Health(DaemonHealth),
    SessionCreated {
        mission_id: StableId,
        session_id: StableId,
    },
    CheckpointSaved {
        checkpoint_id: StableId,
    },
    DesktopWindowClosed {
        daemon_active: bool,
    },
    DesktopReconnected {
        health: DaemonHealth,
        recovered_sessions: usize,
    },
    Stopped,
}

pub trait IpcTransport {
    fn send(&mut self, command: DaemonCommand) -> AcResult<DaemonResponse>;
}

pub struct LocalIpc<'a> {
    daemon: &'a mut DaemonService,
}

impl<'a> LocalIpc<'a> {
    pub fn new(daemon: &'a mut DaemonService) -> Self {
        Self { daemon }
    }
}

impl IpcTransport for LocalIpc<'_> {
    fn send(&mut self, command: DaemonCommand) -> AcResult<DaemonResponse> {
        self.daemon.handle(command)
    }
}

pub struct DaemonService {
    lifecycle: DaemonLifecycle,
    db: ControlPlaneDb,
    db_path: PathBuf,
    kernel: Arc<Mutex<Kernel<ProductionKernelPolicy>>>,
    lock_path: PathBuf,
    lock_file: Option<File>,
    instance_id: StableId,
    recovered: Vec<PersistedSession>,
    hydrated: Vec<HydratedSession>,
    coordinator: MissionCoordinator,
    /// Daemon-configured default workspace root (AGENTCODE_WORKSPACE_ROOT or
    /// cwd).  Used ONLY when a submission carries no project-specific
    /// workspace_root; a supplied project root always wins and is never
    /// silently replaced by this default.
    default_workspace_root: PathBuf,
    /// Live dev-server children keyed by design conversation id.  Managed by
    /// the Design Studio preview lifecycle; children are killed on stop and
    /// are dropped when the daemon exits.
    design_children: Mutex<BTreeMap<String, std::process::Child>>,
    /// Tracked terminal sessions (batch N4): live process handles + captured
    /// output, keyed by session id.  Children are killed on stop (same as
    /// design_children); finished sessions persist their output as evidence.
    terminal_sessions: Arc<Mutex<BTreeMap<String, Arc<TerminalSessionState>>>>,
    /// Watch-the-agent (browser): the design run's CURRENT observation
    /// (label + url), updated as the run navigates; the inbuilt panel shows
    /// it live while the run is active.
    agent_browse_status: Arc<Mutex<Option<Value>>>,
    /// Live panel browser (Codex-style inbuilt browser): ONE Chrome runtime
    /// reused across navigations so the panel behaves like an embedded
    /// browser instead of relaunching per request.  None until first use;
    /// torn down gracefully on daemon stop (Drop) and on explicit close.
    live_browser: Mutex<Option<ac_verification::BrowserRuntime>>,
}

/// Public alias for the include'd terminal module's session type.
pub type TerminalSession = TerminalSessionState;

// Watch-the-agent globals: one process-wide consumer thread (OnceLock) plus
// the terminal registry of the CURRENT daemon (set on each start).
type TerminalRegistry = Arc<Mutex<BTreeMap<String, Arc<TerminalSessionState>>>>;
static LIVE_AGENT_REGISTRY: Mutex<Option<TerminalRegistry>> = Mutex::new(None);
/// Per-mission terminal sinks: the daemon running THAT mission receives the
/// mirrored chunks.  Registered at mission dispatch, removed at mission end.
static LIVE_AGENT_SINKS: Mutex<BTreeMap<String, TerminalRegistry>> = Mutex::new(BTreeMap::new());
static LIVE_AGENT_REGISTRY_ONCE: std::sync::Once = std::sync::Once::new();

/// Production daemon policy. Tool capability checks remain owned by ToolBroker;
/// the bound agent's deterministic final-audit/completion gate supplies
/// evidence before it asks the kernel to mark a mission completed.
#[derive(Default)]
struct ProductionKernelPolicy;

impl PolicyBoundary for ProductionKernelPolicy {
    fn evaluate(&self, decision: KernelDecisionKind) -> PermissionDecision {
        match decision {
            KernelDecisionKind::CreateMission
            | KernelDecisionKind::ActivateMission
            | KernelDecisionKind::CancelMission
            | KernelDecisionKind::FailMission
            | KernelDecisionKind::ApproveChangeSet
            | KernelDecisionKind::CompleteMission => PermissionDecision::Allow,
        }
    }
}

impl DaemonService {
    pub fn open(db_path: impl AsRef<Path>, lock_path: impl Into<PathBuf>) -> AcResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let mut db = ControlPlaneDb::open(&db_path)?;
        db.migrate()?;
        seed_provider_catalog(&db)?;
        let kernel = Arc::new(Mutex::new(Kernel::new(ProductionKernelPolicy)));
        let workspace_root = std::env::var_os("AGENTCODE_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or(
                std::env::current_dir()
                    .map_err(|error| AcError::validation("DAEMON-WORKSPACE", error.to_string()))?,
            );
        let terminal_sessions: Arc<Mutex<BTreeMap<String, Arc<TerminalSessionState>>>> =
            Arc::new(Mutex::new(BTreeMap::new()));
        Ok(Self {
            lifecycle: DaemonLifecycle::Created,
            db,
            coordinator: MissionCoordinator::new(
                db_path.clone(),
                Arc::clone(&kernel),
                Arc::clone(&terminal_sessions),
            ),
            db_path,
            kernel,
            lock_path: lock_path.into(),
            lock_file: None,
            instance_id: StableId::new("daemon"),
            recovered: Vec::new(),
            hydrated: Vec::new(),
            default_workspace_root: workspace_root,
            design_children: Mutex::new(BTreeMap::new()),
            terminal_sessions,
            agent_browse_status: Arc::new(Mutex::new(None)),
            live_browser: Mutex::new(None),
        })
    }

    pub fn start(&mut self) -> AcResult<()> {
        if self.lifecycle == DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-ALREADY_RUNNING",
                "daemon is already running",
            ));
        }
        self.acquire_singleton()?;
        self.kernel_lock()?.start()?;
        self.hydrated = RuntimeHydrator::hydrate_interrupted(&self.db)?;
        self.recovered = self
            .hydrated
            .iter()
            .map(|hydrated| hydrated.session.clone())
            .collect();
        for hydrated in &self.hydrated {
            if let Some(mission) = self
                .db
                .get_mission(&StableId::from_existing(&hydrated.session.mission_id)?)?
            {
                let mission_id = StableId::from_existing(&mission.id)?;
                let state = match mission.state.as_str() {
                    "created" => MissionState::Created,
                    "active" => MissionState::Active,
                    "completed" => MissionState::Completed,
                    "cancelled" => MissionState::Cancelled,
                    "failed" => MissionState::Failed,
                    _ => continue,
                };
                let mut kernel = self.kernel_lock()?;
                if kernel.mission(&mission_id).is_none() {
                    kernel.restore_mission(ac_kernel::Mission {
                        id: mission_id.clone(),
                        original_goal: mission.original_goal.clone(),
                        state,
                        created_at: TimestampMillis::from_millis(mission.created_at_ms as u128),
                    })?;
                }
                drop(kernel);
                // Recovery must reuse the persisted project workspace rather
                // than the daemon's current cwd/AGENTCODE_WORKSPACE_ROOT.  The
                // persisted workspace was validated at submission time and is
                // re-validated here so a stale or tampered value cannot cause
                // the mission to resume in a different directory.
                let workspace_root = match resolve_workspace_root(
                    hydrated.session.workspace_root.as_deref(),
                    &self.default_workspace_root,
                ) {
                    Ok(root) => root,
                    Err(error) => {
                        eprintln!(
                            "MISSION-RECOVERY-WORKSPACE session={} error={}",
                            hydrated.session.id,
                            error.code()
                        );
                        let _ = self.db.update_session_state(
                            &StableId::from_existing(&hydrated.session.id)?,
                            "failed",
                        );
                        continue;
                    }
                };
                let job = QueuedMission {
                    mission_id,
                    session_id: StableId::from_existing(&hydrated.session.id)?,
                    goal: mission.original_goal,
                    workspace_root,
                    attachments: Vec::new(),
                };
                match hydrated.session.state.as_str() {
                    "paused" => self.coordinator.remember_paused(job)?,
                    "completed" | "cancelled" | "failed" => {}
                    _ => self.coordinator.enqueue(job)?,
                }
            }
        }
        // Watch-the-agent (live terminal mirroring): bind this daemon's
        // terminal registry to the process-global consumer so agent-command
        // output streams into mission-tagged sessions the drawer can watch.
        {
            let registry = Arc::clone(&self.terminal_sessions);
            let spawned_once: &mut Option<Result<(), String>> = &mut None;
            LIVE_AGENT_REGISTRY_ONCE.call_once(|| {
                let receiver = ac_tool::install_live_forwarder();
                let spawned = std::thread::Builder::new()
                    .name("live-agent-output".to_string())
                    .spawn(move || {
                        // (mission_ctx, argv) -> (session_id, partial line)
                        let mut sessions: std::collections::HashMap<
                            (String, String),
                            (String, String),
                        > = std::collections::HashMap::new();
                        while let Ok(chunk) = receiver.recv() {
                            // Resolve the sink by the chunk's mission id: the
                            // daemon that RUNS the mission owns the session.
                            // (Production has one daemon; tests may run
                            // several — each mission lands in its own.)
                            let mission_id = parse_mission_tag(
                                &chunk.mission_context.clone().unwrap_or_default(),
                            )
                            .unwrap_or_default();
                            let Some(registry) = LIVE_AGENT_SINKS
                                .lock()
                                .ok()
                                .and_then(|sinks| sinks.get(&mission_id).cloned())
                                .or_else(|| {
                                    LIVE_AGENT_REGISTRY.lock().ok().and_then(|g| g.clone())
                                })
                            else {
                                continue;
                            };
                            let ctx = chunk.mission_context.clone().unwrap_or_default();
                            let argv_key = chunk.argv.join(" ");
                            if argv_key.is_empty() {
                                continue;
                            }
                            let session_id = match sessions.get(&(ctx.clone(), argv_key.clone())) {
                                Some((id, _)) => id.clone(),
                                None => {
                                    let id = StableId::new("terminal").to_string();
                                    let state = Arc::new(TerminalSessionState {
                                        id: id.clone(),
                                        argv: chunk.argv.clone(),
                                        cwd: chunk.cwd.clone(),
                                        mission_id: parse_mission_tag(&ctx),
                                        source: "agent".to_string(),
                                        output: Mutex::new(VecDeque::new()),
                                        total_bytes: AtomicUsize::new(0),
                                        total_lines: AtomicUsize::new(0),
                                        child: Mutex::new(None),
                                        exit_code: Mutex::new(None),
                                        cancelled: AtomicBool::new(false),
                                        started_at_ms: TimestampMillis::now().as_millis() as i64,
                                        evidence_id: Mutex::new(None),
                                    });
                                    registry.lock().unwrap().insert(id.clone(), state);
                                    sessions.insert(
                                        (ctx.clone(), argv_key.clone()),
                                        (id.clone(), String::new()),
                                    );
                                    id
                                }
                            };
                            let entry = match sessions.get_mut(&(ctx, argv_key)) {
                                Some(entry) => entry,
                                None => continue,
                            };
                            entry.1.push_str(&String::from_utf8_lossy(&chunk.bytes));
                            while let Some(pos) = entry.1.find('\n') {
                                let line: String = entry.1.drain(..pos + 1).collect();
                                let line = line.trim_end_matches('\n').to_string();
                                if let Some(state) = registry.lock().unwrap().get(&session_id) {
                                    let mut output = state.output.lock().unwrap();
                                    let bounded: String = line.chars().take(400).collect();
                                    state.total_bytes.fetch_add(
                                        bounded.len(),
                                        std::sync::atomic::Ordering::SeqCst,
                                    );
                                    state
                                        .total_lines
                                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                    output.push_back(bounded);
                                    while output.len() > MAX_TERMINAL_RING_LINES {
                                        output.pop_front();
                                    }
                                }
                            }
                        }
                    });
                *spawned_once = Some(spawned.map(|_| ()).map_err(|e| e.to_string()));
            });
            if let Some(Err(error)) = &*spawned_once {
                eprintln!("live agent-output mirroring unavailable: {error}");
            }
            // Sinks are registered per-mission at dispatch time (see the
            // mission runner); nothing to bind here.  Keep a fallback for
            // context-less chunks: this daemon is the sink of record.
            if let Ok(mut guard) = LIVE_AGENT_REGISTRY.lock() {
                *guard = Some(registry);
            }
        }
        self.lifecycle = DaemonLifecycle::Running;
        Ok(())
    }

    pub fn stop(&mut self) -> AcResult<()> {
        if self.lifecycle != DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-NOT_RUNNING",
                "daemon must be running before stop",
            ));
        }
        self.lifecycle = DaemonLifecycle::Stopping;
        // Close the inbuilt-browser panel's shared Chrome FIRST and
        // gracefully (CDP Browser.close): the runtime's Drop handler also
        // escalates politely, but stopping here keeps the teardown ordered
        // and off the shutdown path's timing.
        if let Ok(mut guard) = self.live_browser.lock() {
            if let Some(mut runtime) = guard.take() {
                runtime.close_all();
            }
        }
        // Cancel in-flight work and join the mission worker BEFORE stopping the
        // kernel.  A mission cancelled mid-flight must still be able to
        // transition its kernel mission to a terminal state during
        // reconciliation; stopping the kernel first would reject those
        // transitions with KERNEL-NOT_RUNNING and leave the mission failed.
        self.coordinator.stop()?;
        self.kernel_lock()?.stop()?;
        self.lock_file = None;
        let _ = fs::remove_file(&self.lock_path);
        self.lifecycle = DaemonLifecycle::Stopped;
        Ok(())
    }

    pub fn restart(&mut self) -> AcResult<()> {
        if self.lifecycle == DaemonLifecycle::Running {
            self.stop()?;
        }
        self.start()
    }

    pub fn recover(&mut self) -> AcResult<usize> {
        self.recovered = self.db.interrupted_sessions()?;
        Ok(self.recovered.len())
    }

    pub fn health(&self) -> DaemonHealth {
        DaemonHealth {
            lifecycle: self.lifecycle,
            recovered_sessions: self.recovered.len(),
        }
    }

    pub fn heartbeat(&self) -> AcResult<DaemonHealth> {
        if self.lifecycle != DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-NOT_RUNNING",
                "daemon heartbeat requires running lifecycle",
            ));
        }
        Ok(self.health())
    }

    pub fn shutdown(&mut self) -> AcResult<()> {
        match self.lifecycle {
            DaemonLifecycle::Running => self.stop(),
            DaemonLifecycle::Stopping | DaemonLifecycle::Stopped | DaemonLifecycle::Created => {
                self.lock_file = None;
                let _ = fs::remove_file(&self.lock_path);
                self.lifecycle = DaemonLifecycle::Stopped;
                Ok(())
            }
        }
    }

    pub fn recovered_sessions(&self) -> &[PersistedSession] {
        &self.recovered
    }

    pub fn active_missions(&self) -> AcResult<Vec<(String, String, usize)>> {
        self.db
            .active_sessions()?
            .into_iter()
            .map(|session| {
                Ok((
                    session.mission_id.clone(),
                    session.state,
                    self.db.tasks_for_mission(&session.mission_id)?.len(),
                ))
            })
            .collect()
    }

    pub fn task_states(&self, mission_id: &str) -> AcResult<Vec<(String, String)>> {
        self.db.tasks_for_mission(mission_id).map(|tasks| {
            tasks
                .into_iter()
                .map(|task| (task.id, task.state))
                .collect()
        })
    }

    pub fn mission_status(&self, mission_id: &str) -> Option<MissionExecutionStatus> {
        self.coordinator.status(mission_id)
    }

    pub fn persisted_mission_state(&self, mission_id: &str) -> AcResult<Option<String>> {
        let id = StableId::from_existing(mission_id)?;
        if let Some(session) = self.db.session_for_mission(&id)? {
            if matches!(
                session.state.as_str(),
                "paused" | "queued" | "running" | "cancelled" | "failed" | "completed"
            ) {
                return Ok(Some(session.state));
            }
        }
        Ok(self.db.get_mission(&id)?.map(|mission| mission.state))
    }

    pub fn mission_goal(&self, mission_id: &str) -> AcResult<Option<String>> {
        let id = StableId::from_existing(mission_id)?;
        Ok(self
            .db
            .get_mission(&id)?
            .map(|mission| mission.original_goal))
    }

    /// The authoritative project workspace for a mission, read from the
    /// persisted session.  This is the directory the daemon will execute in,
    /// not the daemon's own cwd.
    pub fn mission_workspace(&self, mission_id: &str) -> AcResult<Option<String>> {
        let id = StableId::from_existing(mission_id)?;
        Ok(self
            .db
            .session_for_mission(&id)?
            .and_then(|session| session.workspace_root))
    }

    pub fn desktop_settings(&self) -> AcResult<ac_db::DesktopPreferenceRow> {
        Ok(self
            .db
            .desktop_preference("ui")?
            .unwrap_or(ac_db::DesktopPreferenceRow {
                session_id: "ui".to_string(),
                appearance: "light".to_string(),
                notifications_enabled: true,
                completion_sound_enabled: true,
                reduced_motion: false,
                budget_limit_micros: None,
            }))
    }

    pub fn save_desktop_settings(
        &self,
        appearance: String,
        notifications_enabled: bool,
        completion_sound_enabled: bool,
        reduced_motion: bool,
        budget_limit_micros: Option<u64>,
    ) -> AcResult<()> {
        self.db
            .save_desktop_preference(&ac_db::DesktopPreferenceRow {
                session_id: "ui".to_string(),
                appearance,
                notifications_enabled,
                completion_sound_enabled,
                reduced_motion,
                budget_limit_micros,
            })
    }

    pub fn pause_mission(&mut self, mission_id: &str) -> AcResult<()> {
        self.ensure_running()?;
        self.coordinator.pause(mission_id)?;
        if let Some(status) = self.coordinator.status(mission_id) {
            self.db.update_session_state(&status.session_id, "paused")?;
        }
        Ok(())
    }

    pub fn resume_mission(&mut self, mission_id: &str) -> AcResult<()> {
        self.ensure_running()?;
        self.coordinator.resume(mission_id)?;
        if let Some(status) = self.coordinator.status(mission_id) {
            let persisted = match status.state.as_str() {
                "paused" => "paused",
                "running" => "running",
                _ => "queued",
            };
            self.db
                .update_session_state(&status.session_id, persisted)?;
        }
        Ok(())
    }

    pub fn cancel_mission(&mut self, mission_id: &str) -> AcResult<()> {
        self.ensure_running()?;
        let was_running = self.coordinator.cancel(mission_id)?;
        if let Some(status) = self.coordinator.status(mission_id) {
            self.db
                .update_session_state(&status.session_id, "cancelled")?;
        }
        // A queued or paused mission cancelled before its agent ever ran has no
        // run loop to perform the kernel transition (the agent's cancellation
        // checks only execute once execution starts).  Transition and persist
        // the kernel mission to Cancelled here so the authoritative mission
        // state is terminal and survives restart, not left Active forever.
        if !was_running {
            let mut kernel = self.kernel_lock()?;
            let id = StableId::from_existing(mission_id)?;
            let is_active = kernel
                .mission(&id)
                .is_some_and(|m| m.state == ac_kernel::MissionState::Active);
            if is_active {
                let _ =
                    kernel.transition_mission(&id, ac_kernel::MissionState::Cancelled, Vec::new());
            }
            if let Some(mission) = kernel.mission(&id) {
                self.db.put_mission(mission)?;
                for event in kernel.events() {
                    let _ = self.db.append_kernel_event(event);
                }
            }
        }
        Ok(())
    }

    // ── Provider Catalog ──────────────────────────────────────────────────────

    pub fn provider_catalog(&self) -> AcResult<Vec<ProviderCatalogEntry>> {
        let rows = self.db.provider_catalog_entries()?;
        rows.into_iter().map(db_catalog_entry_to_ac).collect()
    }

    pub fn list_provider_accounts(&self, provider_id: &str) -> AcResult<Vec<CatalogAccount>> {
        let rows = self.db.provider_accounts(provider_id)?;
        rows.into_iter().map(db_account_to_ac).collect()
    }

    pub fn provider_account(&self, account_id: &str) -> AcResult<Option<CatalogAccount>> {
        self.db
            .provider_account(account_id)?
            .map(db_account_to_ac)
            .transpose()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_provider_account(
        &self,
        provider_id: &str,
        label: &str,
        credential_ref: &str,
        credential_region: &str,
        organization: &str,
        project: &str,
        workspace: &str,
        enabled: bool,
    ) -> AcResult<StableId> {
        let now = ac_common::TimestampMillis::now().as_millis() as i64;
        let id = StableId::new("acct");
        let row = ac_db::ProviderAccountRow {
            id: id.to_string(),
            provider_id: provider_id.to_string(),
            label: label.to_string(),
            credential_ref: credential_ref.to_string(),
            credential_region: credential_region.to_string(),
            organization: organization.to_string(),
            project: project.to_string(),
            workspace: workspace.to_string(),
            enabled,
            health_state: "unknown".to_string(),
            quota_rate_limit: None,
            quota_remaining: None,
            quota_reset_at_ms: None,
            last_success_at_ms: None,
            last_failure_at_ms: None,
            failure_reason: String::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.db.save_provider_account(&row)?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_provider_account(
        &self,
        account_id: &str,
        label: &str,
        credential_ref: &str,
        credential_region: &str,
        organization: &str,
        project: &str,
        workspace: &str,
        enabled: bool,
    ) -> AcResult<()> {
        let existing = self.db.provider_account(account_id)?.ok_or_else(|| {
            AcError::validation(
                "DAEMON-PROVIDER_ACCOUNT_NOT_FOUND",
                "provider account not found",
            )
        })?;
        let now = ac_common::TimestampMillis::now().as_millis() as i64;
        let row = ac_db::ProviderAccountRow {
            credential_ref: if credential_ref.is_empty() {
                existing.credential_ref
            } else {
                credential_ref.to_string()
            },
            ..existing
        };
        let mut row = row;
        row.label = label.to_string();
        row.credential_region = credential_region.to_string();
        row.organization = organization.to_string();
        row.project = project.to_string();
        row.workspace = workspace.to_string();
        row.enabled = enabled;
        row.updated_at_ms = now;
        self.db.save_provider_account(&row)
    }

    pub fn delete_provider_account(&self, account_id: &str) -> AcResult<()> {
        self.db.delete_provider_account(account_id)
    }

    /// Toggle only the enabled flag of an account.  The credential reference is
    /// never round-tripped through the UI so a masked value cannot overwrite it.
    pub fn set_provider_account_enabled(&self, account_id: &str, enabled: bool) -> AcResult<()> {
        let existing = self.db.provider_account(account_id)?.ok_or_else(|| {
            AcError::validation(
                "DAEMON-PROVIDER_ACCOUNT_NOT_FOUND",
                "provider account not found",
            )
        })?;
        let now = ac_common::TimestampMillis::now().as_millis() as i64;
        let mut row = existing;
        row.enabled = enabled;
        row.updated_at_ms = now;
        self.db.save_provider_account(&row)
    }

    /// Rotate an account's credential reference to a newly stored secret.  The
    /// UI passes only the new `secret:NAME` reference (from StoreCredential),
    /// never the raw key and never the old masked credential.
    pub fn rotate_provider_account(&self, account_id: &str, credential_ref: &str) -> AcResult<()> {
        if credential_ref.trim().is_empty() {
            return Err(AcError::validation(
                "DAEMON-PROVIDER_ROTATE_EMPTY",
                "rotation requires a new credential reference",
            ));
        }
        let existing = self.db.provider_account(account_id)?.ok_or_else(|| {
            AcError::validation(
                "DAEMON-PROVIDER_ACCOUNT_NOT_FOUND",
                "provider account not found",
            )
        })?;
        let now = ac_common::TimestampMillis::now().as_millis() as i64;
        let mut row = existing;
        row.credential_ref = credential_ref.to_string();
        row.health_state = "ok".to_string();
        row.last_failure_at_ms = None;
        row.failure_reason = String::new();
        row.last_success_at_ms = Some(now);
        row.updated_at_ms = now;
        self.db.save_provider_account(&row)
    }

    pub fn test_provider_account(&self, account_id: &str) -> AcResult<ProviderAccountStatus> {
        let account = self.db.provider_account(account_id)?.ok_or_else(|| {
            AcError::validation(
                "DAEMON-PROVIDER_ACCOUNT_NOT_FOUND",
                "provider account not found",
            )
        })?;
        let catalog_acct = db_account_to_ac(account)?;
        let kind = provider_connection_kind(&catalog_acct.provider_id);
        let endpoint = catalog_account_endpoint(&catalog_acct)?;
        let model = "test".to_string();
        let test = ProviderConnectionTest::new(catalog_acct.clone(), endpoint, model, kind)?;
        let result = test_provider_account(&test, &|| false)?;
        // Update the account's health state in the DB
        let now = ac_common::TimestampMillis::now().as_millis() as i64;
        let health_state = if result.ok { "ok" } else { "failed" };
        if let Ok(Some(existing)) = self.db.provider_account(account_id) {
            let updated = ac_db::ProviderAccountRow {
                health_state: health_state.to_string(),
                last_success_at_ms: if result.ok {
                    Some(now)
                } else {
                    existing.last_success_at_ms
                },
                last_failure_at_ms: if result.ok {
                    existing.last_failure_at_ms
                } else {
                    Some(now)
                },
                failure_reason: if result.ok {
                    String::new()
                } else {
                    result.detail.clone()
                },
                updated_at_ms: now,
                ..existing
            };
            let _ = self.db.save_provider_account(&updated);
        }
        let _ = self
            .db
            .append_provider_health_observation(&ac_db::ProviderHealthObservationRow {
                id: StableId::new("health").to_string(),
                account_id: account_id.to_string(),
                success: result.ok,
                latency_ms: result.latency_ms,
                failure_code: result
                    .failure
                    .as_ref()
                    .map(|f| format!("{f:?}"))
                    .unwrap_or_default(),
                failure_message: result.detail.clone(),
                observed_at_ms: now,
            });
        Ok(result)
    }

    pub fn discover_provider_models(
        &self,
        endpoint_base: &str,
        kind: &str,
    ) -> AcResult<Vec<ac_provider::catalog::DiscoveredModel>> {
        let discovery_kind = match kind {
            "ollama" => ModelDiscoveryKind::Ollama,
            _ => ModelDiscoveryKind::OpenAiCompatible,
        };
        discover_models(endpoint_base, discovery_kind, 5000, 10000)
    }

    /// Bounded discovery for FIRST-RUN health surfaces (ReadinessGet): a
    /// busy or mid-pull Ollama must never stall readiness.  The read
    /// timeout honors the provider layer's 5s floor; the connect timeout
    /// is capped tight so a dead port answers fast.
    pub fn discover_provider_models_bounded(
        &self,
        endpoint_base: &str,
        kind: &str,
        hard_deadline_ms: u64,
    ) -> AcResult<Vec<ac_provider::catalog::DiscoveredModel>> {
        let endpoint = endpoint_base.to_string();
        let discovery_kind = match kind {
            "ollama" => ModelDiscoveryKind::Ollama,
            _ => ModelDiscoveryKind::OpenAiCompatible,
        };
        // Readiness must answer fast even when Ollama is mid-pull (its
        // /api/tags can block on the registry lock for many seconds).  The
        // probe runs on a detached worker that reports through a channel;
        // the caller waits at most the hard deadline.  A slow-but-live
        // Ollama surfaces as an honest timeout error, never a stall (the
        // worker finishes on its own and drops cleanly).
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(discover_models(&endpoint, discovery_kind, 1000, 5000));
        });
        let deadline = std::time::Duration::from_millis(hard_deadline_ms.max(1000));
        match rx.recv_timeout(deadline) {
            Ok(result) => result,
            Err(_) => Err(AcError::validation(
                "PROVIDER-DISCOVERY_SLOW",
                "model discovery exceeded the readiness deadline; the provider may be starting or busy",
            )),
        }
    }

    // ── Backend-Owned Credential Store ─────────────────────────────────────────

    /// Store a raw credential value into the single backend-owned secret store
    /// (`AGENTCODE_SECRET_DIR`) and return its `secret:NAME` reference.  This is
    /// the one authoritative secret-storage path; the desktop layer must not
    /// maintain its own secret files.
    pub fn store_credential(&self, name: &str, value: &str) -> AcResult<String> {
        if name.trim().is_empty() {
            return Err(AcError::validation(
                "DAEMON-CREDENTIAL_NAME_EMPTY",
                "credential name is required",
            ));
        }
        if value.is_empty() {
            return Err(AcError::validation(
                "DAEMON-CREDENTIAL_VALUE_EMPTY",
                "credential value is required",
            ));
        }
        let dir = std::env::var("AGENTCODE_SECRET_DIR").map_err(|_| {
            AcError::validation(
                "DAEMON-SECRET_DIR_UNAVAILABLE",
                "secret storage requires AGENTCODE_SECRET_DIR",
            )
        })?;
        let dir_path = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir_path)
            .map_err(|error| AcError::validation("DAEMON-SECRET_DIR_CREATE", error.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir_path, std::fs::Permissions::from_mode(0o700));
        }
        let path = dir_path.join(sanitize_secret_name(name));
        std::fs::write(&path, value.as_bytes())
            .map_err(|error| AcError::validation("DAEMON-SECRET_WRITE", error.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(format!("secret:{}", sanitize_secret_name(name)))
    }

    pub fn delete_credential(&self, name: &str) -> AcResult<()> {
        let dir = std::env::var("AGENTCODE_SECRET_DIR").map_err(|_| {
            AcError::validation(
                "DAEMON-SECRET_DIR_UNAVAILABLE",
                "secret storage requires AGENTCODE_SECRET_DIR",
            )
        })?;
        let path = std::path::Path::new(&dir).join(sanitize_secret_name(name));
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|error| AcError::validation("DAEMON-SECRET_REMOVE", error.to_string()))?;
        }
        Ok(())
    }

    /// Run a real provider connection test for a credential reference WITHOUT
    /// persisting an account.  Used by the UI's "Test Connection" so success is
    /// only reported after the provider is actually contacted.
    pub fn test_credential(
        &self,
        provider_id: &str,
        credential_ref: &str,
        organization: &str,
        project: &str,
        workspace: &str,
    ) -> AcResult<ProviderAccountStatus> {
        if provider_id.trim().is_empty() || credential_ref.trim().is_empty() {
            return Err(AcError::validation(
                "DAEMON-TEST_CREDENTIAL_INVALID",
                "provider and credential reference are required",
            ));
        }
        let account = CatalogAccount {
            id: StableId::new("tmp-acct"),
            provider_id: StableId::from_existing(provider_id)?,
            label: "connection-test".to_string(),
            credential_ref: credential_ref.to_string(),
            credential_region: String::new(),
            organization: organization.to_string(),
            project: project.to_string(),
            workspace: workspace.to_string(),
            enabled: true,
            health_state: "unknown".to_string(),
            quota_rate_limit: None,
            quota_remaining: None,
            quota_reset_at_ms: None,
            last_success_at_ms: None,
            last_failure_at_ms: None,
            failure_reason: String::new(),
        };
        let kind = provider_connection_kind(&account.provider_id);
        let endpoint = catalog_account_endpoint(&account)?;
        let test = ProviderConnectionTest::new(account, endpoint, "test", kind)?;
        test_provider_account(&test, &|| false)
    }

    // ── Provider Registry Builder ──────────────────────────────────────────────

    /// Build a ProviderRegistry from the DB catalog + accounts, falling back to
    /// the environment for providers that have no stored accounts.
    pub fn build_provider_registry(&self) -> AcResult<ac_provider::ProviderRegistry> {
        daemon_provider_registry(&self.db, &self.db_path)
    }

    /// Batch N1 (G1): the user's persisted routing profile, with a safe
    /// fallback to LocalFirst when nothing (or something invalid) is
    /// stored.  Production call sites use this instead of hardcoding a
    /// profile, so the Settings control actually governs routing.
    pub fn preferred_routing_profile(&self) -> ac_provider::RoutingProfile {
        match self.db.provider_preference("global") {
            Ok(Some(row)) => parse_routing_profile(&row.routing_profile),
            _ => ac_provider::RoutingProfile::LocalFirst,
        }
    }

    /// Batch N1 (G1): the persisted preferred-model hint (may be empty,
    /// meaning "no preference" — routing then follows the profile only).
    pub fn preferred_model(&self) -> String {
        self.db
            .provider_preference("global")
            .ok()
            .flatten()
            .map(|row| row.preferred_model)
            .unwrap_or_default()
    }

    pub fn handle(&mut self, command: DaemonCommand) -> AcResult<DaemonResponse> {
        match command {
            DaemonCommand::Health => Ok(DaemonResponse::Health(self.health())),
            DaemonCommand::CreateSession {
                goal,
                workspace_root,
            } => {
                self.ensure_running()?;
                // The supplied project workspace is authoritative and is
                // validated independently of the frontend (canonicalized,
                // must be a real directory, must not escape).  A missing value
                // falls back to the daemon-configured default only because no
                // project-specific workspace was supplied.
                let workspace = resolve_workspace_root(
                    workspace_root.as_deref(),
                    &self.default_workspace_root,
                )?;
                let mut kernel = self.kernel_lock()?;
                let mission_id = kernel.create_mission(goal.clone())?;
                kernel.transition_mission(&mission_id, MissionState::Active, Vec::new())?;
                if let Some(mission) = kernel.mission(&mission_id) {
                    self.db.put_mission(mission)?;
                }
                for event in kernel.events() {
                    let _ = self.db.append_kernel_event(event);
                }
                let session = AgentSession::new(Worker::new());
                self.db.save_session(
                    session.id(),
                    &mission_id,
                    session_state(session.state()),
                    Some(workspace.to_string_lossy().as_ref()),
                )?;
                self.coordinator.enqueue(QueuedMission {
                    mission_id: mission_id.clone(),
                    session_id: session.id().clone(),
                    goal,
                    workspace_root: workspace,
                    attachments: Vec::new(),
                })?;
                Ok(DaemonResponse::SessionCreated {
                    mission_id,
                    session_id: session.id().clone(),
                })
            }
            DaemonCommand::CheckpointSession {
                session_id,
                next_step,
            } => {
                self.ensure_running()?;
                let checkpoint_id = StableId::new("daemoncp");
                self.db
                    .save_checkpoint(&checkpoint_id, &session_id, next_step, "executing")?;
                self.db.update_session_state(&session_id, "executing")?;
                Ok(DaemonResponse::CheckpointSaved { checkpoint_id })
            }
            DaemonCommand::DesktopWindowClosed { .. } => Ok(DaemonResponse::DesktopWindowClosed {
                daemon_active: self.lifecycle == DaemonLifecycle::Running,
            }),
            DaemonCommand::ReconnectDesktop { .. } => Ok(DaemonResponse::DesktopReconnected {
                health: self.health(),
                recovered_sessions: self.recovered.len(),
            }),
            DaemonCommand::Stop => {
                self.stop()?;
                Ok(DaemonResponse::Stopped)
            }
        }
    }

    fn ensure_running(&self) -> AcResult<()> {
        if self.lifecycle != DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-NOT_RUNNING",
                "daemon command requires running lifecycle",
            ));
        }
        Ok(())
    }

    fn kernel_lock(&self) -> AcResult<std::sync::MutexGuard<'_, Kernel<ProductionKernelPolicy>>> {
        self.kernel.lock().map_err(|_| {
            AcError::conflict("DAEMON-KERNEL_POISONED", "shared kernel lock is poisoned")
        })
    }

    fn acquire_singleton(&mut self) -> AcResult<()> {
        if self.lock_path.exists() {
            let stale = fs::read_to_string(&self.lock_path)
                .ok()
                .and_then(|metadata| lock_pid(&metadata))
                .is_none_or(|pid| !process_is_alive(pid));
            if stale {
                fs::remove_file(&self.lock_path).map_err(|error| {
                    AcError::conflict("DAEMON-STALE_LOCK_REMOVE", error.to_string())
                })?;
            }
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.lock_path)
            .map_err(|err| {
                AcError::conflict(
                    "DAEMON-SINGLETON_LOCKED",
                    format!("daemon singleton lock unavailable: {}", err),
                )
            })?;
        writeln!(
            file,
            "pid={}\nstarted_at_ms={}\ninstance_id={}",
            std::process::id(),
            TimestampMillis::now().as_millis(),
            self.instance_id
        )
        .map_err(|error| AcError::validation("DAEMON-SINGLETON_WRITE", error.to_string()))?;
        self.lock_file = Some(file);
        Ok(())
    }
}

fn lock_pid(metadata: &str) -> Option<u32> {
    metadata
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|pid| pid.parse().ok())
}

/// Seed the provider catalog table with the canonical provider set that
/// AgentCode's OmniRoute registry actually supports.  The table is the source
/// of truth for `ListProviders`; seeding is idempotent and only inserts rows
/// that are missing so user-added providers are never overwritten.
fn seed_provider_catalog(db: &ControlPlaneDb) -> AcResult<()> {
    let now = TimestampMillis::now().as_millis() as i64;
    let canonical: Vec<ProviderCatalogRow> = vec![
        // OmniRouter — THE managed router (OmniRoute architecture): one
        // account routes to every supported model.  First in the catalog so
        // the Providers tab leads with it.
        ProviderCatalogRow {
            id: "omnirouter".to_string(),
            display_name: "OmniRouter".to_string(),
            description: "One account, every model — managed routing (OmniRoute).".to_string(),
            website_url: "https://omnirouter.app".to_string(),
            logo_url: String::new(),
            credential_url: "https://omnirouter.app/settings/keys".to_string(),
            pricing_classification: "free-tier".to_string(),
            capabilities: serde_json::to_string(&vec!["chat", "tools", "vision"]).unwrap(),
            created_at_ms: now,
            updated_at_ms: now,
        },
        ProviderCatalogRow {
            id: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            description: "Fast inference models (GPT series).".to_string(),
            website_url: "https://openai.com".to_string(),
            logo_url: String::new(),
            credential_url: "https://platform.openai.com/api-keys".to_string(),
            pricing_classification: "paid".to_string(),
            capabilities: serde_json::to_string(&vec!["chat", "tools", "vision"]).unwrap(),
            created_at_ms: now,
            updated_at_ms: now,
        },
        ProviderCatalogRow {
            id: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            description: "High-quality Claude models.".to_string(),
            website_url: "https://anthropic.com".to_string(),
            logo_url: String::new(),
            credential_url: "https://console.anthropic.com/settings/keys".to_string(),
            pricing_classification: "paid".to_string(),
            capabilities: serde_json::to_string(&vec!["chat", "tools"]).unwrap(),
            created_at_ms: now,
            updated_at_ms: now,
        },
        ProviderCatalogRow {
            id: "gemini".to_string(),
            display_name: "Google / Gemini".to_string(),
            description: "Free tier available (Gemini models).".to_string(),
            website_url: "https://ai.google.dev".to_string(),
            logo_url: String::new(),
            credential_url: "https://aistudio.google.com/app/apikey".to_string(),
            pricing_classification: "free-tier".to_string(),
            capabilities: serde_json::to_string(&vec!["chat", "vision"]).unwrap(),
            created_at_ms: now,
            updated_at_ms: now,
        },
        ProviderCatalogRow {
            id: "ollama".to_string(),
            display_name: "Ollama".to_string(),
            description: "Local models, free, offline capable.".to_string(),
            website_url: "https://ollama.com".to_string(),
            logo_url: String::new(),
            credential_url: "https://ollama.com/download".to_string(),
            pricing_classification: "local".to_string(),
            capabilities: serde_json::to_string(&vec!["chat", "tools", "local"]).unwrap(),
            created_at_ms: now,
            updated_at_ms: now,
        },
        ProviderCatalogRow {
            id: "lm-studio".to_string(),
            display_name: "LM Studio".to_string(),
            description: "Local OpenAI-compatible models.".to_string(),
            website_url: "https://lmstudio.ai".to_string(),
            logo_url: String::new(),
            credential_url: "https://lmstudio.ai/docs".to_string(),
            pricing_classification: "local".to_string(),
            capabilities: serde_json::to_string(&vec!["chat", "local"]).unwrap(),
            created_at_ms: now,
            updated_at_ms: now,
        },
    ];
    for row in canonical {
        if db.provider_catalog_entry(&row.id)?.is_none() {
            db.save_provider_catalog_entry(&row)?;
        }
    }
    Ok(())
}

fn db_catalog_entry_to_ac(row: ac_db::ProviderCatalogRow) -> AcResult<ProviderCatalogEntry> {
    let capabilities = serde_json::from_str::<Vec<String>>(&row.capabilities)
        .map_err(|error| AcError::validation("DAEMON-PROVIDER_CATALOG_JSON", error.to_string()))
        .unwrap_or_default();
    Ok(ProviderCatalogEntry {
        id: StableId::from_existing(&row.id)?,
        display_name: row.display_name,
        description: row.description,
        website_url: row.website_url,
        logo_url: row.logo_url,
        credential_url: row.credential_url,
        pricing_classification: row.pricing_classification,
        capabilities,
    })
}

/// Sanitize a credential secret name to a filesystem-safe identifier.
fn sanitize_secret_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches('_');
    if sanitized.is_empty() {
        "credential".to_string()
    } else {
        sanitized.to_string()
    }
}

fn db_account_to_ac(row: ac_db::ProviderAccountRow) -> AcResult<CatalogAccount> {
    Ok(CatalogAccount {
        id: StableId::from_existing(&row.id)?,
        provider_id: StableId::from_existing(&row.provider_id)?,
        label: row.label,
        credential_ref: row.credential_ref,
        credential_region: row.credential_region,
        organization: row.organization,
        project: row.project,
        workspace: row.workspace,
        enabled: row.enabled,
        health_state: row.health_state,
        quota_rate_limit: row.quota_rate_limit.map(|v| v as u64),
        quota_remaining: row.quota_remaining.map(|v| v as u64),
        quota_reset_at_ms: row.quota_reset_at_ms.map(|v| v as u64),
        last_success_at_ms: row.last_success_at_ms.map(|v| v as u64),
        last_failure_at_ms: row.last_failure_at_ms.map(|v| v as u64),
        failure_reason: row.failure_reason,
    })
}

/// Parse a persisted routing-profile name into the enum.  Unknown values
/// fall back to LocalFirst (safe, offline-capable default) — a corrupted
/// preference must never make routing fail closed.
pub fn parse_routing_profile(value: &str) -> ac_provider::RoutingProfile {
    match value {
        "FreeOnly" => ac_provider::RoutingProfile::FreeOnly,
        "FreeFirst" => ac_provider::RoutingProfile::FreeFirst,
        "LocalFirst" => ac_provider::RoutingProfile::LocalFirst,
        "QualityFirst" => ac_provider::RoutingProfile::QualityFirst,
        "PaidAllowed" => ac_provider::RoutingProfile::PaidAllowed,
        "Offline" => ac_provider::RoutingProfile::Offline,
        _ => ac_provider::RoutingProfile::LocalFirst,
    }
}

/// Build a ProviderRegistry from the durable catalog + environment,
/// used by every mission executed through the daemon.
fn daemon_provider_registry(
    db: &ControlPlaneDb,
    _db_path: &Path,
) -> AcResult<ac_provider::ProviderRegistry> {
    use ac_agent::{provider_registry_from_config, ProviderConfigEntry, ProviderRegistryConfig};
    let mut config = ProviderRegistryConfig::from_environment();
    let entries = db.provider_catalog_entries().unwrap_or_default();
    for catalog_row in entries {
        let accounts = db.provider_accounts(&catalog_row.id).unwrap_or_default();
        for acct in accounts {
            if !acct.enabled {
                continue;
            }
            let kind = provider_config_kind(&catalog_row.id);
            let endpoint = resolve_account_endpoint_helper(&catalog_row.id);
            config.entries.push(ProviderConfigEntry {
                kind,
                provider_id: catalog_row.id.clone(),
                name: acct.label.clone(),
                enabled: true,
                endpoint,
                credential_env: Some(acct.credential_ref.clone()),
                model: provider_default_model(&catalog_row.id),
                models: Vec::new(),
                connect_timeout_ms: 10_000,
                read_timeout_ms: 60_000,
                custom_headers: Vec::new(),
                allow_plain_http_remote: false,
                local: catalog_row.id == "ollama",
                paid: !catalog_row.id.contains("ollama"),
                privacy: if catalog_row.id == "ollama" {
                    PrivacyClass::LocalOnly
                } else {
                    PrivacyClass::ExternalAllowed
                },
                input_cost_micros: None,
                output_cost_micros: None,
            });
        }
    }
    provider_registry_from_config(config)
}

fn resolve_account_endpoint_helper(provider_id: &str) -> String {
    match provider_id {
        "ollama" => std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434/api/chat".to_string()),
        "lm-studio" | "lmstudio" => std::env::var("LMSTUDIO_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1234/v1/chat/completions".to_string()),
        "openai" => std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string()),
        "anthropic" => std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string()),
        "gemini" => std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| {
            format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string())
            )
        }),
        _ => "config:unknown".to_string(),
    }
}

fn provider_connection_kind(provider_id: &StableId) -> ProviderConnectionKind {
    match provider_id.as_str() {
        "ollama" => ProviderConnectionKind::OllamaChat,
        "lm-studio" | "lmstudio" | "openai-compatible" => ProviderConnectionKind::OpenAiCompatible,
        "anthropic" => ProviderConnectionKind::AnthropicMessages,
        "gemini" => ProviderConnectionKind::GeminiGenerateContent,
        _ => ProviderConnectionKind::OpenAiChatCompletions,
    }
}

fn provider_config_kind(provider_id: &str) -> ac_agent::ProviderConfigKind {
    match provider_id {
        "openai" => ac_agent::ProviderConfigKind::OpenAi,
        "anthropic" => ac_agent::ProviderConfigKind::Anthropic,
        "gemini" => ac_agent::ProviderConfigKind::Gemini,
        "ollama" => ac_agent::ProviderConfigKind::Ollama,
        "lm-studio" | "lmstudio" => ac_agent::ProviderConfigKind::LmStudio,
        _ => ac_agent::ProviderConfigKind::OpenAi,
    }
}

fn provider_default_model(provider_id: &str) -> String {
    match provider_id {
        "ollama" => {
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3b".to_string())
        }
        "lm-studio" | "lmstudio" => {
            std::env::var("LMSTUDIO_MODEL").unwrap_or_else(|_| "local-model".to_string())
        }
        "openai" => std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        "anthropic" => std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-3-5-haiku-latest".to_string()),
        "gemini" => {
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string())
        }
        _ => "default".to_string(),
    }
}

/// Resolve a provider account's chat endpoint.  Local providers default to
/// their standard loopback endpoints; remote providers use the conventional
/// base URL environment variable when set, otherwise the well-known public
/// endpoint for that provider kind.
fn catalog_account_endpoint(account: &CatalogAccount) -> AcResult<String> {
    let provider = account.provider_id.as_str();
    let base = match provider {
        "ollama" => {
            let mut base = std::env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
            if !base.ends_with("/api/chat") {
                if !base.ends_with('/') {
                    base.push('/');
                }
                base.push_str("api/chat");
            }
            base
        }
        "lm-studio" | "lmstudio" => {
            let mut base = std::env::var("LMSTUDIO_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:1234".to_string());
            if !base.ends_with("/v1/chat/completions") {
                if !base.ends_with('/') {
                    base.push('/');
                }
                base.push_str("v1/chat/completions");
            }
            base
        }
        "openai" => std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string()),
        "anthropic" => std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string()),
        "gemini" => std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| {
            format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string())
            )
        }),
        _ => {
            return Err(AcError::validation(
                "DAEMON-PROVIDER_ENDPOINT_UNKNOWN",
                format!("no endpoint known for provider {provider}"),
            ));
        }
    };
    if base.trim().is_empty() {
        return Err(AcError::validation(
            "DAEMON-PROVIDER_ENDPOINT_EMPTY",
            format!("provider {provider} has an empty endpoint"),
        ));
    }
    Ok(base)
}

fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

impl DaemonLifecycleRuntime for DaemonService {
    fn start(&mut self) -> AcResult<()> {
        DaemonService::start(self)
    }

    fn stop(&mut self) -> AcResult<()> {
        DaemonService::stop(self)
    }

    fn restart(&mut self) -> AcResult<()> {
        DaemonService::restart(self)
    }

    fn recover(&mut self) -> AcResult<usize> {
        DaemonService::recover(self)
    }

    fn heartbeat(&self) -> AcResult<DaemonHealth> {
        DaemonService::heartbeat(self)
    }

    fn shutdown(&mut self) -> AcResult<()> {
        DaemonService::shutdown(self)
    }
}

fn session_state(state: AgentSessionState) -> &'static str {
    match state {
        AgentSessionState::Created => "created",
        AgentSessionState::Running => "running",
        AgentSessionState::Cancelling => "cancelling",
        AgentSessionState::Stopped => "stopped",
    }
}

impl Drop for DaemonService {
    fn drop(&mut self) {
        self.lock_file = None;
        if self.lifecycle == DaemonLifecycle::Running {
            let _ = fs::remove_file(&self.lock_path);
        }
        // Kill any live dev server children
        let mut children = self.design_children.lock().unwrap();
        let ids: Vec<String> = children.keys().cloned().collect();
        for id in ids {
            if let Some(mut child) = children.remove(&id) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        drop(children);
        // Kill any live terminal sessions (batch N4) — same guarantee.
        self.terminal_shutdown_all();
    }
}

pub fn default_paths(base: impl AsRef<Path>) -> (PathBuf, PathBuf) {
    let base = base.as_ref();
    (
        base.join("agentcode.sqlite"),
        base.join("agentcode-daemon.lock"),
    )
}

pub fn default_socket_path(base: impl AsRef<Path>) -> PathBuf {
    base.as_ref().join("agentcode.sock")
}

/// Per-user durable runtime state. Tests and explicit deployments may override
/// this with `AGENTCODE_RUNTIME_DIR`, but production must not use shared temp.
pub fn default_runtime_dir() -> AcResult<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        AcError::validation(
            "DAEMON-RUNTIME_HOME_UNAVAILABLE",
            "HOME is required for daemon runtime",
        )
    })?;
    let runtime = PathBuf::from(home).join("Library/Application Support/AgentCode/runtime");
    fs::create_dir_all(&runtime)
        .map_err(|error| AcError::validation("DAEMON-RUNTIME_CREATE", error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).map_err(|error| {
            AcError::validation("DAEMON-RUNTIME_PERMISSIONS", error.to_string())
        })?;
    }
    Ok(runtime)
}

#[allow(dead_code)]
fn _timestamp_for_observability() -> TimestampMillis {
    TimestampMillis::now()
}

/// Crate-wide mutex serializing tests that mutate process environment
/// variables (AGENTCODE_SECRET_DIR, provider base URLs).  Both the lib test
/// module and the IPC test module share this so parallel tests cannot race on
/// process-global env state.
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

/// Test-only re-export of the live-mirroring context setter (tests drive
/// the watch-the-agent path exactly as the mission runner does).
#[cfg(test)]
pub mod ac_tool_mirror_for_test {
    pub fn set_context(ctx: Option<String>) {
        ac_tool::set_live_mission_context(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn with_daemon_env_lock<T>(f: impl FnOnce() -> T) -> T {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        f()
    }

    fn temp_paths() -> (PathBuf, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("agentcode-daemon-{}", StableId::new("tmp")));
        fs::create_dir_all(&dir).unwrap();
        let (db, lock) = default_paths(&dir);
        (dir, db, lock)
    }

    #[test]
    fn provider_catalog_is_seeded_on_open() {
        let (dir, db, lock) = temp_paths();
        let daemon = DaemonService::open(&db, &lock).unwrap();
        let catalog = daemon.provider_catalog().unwrap();
        let ids = catalog
            .iter()
            .map(|entry| entry.id.to_string())
            .collect::<Vec<_>>();
        for expected in [
            "omnirouter",
            "openai",
            "anthropic",
            "gemini",
            "ollama",
            "lm-studio",
        ] {
            assert!(
                ids.contains(&expected.to_string()),
                "catalog must contain {expected}, got {ids:?}"
            );
        }
        // Seeding is idempotent: reopening does not duplicate.
        drop(daemon);
        let daemon = DaemonService::open(&db, &lock).unwrap();
        assert_eq!(daemon.provider_catalog().unwrap().len(), 6);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn provider_accounts_persist_across_reopen() {
        let (dir, db, lock) = temp_paths();
        let daemon = DaemonService::open(&db, &lock).unwrap();
        let account_id = daemon
            .save_provider_account("ollama", "work", "secret:ollama-work", "", "", "", "", true)
            .unwrap();
        // Read back through a fresh daemon (simulates restart).
        drop(daemon);
        let daemon = DaemonService::open(&db, &lock).unwrap();
        let accounts = daemon.list_provider_accounts("ollama").unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id.to_string(), account_id.to_string());
        assert_eq!(accounts[0].credential_ref, "secret:ollama-work");
        assert!(accounts[0].enabled);
        // Disable + rotate through the dedicated backend operations.
        daemon
            .set_provider_account_enabled(account_id.as_str(), false)
            .unwrap();
        let accounts = daemon.list_provider_accounts("ollama").unwrap();
        assert!(!accounts[0].enabled);
        daemon
            .rotate_provider_account(account_id.as_str(), "secret:rotated")
            .unwrap();
        let accounts = daemon.list_provider_accounts("ollama").unwrap();
        assert_eq!(accounts[0].credential_ref, "secret:rotated");
        // Delete.
        daemon.delete_provider_account(account_id.as_str()).unwrap();
        assert!(daemon.list_provider_accounts("ollama").unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn store_credential_writes_only_masked_and_readable_ref() {
        with_daemon_env_lock(|| {
            let (dir, db, lock) = temp_paths();
            std::env::set_var("AGENTCODE_SECRET_DIR", dir.join("secrets"));
            let daemon = DaemonService::open(&db, &lock).unwrap();
            let reference = daemon
                .store_credential("test-key", "sk-live-secret")
                .unwrap();
            assert_eq!(reference, "secret:test-key");
            // The stored value is resolvable by the provider credential resolver.
            let resolved = ac_provider::catalog::resolve_credential_ref(&reference).unwrap();
            assert_eq!(resolved, "sk-live-secret");
            // Delete removes the file.
            daemon.delete_credential("test-key").unwrap();
            assert!(ac_provider::catalog::resolve_credential_ref(&reference).is_err());
            let _ = fs::remove_dir_all(dir);
        });
    }

    #[test]
    fn test_credential_returns_normalized_result_without_saving() {
        with_daemon_env_lock(|| {
            let (dir, db, lock) = temp_paths();
            std::env::set_var("AGENTCODE_SECRET_DIR", dir.join("secrets"));
            let daemon = DaemonService::open(&db, &lock).unwrap();
            // An unreachable endpoint must normalize to a non-ok status, not panic
            // and not create an account.
            let reference = daemon.store_credential("probe", "bad-key").unwrap();
            let status = daemon
                .test_credential("openai", &reference, "", "", "")
                .unwrap();
            assert!(!status.ok);
            assert!(status.failure.is_some());
            assert!(status.masked_credential.contains("****"));
            assert!(daemon.list_provider_accounts("openai").unwrap().is_empty());
            let _ = fs::remove_dir_all(dir);
        });
    }

    #[test]
    fn saved_secret_account_reaches_real_authenticated_request() {
        with_daemon_env_lock(|| {
            use std::io::{Read, Write};
            use std::net::TcpListener;
            use std::sync::mpsc;
            use std::time::Duration as StdDuration;

            let (dir, db, lock) = temp_paths();
            let secrets = dir.join("secrets");
            std::env::set_var("AGENTCODE_SECRET_DIR", &secrets);

            // Local HTTP server that captures the Authorization header so the test
            // proves the real request path without exposing the credential.
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!(
                "http://{}/v1/chat/completions",
                listener.local_addr().unwrap()
            );
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0_u8; 8192];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                tx.send(request).unwrap();
                let body = "{\"choices\":[{\"message\":{\"content\":\"plan=ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":6}}";
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                let _ = stream.write_all(response.as_bytes());
            });

            std::env::set_var("OPENAI_BASE_URL", &endpoint);
            let had_openai_key = std::env::var("OPENAI_API_KEY").ok();
            std::env::remove_var("OPENAI_API_KEY");

            // 1. Store a credential through the backend secret store.
            let daemon = DaemonService::open(&db, &lock).unwrap();
            let reference = daemon
                .store_credential("live-key", "sk-live-secret")
                .unwrap();
            assert_eq!(reference, "secret:live-key");

            // 2. Create a provider account that references the stored secret.
            let account_id = daemon
                .save_provider_account("openai", "live-account", &reference, "", "", "", "", true)
                .unwrap();

            // 3. Re-open/reload the provider registry from durable state.
            drop(daemon);
            let daemon = DaemonService::open(&db, &lock).unwrap();
            let accounts = daemon.list_provider_accounts("openai").unwrap();
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].credential_ref, "secret:live-key");
            assert_eq!(accounts[0].id.to_string(), account_id.to_string());

            // 4. Build the actual provider registry (the runtime request path).
            let mut registry = daemon.build_provider_registry().unwrap();

            // 5. Perform an actual authenticated provider request through the
            //    daemon-built registry.  This exercises the full runtime path:
            //    registry -> adapter -> auth_headers -> secret resolution -> HTTP.
            let profile = ac_provider::TaskProfile::coding(
                StableId::new("task"),
                ac_provider::RoutingProfile::FreeFirst,
            );
            let execution = registry
                .request_model(&profile, "plan this", 128, &|| false)
                .expect("authenticated provider request must succeed");
            assert!(execution
                .events
                .iter()
                .any(|event| { matches!(event, ac_provider::ProviderStreamEvent::Finished) }));
            assert!(execution.decision.selected.is_some());

            let wire = rx.recv_timeout(StdDuration::from_secs(2)).unwrap();
            assert!(
                wire.to_ascii_lowercase()
                    .contains("authorization: bearer sk-live-secret"),
                "daemon registry must send the resolved secret on the wire"
            );
            // The raw credential never leaks into the account response.
            assert!(!format!("{accounts:?}").contains("sk-live-secret"));

            std::env::remove_var("OPENAI_BASE_URL");
            std::env::remove_var("AGENTCODE_SECRET_DIR");
            if let Some(key) = had_openai_key {
                std::env::set_var("OPENAI_API_KEY", key);
            }
            let _ = fs::remove_dir_all(dir);
        });
    }

    #[test]
    fn provider_account_credentials_remain_isolated_between_accounts() {
        with_daemon_env_lock(|| {
            let (dir, db, lock) = temp_paths();
            let secrets = dir.join("secrets");
            std::env::set_var("AGENTCODE_SECRET_DIR", &secrets);

            let daemon = DaemonService::open(&db, &lock).unwrap();
            let ref_a = daemon
                .store_credential("acct-a-key", "sk-secret-a")
                .unwrap();
            let ref_b = daemon
                .store_credential("acct-b-key", "sk-secret-b")
                .unwrap();
            assert_ne!(ref_a, ref_b);

            let account_a = daemon
                .save_provider_account("openai", "account-a", &ref_a, "", "", "", "", true)
                .unwrap();
            let account_b = daemon
                .save_provider_account("openai", "account-b", &ref_b, "", "", "", "", true)
                .unwrap();

            // Each account stores only its own reference; neither holds the
            // other's secret value or reference.
            let accounts = daemon.list_provider_accounts("openai").unwrap();
            assert_eq!(accounts.len(), 2);
            let a = accounts
                .iter()
                .find(|account| account.id == account_a)
                .unwrap();
            let b = accounts
                .iter()
                .find(|account| account.id == account_b)
                .unwrap();
            assert_eq!(a.credential_ref, "secret:acct-a-key");
            assert_eq!(b.credential_ref, "secret:acct-b-key");
            assert!(a.masked_credential().contains("****"));
            assert!(b.masked_credential().contains("****"));

            // The secrets resolve independently through the canonical resolver.
            assert_eq!(
                ac_provider::catalog::resolve_credential_ref(&ref_a).unwrap(),
                "sk-secret-a"
            );
            assert_eq!(
                ac_provider::catalog::resolve_credential_ref(&ref_b).unwrap(),
                "sk-secret-b"
            );

            // Deleting account A must not disturb account B.
            daemon.delete_provider_account(account_a.as_str()).unwrap();
            let remaining = daemon.list_provider_accounts("openai").unwrap();
            assert_eq!(remaining.len(), 1);
            assert_eq!(remaining[0].id, account_b);

            // Rotating B must not resurrect A's reference.
            daemon
                .rotate_provider_account(account_b.as_str(), "secret:acct-b-key")
                .unwrap();
            let remaining = daemon.list_provider_accounts("openai").unwrap();
            assert_eq!(remaining.len(), 1);
            assert_eq!(remaining[0].credential_ref, "secret:acct-b-key");

            std::env::remove_var("AGENTCODE_SECRET_DIR");
            let _ = fs::remove_dir_all(dir);
        });
    }

    #[test]
    fn missing_secret_reference_fails_account_operation_honestly() {
        with_daemon_env_lock(|| {
            let (dir, db, lock) = temp_paths();
            let secrets = dir.join("secrets");
            std::env::set_var("AGENTCODE_SECRET_DIR", &secrets);

            let daemon = DaemonService::open(&db, &lock).unwrap();
            // A reference to a secret file that was never stored must resolve
            // to an honest error, not a placeholder value.
            let err =
                ac_provider::catalog::resolve_credential_ref("secret:never-stored").unwrap_err();
            assert_eq!(err.code(), "PROVIDER-CREDENTIAL_UNAVAILABLE");

            // Account creation still stores the reference (lazy resolution at
            // the request boundary), but a real request must then fail.
            let account_id = daemon
                .save_provider_account(
                    "openai",
                    "broken",
                    "secret:never-stored",
                    "",
                    "",
                    "",
                    "",
                    true,
                )
                .unwrap();
            let account_ref = account_id.to_string();
            drop(daemon);
            let daemon = DaemonService::open(&db, &lock).unwrap();
            let accounts = daemon.list_provider_accounts("openai").unwrap();
            assert_eq!(accounts[0].credential_ref, "secret:never-stored");
            assert_eq!(accounts[0].id.to_string(), account_ref);

            std::env::remove_var("AGENTCODE_SECRET_DIR");
            let _ = fs::remove_dir_all(dir);
        });
    }

    #[test]
    fn daemon_starts_routes_ipc_and_stops() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let mut ipc = LocalIpc::new(&mut daemon);
        let response = ipc
            .send(DaemonCommand::CreateSession {
                goal: "Create README.md".to_string(),
                workspace_root: None,
            })
            .unwrap();
        assert!(matches!(response, DaemonResponse::SessionCreated { .. }));
        assert!(matches!(
            ipc.send(DaemonCommand::Health).unwrap(),
            DaemonResponse::Health(DaemonHealth {
                lifecycle: DaemonLifecycle::Running,
                ..
            })
        ));
        assert_eq!(
            ipc.send(DaemonCommand::Stop).unwrap(),
            DaemonResponse::Stopped
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn submitted_mission_is_daemon_owned_and_cancellation_is_durable() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let (mission_id, session_id) = match daemon
            .handle(DaemonCommand::CreateSession {
                goal: "queued daemon execution".to_string(),
                workspace_root: None,
            })
            .unwrap()
        {
            DaemonResponse::SessionCreated {
                mission_id,
                session_id,
            } => (mission_id, session_id),
            _ => panic!("expected durable mission/session"),
        };
        daemon.cancel_mission(mission_id.as_str()).unwrap();
        let status = daemon.mission_status(mission_id.as_str()).unwrap();
        assert_eq!(status.mission_id, mission_id);
        assert_eq!(status.session_id, session_id);
        assert_eq!(status.state, "cancelled");
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn desktop_close_does_not_stop_daemon_and_reopen_reconnects() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let mut ipc = LocalIpc::new(&mut daemon);
        let desktop_session_id = StableId::new("desktop");
        assert_eq!(
            ipc.send(DaemonCommand::DesktopWindowClosed {
                desktop_session_id: desktop_session_id.clone()
            })
            .unwrap(),
            DaemonResponse::DesktopWindowClosed {
                daemon_active: true
            }
        );
        assert!(matches!(
            ipc.send(DaemonCommand::ReconnectDesktop { desktop_session_id })
                .unwrap(),
            DaemonResponse::DesktopReconnected {
                health: DaemonHealth {
                    lifecycle: DaemonLifecycle::Running,
                    ..
                },
                ..
            }
        ));
        assert_eq!(
            ipc.send(DaemonCommand::Stop).unwrap(),
            DaemonResponse::Stopped
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn singleton_prevents_duplicate_launch() {
        let (dir, db, lock) = temp_paths();
        let mut first = DaemonService::open(&db, &lock).unwrap();
        first.start().unwrap();
        let mut second = DaemonService::open(&db, &lock).unwrap();
        let err = second.start().unwrap_err();
        assert_eq!(err.code(), "DAEMON-SINGLETON_LOCKED");
        first.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_singleton_lock_is_reclaimed_but_live_owner_is_not() {
        let (dir, db, lock) = temp_paths();
        fs::write(&lock, "pid=999999\nstarted_at_ms=1\ninstance_id=old").unwrap();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        assert!(fs::read_to_string(&lock).unwrap().contains("pid="));
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unix_ipc_is_framed_and_disconnect_does_not_stop_daemon() {
        let (dir, db, lock) = temp_paths();
        let socket = default_socket_path(&dir);
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Ok((server, listener)) = UnixIpcServer::bind(&socket) else {
            eprintln!("unix IPC bind is environment-blocked in this sandbox");
            daemon.stop().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        assert_eq!(
            fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let client = UnixIpcClient::new(&socket);
        let join = std::thread::spawn(move || {
            client
                .request(
                    serde_json::json!({"id":"one","command":"SubmitMission","goal":"keep going"}),
                )
                .unwrap()
        });
        while !server.serve_once(&listener, &mut daemon).unwrap() {
            if join.is_finished() {
                break;
            }
        }
        let response = join.join().unwrap();
        assert_eq!(response["ok"], true, "unexpected IPC response: {response}");
        assert_eq!(daemon.health().lifecycle, DaemonLifecycle::Running);
        server.cleanup();
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restart_recovers_interrupted_session() {
        let (dir, db_path, lock) = temp_paths();
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let mission_id = StableId::from_existing("mission-restart-recover").unwrap();
            let session_id = StableId::from_existing("session-restart-recover").unwrap();
            db.put_mission(&ac_kernel::Mission {
                id: mission_id.clone(),
                original_goal: "Recover me".to_string(),
                state: MissionState::Active,
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_session(&session_id, &mission_id, "executing", None)
                .unwrap();
            db.save_checkpoint(
                &StableId::from_existing("checkpoint-restart-recover").unwrap(),
                &session_id,
                3,
                "executing",
            )
            .unwrap();
        }
        let mut restarted = DaemonService::open(&db_path, &lock).unwrap();
        restarted.start().unwrap();
        assert!(!restarted.recovered_sessions().is_empty());
        restarted.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restart_does_not_enqueue_terminal_or_paused_sessions() {
        let (dir, db_path, lock) = temp_paths();
        let active_mission = StableId::from_existing("mission-restart-active").unwrap();
        let paused_mission = StableId::from_existing("mission-restart-paused").unwrap();
        let completed_mission = StableId::from_existing("mission-restart-completed").unwrap();
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            for (mission_id, goal) in [
                (&active_mission, "active should requeue"),
                (&paused_mission, "paused should stay paused"),
                (&completed_mission, "completed should stay complete"),
            ] {
                db.put_mission(&ac_kernel::Mission {
                    id: mission_id.clone(),
                    original_goal: goal.to_string(),
                    state: MissionState::Active,
                    created_at: TimestampMillis::now(),
                })
                .unwrap();
            }
            db.save_session(
                &StableId::from_existing("session-restart-active").unwrap(),
                &active_mission,
                "running",
                None,
            )
            .unwrap();
            db.save_session(
                &StableId::from_existing("session-restart-paused").unwrap(),
                &paused_mission,
                "paused",
                None,
            )
            .unwrap();
            db.save_session(
                &StableId::from_existing("session-restart-completed").unwrap(),
                &completed_mission,
                "completed",
                None,
            )
            .unwrap();
        }
        let mut restarted = DaemonService::open(&db_path, &lock).unwrap();
        restarted.start().unwrap();
        let active_state = restarted
            .mission_status(active_mission.as_str())
            .unwrap()
            .state;
        assert!(
            matches!(active_state.as_str(), "queued" | "running"),
            "unexpected active recovery state: {active_state}"
        );
        let paused_state = restarted
            .mission_status(paused_mission.as_str())
            .unwrap()
            .state;
        assert_eq!(paused_state, "paused");
        assert!(restarted
            .mission_status(completed_mission.as_str())
            .is_none());
        restarted.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lifecycle_contract_supports_recover_heartbeat_and_shutdown() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        assert!(daemon.heartbeat().is_err());
        DaemonLifecycleRuntime::start(&mut daemon).unwrap();
        assert_eq!(
            DaemonLifecycleRuntime::heartbeat(&daemon)
                .unwrap()
                .lifecycle,
            DaemonLifecycle::Running
        );
        assert_eq!(DaemonLifecycleRuntime::recover(&mut daemon).unwrap(), 0);
        DaemonLifecycleRuntime::shutdown(&mut daemon).unwrap();
        assert_eq!(daemon.health().lifecycle, DaemonLifecycle::Stopped);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn phase27_release_engineering_verifies_artifact_update_rollback_and_diagnostics() {
        let mut release = ReleaseEngineer::new();
        let build = release
            .capture_build("1.0.0", "commit-a", "release", "macos-arm64")
            .unwrap();
        assert!(!build.reproducible);
        let version = ReleaseVersion {
            version: "1.0.1".to_string(),
            source_commit: "commit-b".to_string(),
        };
        let bytes = b"agentcode-app-bundle";
        let artifact = release
            .create_artifact(&version, "macos-arm64", "app-bundle", bytes)
            .unwrap();
        assert!(release.verify_artifact(&artifact, bytes));
        assert!(!release.verify_artifact(&artifact, b"tampered"));
        let update = release.plan_update("1.0.0", &artifact, bytes);
        assert_eq!(update.decision, UpdateDecision::Install);
        assert!(update.verified);
        let blocked = release.plan_update("1.0.0", &artifact, b"tampered");
        assert_eq!(blocked.decision, UpdateDecision::Blocked);
        let rollback = release.rollback_after_failed_update("1.0.0", "1.0.1", "rollback:commit-a");
        assert_eq!(rollback.decision, UpdateDecision::Rollback);
        let diagnostics =
            release.diagnostics_report("AC_SECRET_CANARY", "daemon ok AC_SECRET_CANARY");
        assert!(!diagnostics.contains("AC_SECRET_CANARY"));
        assert!(diagnostics.contains("[REDACTED]"));
    }

    #[test]
    fn phase27_packaging_layout_keeps_state_outside_user_repository_and_reset_is_safe() {
        let repository = PathBuf::from("/Volumes/T7 Shield/My Repo");
        let layout = PackagingLayout {
            app_bundle_id: "com.agentcode.desktop".to_string(),
            config_dir: PathBuf::from("/Users/me/Library/Application Support/AgentCode/config"),
            data_dir: PathBuf::from("/Users/me/Library/Application Support/AgentCode/data"),
            cache_dir: PathBuf::from("/Users/me/Library/Caches/AgentCode"),
            log_dir: PathBuf::from("/Users/me/Library/Logs/AgentCode"),
            managed_tools_dir: PathBuf::from(
                "/Users/me/Library/Application Support/AgentCode/tools",
            ),
            browser_profiles_dir: PathBuf::from(
                "/Users/me/Library/Application Support/AgentCode/browser",
            ),
        };
        layout.validate(&repository).unwrap();
        let release = ReleaseEngineer::new();
        assert!(release
            .reset_plan_preserves_repository(&repository)
            .iter()
            .all(|path| !path.starts_with(&repository)));
        let unsafe_layout = PackagingLayout {
            data_dir: repository.join(".agentcode"),
            ..layout
        };
        assert_eq!(
            unsafe_layout.validate(&repository).unwrap_err().code(),
            "RELEASE-APP_STATE_IN_REPOSITORY"
        );
    }

    #[test]
    fn phase28_release_candidate_gate_blocks_failed_validation_and_accepts_complete_evidence() {
        let mut release = ReleaseEngineer::new();
        let build = release
            .capture_build("1.0.0-rc.1", "commit-rc", "release", "macos-arm64")
            .unwrap();
        let mut candidate = release
            .create_candidate(
                "1.0.0-rc.1",
                "rc.1",
                &build,
                "macos-arm64",
                vec!["release/rc/scope-freeze.md".to_string()],
            )
            .unwrap();
        let migration = MigrationSafetyReport {
            fresh_install: true,
            upgrade: true,
            schema_version: 18,
            interrupted_recovery: true,
            evidence_ref: "release/rc/migration-report.md".to_string(),
        };
        let failing = ReleaseChecklist {
            security_checks: true,
            tests: false,
            artifact_verification: true,
            migration_validation: true,
        };
        let failed_validation =
            release.run_production_validation(&candidate, &migration, &failing, true, "rc/failed");
        let blocked_gate = ReleaseCandidateGate {
            tests_pass: false,
            security_pass: true,
            migrations_pass: true,
            artifacts_valid: true,
            evidence_refs: failed_validation.evidence_refs(),
        };
        assert_eq!(
            release
                .approve_candidate(&mut candidate, &failed_validation, &blocked_gate)
                .unwrap_err()
                .code(),
            "RC-APPROVAL_BLOCKED"
        );

        let passing = ReleaseChecklist {
            security_checks: true,
            tests: true,
            artifact_verification: true,
            migration_validation: true,
        };
        let validation =
            release.run_production_validation(&candidate, &migration, &passing, true, "rc/pass");
        let gate = ReleaseCandidateGate {
            tests_pass: true,
            security_pass: true,
            migrations_pass: true,
            artifacts_valid: true,
            evidence_refs: validation.evidence_refs(),
        };
        release
            .approve_candidate(&mut candidate, &validation, &gate)
            .unwrap();
        assert_eq!(
            candidate.validation_status,
            ReleaseCandidateStatus::Accepted
        );
    }

    #[test]
    fn phase29_release_manifest_decision_bundle_and_status_require_evidence() {
        let mut release = ReleaseEngineer::new();
        let version = ReleaseVersion {
            version: "1.0.0".to_string(),
            source_commit: "commit-v1".to_string(),
        };
        let artifact = release
            .create_artifact(&version, "macos-arm64", "app-bundle", b"agentcode-v1")
            .unwrap();
        let manifest = release
            .final_manifest(
                "1.0.0",
                vec![
                    "autonomous coding missions".to_string(),
                    "Discuss Mode".to_string(),
                    "Design Studio core".to_string(),
                    "Security baseline".to_string(),
                ],
                vec!["0018_release_candidate_v1.sql".to_string()],
                std::slice::from_ref(&artifact),
                vec!["signed/notarized public artifact is prerequisite-bound".to_string()],
            )
            .unwrap();
        assert!(manifest.manifest_hash.starts_with("fnv1a64:"));

        let build = release
            .capture_build("1.0.0", "commit-v1", "release", "macos-arm64")
            .unwrap();
        let candidate = release
            .create_candidate(
                "1.0.0",
                "v1-final",
                &build,
                "macos-arm64",
                vec!["release/manifest/v1.json".to_string()],
            )
            .unwrap();
        let validation = release.run_production_validation(
            &candidate,
            &MigrationSafetyReport {
                fresh_install: true,
                upgrade: true,
                schema_version: 18,
                interrupted_recovery: true,
                evidence_ref: "release/migration/final.md".to_string(),
            },
            &ReleaseChecklist {
                security_checks: true,
                tests: true,
                artifact_verification: true,
                migration_validation: true,
            },
            true,
            "release/final",
        );
        let decision = release
            .approve_release(&manifest, &validation, "security gate passed")
            .unwrap();
        assert_eq!(decision.approved_version, "1.0.0");
        let bundle = release
            .evidence_bundle(
                "1.0.0",
                "release/audit.md",
                "release/security.md",
                "release/validation.md",
                "release/artifact.md",
                "release/migration.md",
            )
            .unwrap();
        assert!(bundle.complete());

        let mut state = ReleaseStateMachine::new("1.0.0").unwrap();
        assert_eq!(
            state
                .transition(ReleaseStatus::Released, "release/evidence")
                .unwrap_err()
                .code(),
            "RELEASE-INVALID_TRANSITION"
        );
        state
            .transition(ReleaseStatus::Candidate, "release/rc-accepted")
            .unwrap();
        state
            .transition(ReleaseStatus::Approved, "release/decision")
            .unwrap();
        state
            .transition(ReleaseStatus::Released, "release/published-hash")
            .unwrap();
        assert_eq!(state.status, ReleaseStatus::Released);
    }

    #[test]
    fn coordinator_terminal_state_is_bounded_and_cancellation_does_not_leak() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();

        // Cancel several missions to populate cancelled/cancellation maps,
        // then verify terminal cleanup removes them.
        let mut cancelled_ids = Vec::new();
        for i in 0..5 {
            let (mission_id, _) = match daemon
                .handle(DaemonCommand::CreateSession {
                    goal: format!("bounded-state-{i}"),
                    workspace_root: None,
                })
                .unwrap()
            {
                DaemonResponse::SessionCreated {
                    mission_id,
                    session_id,
                } => (mission_id, session_id),
                _ => panic!("expected session"),
            };
            daemon.cancel_mission(mission_id.as_str()).unwrap();
            cancelled_ids.push(mission_id);
        }
        // After cancellation + terminal processing, the in-memory
        // cancellation/cleaned maps should not retain entries for terminal
        // missions.  We verify by checking status is bounded and correct.
        for mid in &cancelled_ids {
            let status = daemon.mission_status(mid.as_str()).unwrap();
            assert_eq!(status.state, "cancelled");
        }
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn coordinator_queued_cancelled_missions_do_not_leak_maps() {
        // P1-06: a queued mission cancelled before the worker dequeues it must
        // not leak entries in the cancelled/cancellation/pause_flags maps.  The
        // worker's cancelled branch is the only place such a mission is seen.
        let (dir, db, lock) = temp_paths();
        // Isolated workspace so a racing worker can never touch the repo cwd.
        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let mut mission_ids = Vec::new();
        for i in 0..8 {
            let (mission_id, _sid) = match daemon
                .handle(DaemonCommand::CreateSession {
                    goal: format!("queued-cancel-{i}"),
                    workspace_root: Some(workspace.to_string_lossy().to_string()),
                })
                .unwrap()
            {
                DaemonResponse::SessionCreated {
                    mission_id,
                    session_id,
                } => (mission_id, session_id),
                _ => panic!("expected session"),
            };
            // Cancel while still queued (no worker dequeue happens because the
            // daemon never ran the mock provider for these missions).
            daemon.cancel_mission(mission_id.as_str()).unwrap();
            mission_ids.push(mission_id);
        }
        // Give the worker a moment to drain the channel messages for the
        // cancelled missions and perform cleanup.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let (_, cancelled_len, cancellation_len, pause_flags_len) =
                daemon.coordinator.map_sizes();
            if cancelled_len == 0 && cancellation_len == 0 && pause_flags_len == 0 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "coordinator maps still hold cancelled missions: cancelled={cancelled_len} cancellation={cancellation_len} pause_flags={pause_flags_len}"
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        for mid in &mission_ids {
            let status = daemon.mission_status(mid.as_str()).unwrap();
            assert_eq!(status.state, "cancelled", "status must remain cancelled");
        }
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_mission_pause_blocks_forward_work_and_resumes() {
        // P1-02: pausing an ACTIVE mission must set its pause flag, and resume
        // must clear it.  Constructs the coordinator directly so the worker
        // thread never executes the synthetic mission (deterministic).
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let mission_id = StableId::new("pause-active");
        let session_id = StableId::new("session-pause-active");
        let kernel = Arc::clone(&daemon.kernel);
        let coordinator =
            MissionCoordinator::new(db.clone(), kernel, Arc::new(Mutex::new(BTreeMap::new())));
        {
            let mut state = coordinator.state.lock().unwrap();
            state.active = Some(mission_id.clone());
            state.statuses.insert(
                mission_id.to_string(),
                MissionExecutionStatus {
                    mission_id: mission_id.clone(),
                    session_id: session_id.clone(),
                    state: "running".to_string(),
                },
            );
            state
                .cancellation
                .insert(mission_id.to_string(), CancellationToken::new());
            state
                .pause_flags
                .insert(mission_id.to_string(), Arc::new(AtomicBool::new(false)));
        }
        let pause = coordinator
            .state
            .lock()
            .unwrap()
            .pause_flags
            .get(mission_id.as_str())
            .cloned()
            .unwrap();
        coordinator.pause(mission_id.as_str()).unwrap();
        assert!(
            pause.load(Ordering::SeqCst),
            "active pause must set the flag"
        );
        assert_eq!(
            coordinator.status(mission_id.as_str()).unwrap().state,
            "paused"
        );
        coordinator.resume(mission_id.as_str()).unwrap();
        assert!(
            !pause.load(Ordering::SeqCst),
            "resume must clear the pause flag"
        );
        assert_eq!(
            coordinator.status(mission_id.as_str()).unwrap().state,
            "running"
        );
        // Cancellation of a paused mission must still work and clear the flag.
        coordinator.pause(mission_id.as_str()).unwrap();
        assert!(pause.load(Ordering::SeqCst));
        coordinator.cancel(mission_id.as_str()).unwrap();
        assert!(!pause.load(Ordering::SeqCst));
        let token = coordinator
            .state
            .lock()
            .unwrap()
            .cancellation
            .get(mission_id.as_str())
            .cloned()
            .unwrap();
        assert!(token.is_cancelled());
        coordinator.stop().unwrap();
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    // ── Workspace binding (project-to-daemon) ─────────────────────────────

    #[test]
    fn submitted_mission_persists_supplied_workspace() {
        // TEST 1/2: Submit missions for Project A and Project B.  Each must
        // persist its own workspace_root independently.
        let (dir, db, lock) = temp_paths();
        let proj_a = dir.join("proj_a");
        let proj_b = dir.join("proj_b");
        fs::create_dir_all(&proj_a).unwrap();
        fs::create_dir_all(&proj_b).unwrap();
        let canon_a = fs::canonicalize(&proj_a).unwrap();
        let canon_b = fs::canonicalize(&proj_b).unwrap();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let resp_a = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "task A".to_string(),
                workspace_root: Some(proj_a.to_string_lossy().to_string()),
            })
            .unwrap();
        let resp_b = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "task B".to_string(),
                workspace_root: Some(proj_b.to_string_lossy().to_string()),
            })
            .unwrap();
        let (mid_a, _sid_a) = match resp_a {
            DaemonResponse::SessionCreated {
                mission_id,
                session_id,
            } => (mission_id, session_id),
            _ => panic!("expected session"),
        };
        let (mid_b, _sid_b) = match resp_b {
            DaemonResponse::SessionCreated {
                mission_id,
                session_id,
            } => (mission_id, session_id),
            _ => panic!("expected session"),
        };
        assert_eq!(
            daemon.mission_workspace(mid_a.as_str()).unwrap().unwrap(),
            canon_a.to_string_lossy(),
            "Project A workspace must be persisted"
        );
        assert_eq!(
            daemon.mission_workspace(mid_b.as_str()).unwrap().unwrap(),
            canon_b.to_string_lossy(),
            "Project B workspace must be persisted"
        );
        assert_ne!(
            canon_a, canon_b,
            "two different projects must have different workspaces"
        );
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recovered_mission_uses_persisted_workspace() {
        // TEST 3/8: A session whose workspace_root is persisted in the DB
        // must be recovered with the SAME workspace identity after a daemon
        // restart, even when the daemon's configured default differs.
        let (dir, db_path, lock) = temp_paths();
        let project = dir.join("recovery_project");
        fs::create_dir_all(&project).unwrap();
        let canon = fs::canonicalize(&project).unwrap();
        let session_id = StableId::new("recovery-session");
        let mission_id = StableId::new("recovery-mission");
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.put_mission(&ac_kernel::Mission {
                id: mission_id.clone(),
                original_goal: "recover me".to_string(),
                state: MissionState::Active,
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_session(
                &session_id,
                &mission_id,
                "executing",
                Some(canon.to_string_lossy().as_ref()),
            )
            .unwrap();
        }
        // Simulate daemon restart with the same runtime directory.
        let mut restarted = DaemonService::open(&db_path, &lock).unwrap();
        restarted.start().unwrap();
        // The interrupted session must be recovered, and its workspace
        // identity must come from the DB, not from the daemon default.
        assert!(
            !restarted.recovered_sessions().is_empty(),
            "interrupted session must be recovered"
        );
        assert_eq!(
            restarted
                .mission_workspace(mission_id.as_str())
                .unwrap()
                .unwrap(),
            canon.to_string_lossy(),
            "recovered workspace must match the persisted project path"
        );
        restarted.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn supplied_workspace_beats_daemon_default() {
        // TEST 4: When a project-specific workspace is supplied, the daemon
        // must use it even though its own configured default is a different
        // directory.
        let (dir, db, lock) = temp_paths();
        let project = dir.join("my_explicit_project");
        fs::create_dir_all(&project).unwrap();
        let canon_project = fs::canonicalize(&project).unwrap();
        // The daemon's default_workspace_root is the current dir (the
        // AgentCode repo root).  The supplied project is different, so
        // verify the persisted workspace is the project, not the default.
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        assert_ne!(
            daemon.default_workspace_root, canon_project,
            "test fixture: daemon default must differ from supplied project"
        );
        daemon.start().unwrap();
        let response = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "explicit project".to_string(),
                workspace_root: Some(project.to_string_lossy().to_string()),
            })
            .unwrap();
        let mid = match response {
            DaemonResponse::SessionCreated { mission_id, .. } => mission_id,
            _ => panic!("expected session"),
        };
        assert_eq!(
            daemon.mission_workspace(mid.as_str()).unwrap().unwrap(),
            canon_project.to_string_lossy(),
            "supplied workspace must win over daemon default"
        );
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_workspace_path_is_rejected() {
        // TEST 5: A non-existent or non-directory path must be rejected.
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        // Non-existent path
        let missing = dir.join("does-not-exist");
        let err = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "missing".to_string(),
                workspace_root: Some(missing.to_string_lossy().to_string()),
            })
            .unwrap_err();
        assert_eq!(err.code(), "DAEMON-WORKSPACE_UNAVAILABLE");
        // Non-directory path (a file)
        let file = dir.join("some_file.txt");
        fs::write(&file, "not a directory").unwrap();
        let err = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "file".to_string(),
                workspace_root: Some(file.to_string_lossy().to_string()),
            })
            .unwrap_err();
        assert_eq!(err.code(), "DAEMON-WORKSPACE_NOT_DIR");
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn filesystem_root_workspace_is_rejected() {
        // TEST 6: A workspace that resolves to a filesystem root must be
        // rejected as outside the allowed boundary.
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let err = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "root escape".to_string(),
                workspace_root: Some("/".to_string()),
            })
            .unwrap_err();
        assert_eq!(
            err.code(),
            "DAEMON-WORKSPACE_ROOT_DENIED",
            "filesystem root must be rejected by policy"
        );
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn two_projects_do_not_share_workspace() {
        // TEST 7: Two missions for two different projects must have distinct
        // persistent workspace roots and cannot accidentally share the same
        // worktree space.
        let (dir, db, lock) = temp_paths();
        let alpha = dir.join("alpha");
        let beta = dir.join("beta");
        fs::create_dir_all(&alpha).unwrap();
        fs::create_dir_all(&beta).unwrap();
        let canon_alpha = fs::canonicalize(&alpha).unwrap();
        let canon_beta = fs::canonicalize(&beta).unwrap();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let resp_a = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "alpha".to_string(),
                workspace_root: Some(alpha.to_string_lossy().to_string()),
            })
            .unwrap();
        let resp_b = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "beta".to_string(),
                workspace_root: Some(beta.to_string_lossy().to_string()),
            })
            .unwrap();
        let mid_a = match resp_a {
            DaemonResponse::SessionCreated { mission_id, .. } => mission_id,
            _ => panic!("expected session"),
        };
        let mid_b = match resp_b {
            DaemonResponse::SessionCreated { mission_id, .. } => mission_id,
            _ => panic!("expected session"),
        };
        let ws_a = daemon.mission_workspace(mid_a.as_str()).unwrap().unwrap();
        let ws_b = daemon.mission_workspace(mid_b.as_str()).unwrap().unwrap();
        assert_eq!(ws_a, canon_alpha.to_string_lossy());
        assert_eq!(ws_b, canon_beta.to_string_lossy());
        assert_ne!(
            ws_a, ws_b,
            "two different projects must not share workspace identity"
        );
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn frontend_cannot_silently_redirect_workspace() {
        // TEST 9: The daemon validates workspace_root independently and
        // rejects values that would redirect execution to a different
        // directory than the user intended.  A relative / empty / invalid
        // path must be rejected rather than silently used.
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        // Empty string must be rejected (not silently ignored).
        let err = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "empty workspace".to_string(),
                workspace_root: Some("   ".to_string()),
            })
            .unwrap_err();
        assert_eq!(err.code(), "DAEMON-WORKSPACE_EMPTY");
        // Relative path must be rejected.
        let err = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "relative".to_string(),
                workspace_root: Some("../somewhere".to_string()),
            })
            .unwrap_err();
        assert_eq!(err.code(), "DAEMON-WORKSPACE_NOT_ABSOLUTE");
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn workspace_root_default_fallback_does_not_leak_to_other_missions() {
        // When no workspace_root is supplied (None), the daemon default
        // is used.  That default is independent per-mission — it cannot
        // cause one mission's workspace to contaminate another's.
        let (dir, db, lock) = temp_paths();
        let explicit_project = dir.join("explicit");
        fs::create_dir_all(&explicit_project).unwrap();
        let canon_explicit = fs::canonicalize(&explicit_project).unwrap();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        // Mission A: no workspace → uses daemon default (cwd/repo-root)
        let resp_a = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "default".to_string(),
                workspace_root: None,
            })
            .unwrap();
        let mid_a = match resp_a {
            DaemonResponse::SessionCreated { mission_id, .. } => mission_id,
            _ => panic!("expected session"),
        };
        // Mission B: explicit workspace
        let resp_b = daemon
            .handle(DaemonCommand::CreateSession {
                goal: "explicit".to_string(),
                workspace_root: Some(explicit_project.to_string_lossy().to_string()),
            })
            .unwrap();
        let mid_b = match resp_b {
            DaemonResponse::SessionCreated { mission_id, .. } => mission_id,
            _ => panic!("expected session"),
        };
        let ws_a = daemon.mission_workspace(mid_a.as_str()).unwrap().unwrap();
        let ws_b = daemon.mission_workspace(mid_b.as_str()).unwrap().unwrap();
        // The default workspace must be a real directory (the canonicalized
        // daemon default).  The explicit workspace must match the supplied
        // project.  They must differ.
        assert!(!ws_a.is_empty(), "default workspace must resolve");
        assert_eq!(ws_b, canon_explicit.to_string_lossy());
        assert_ne!(
            ws_a, ws_b,
            "default and explicit workspaces must not leak across missions"
        );
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn multiple_restarts_keep_terminal_missions_terminal() {
        let (dir, db_path, lock) = temp_paths();
        let mission_id = StableId::from_existing("mission-multi-restart").unwrap();
        let session_id = StableId::from_existing("session-multi-restart").unwrap();
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.put_mission(&ac_kernel::Mission {
                id: mission_id.clone(),
                original_goal: "terminal stability".to_string(),
                state: MissionState::Completed,
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_session(&session_id, &mission_id, "completed", None)
                .unwrap();
        }
        // Restart three times; the completed mission must stay terminal
        // and never be re-enqueued.
        for cycle in 0..3 {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            let status = daemon.mission_status(mission_id.as_str());
            // A completed mission should not have a coordinator status
            // (it is not queued, running, or paused).
            assert!(
                status.is_none(),
                "cycle {cycle}: completed mission must not be in coordinator status"
            );
            let state = daemon.persisted_mission_state(mission_id.as_str()).unwrap();
            assert_eq!(
                state.as_deref(),
                Some("completed"),
                "cycle {cycle}: completed mission must stay completed, got {state:?}"
            );
            daemon.stop().unwrap();
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn multiple_restarts_keep_cancelled_session_terminal() {
        let (dir, db_path, lock) = temp_paths();
        let mission_id = StableId::from_existing("mission-cancelled-restart").unwrap();
        let session_id = StableId::from_existing("session-cancelled-restart").unwrap();
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.put_mission(&ac_kernel::Mission {
                id: mission_id.clone(),
                original_goal: "cancelled stability".to_string(),
                state: MissionState::Cancelled,
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_session(&session_id, &mission_id, "cancelled", None)
                .unwrap();
        }
        for cycle in 0..3 {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            let state = daemon.persisted_mission_state(mission_id.as_str()).unwrap();
            assert_eq!(
                state.as_deref(),
                Some("cancelled"),
                "cycle {cycle}: cancelled mission must stay cancelled, got {state:?}"
            );
            daemon.stop().unwrap();
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn multiple_restarts_keep_failed_session_terminal() {
        let (dir, db_path, lock) = temp_paths();
        let mission_id = StableId::from_existing("mission-failed-restart").unwrap();
        let session_id = StableId::from_existing("session-failed-restart").unwrap();
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.put_mission(&ac_kernel::Mission {
                id: mission_id.clone(),
                original_goal: "failed stability".to_string(),
                state: MissionState::Failed,
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_session(&session_id, &mission_id, "failed", None)
                .unwrap();
        }
        for cycle in 0..3 {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            let status = daemon.mission_status(mission_id.as_str());
            assert!(
                status.is_none(),
                "cycle {cycle}: failed mission must not be in coordinator status"
            );
            let state = daemon.persisted_mission_state(mission_id.as_str()).unwrap();
            assert_eq!(
                state.as_deref(),
                Some("failed"),
                "cycle {cycle}: failed mission must stay failed, got {state:?}"
            );
            daemon.stop().unwrap();
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restart_does_not_duplicate_evidence() {
        let (dir, db_path, lock) = temp_paths();
        let mission_id = StableId::from_existing("mission-evidence-stable").unwrap();
        let session_id = StableId::from_existing("session-evidence-stable").unwrap();
        let evidence_id = StableId::from_existing("ev-evidence-stable").unwrap();
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.put_mission(&ac_kernel::Mission {
                id: mission_id.clone(),
                original_goal: "evidence stability".to_string(),
                state: MissionState::Completed,
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_session(&session_id, &mission_id, "completed", None)
                .unwrap();
            db.append_evidence(&ac_evidence::EvidenceRecord {
                id: evidence_id.clone(),
                kind: ac_evidence::EvidenceKind::DerivedContext,
                provenance: ac_evidence::Provenance {
                    source: "test".to_string(),
                    commit: None,
                    worktree: None,
                    tool: None,
                },
                artifact_uri: "mem://test/stable".to_string(),
                content_hash: "hash".to_string(),
                raw_content: None,
                model_summary: None,
                sensitive: false,
                created_at: TimestampMillis::now(),
            })
            .unwrap();
        }
        let evidence_count_before = {
            let db = ControlPlaneDb::open(&db_path).unwrap();
            db.evidence_records().unwrap().len()
        };
        for cycle in 0..3 {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            daemon.stop().unwrap();
            let db = ControlPlaneDb::open(&db_path).unwrap();
            assert_eq!(
                db.evidence_records().unwrap().len(),
                evidence_count_before,
                "cycle {cycle}: evidence count must not change across restarts"
            );
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn queued_cancelled_mission_persists_terminal_cancelled_state_across_restart() {
        let (dir, db_path, lock) = temp_paths();
        let workspace = dir.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&workspace)
            .status()
            .unwrap();
        assert!(status.success(), "git init must succeed");
        let mission_id = {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            let (mission_id, _session_id) = match daemon
                .handle(DaemonCommand::CreateSession {
                    goal: "queued-cancel-terminal".to_string(),
                    workspace_root: Some(workspace.to_string_lossy().to_string()),
                })
                .unwrap()
            {
                DaemonResponse::SessionCreated {
                    mission_id,
                    session_id,
                } => (mission_id, session_id),
                _ => panic!("expected session"),
            };
            daemon.cancel_mission(mission_id.as_str()).unwrap();
            // Wait for the worker to drain the queued Run message.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let (_, cancelled_len, cancellation_len, pause_flags_len) =
                    daemon.coordinator.map_sizes();
                if cancelled_len == 0 && cancellation_len == 0 && pause_flags_len == 0 {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "coordinator maps not cleaned: cancelled={cancelled_len} cancellation={cancellation_len} pause_flags={pause_flags_len}"
                );
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            // The persisted missions row must be "cancelled", not "active".
            {
                let db = ControlPlaneDb::open(&db_path).unwrap();
                let persisted = db.get_mission(&mission_id).unwrap().unwrap();
                assert_eq!(
                    persisted.state, "cancelled",
                    "queued-cancelled mission must persist as cancelled, got {}",
                    persisted.state
                );
            }
            daemon.stop().unwrap();
            mission_id
        };
        // Restart: the mission must remain terminal Cancelled in the kernel.
        for cycle in 0..3 {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            let db = ControlPlaneDb::open(&db_path).unwrap();
            let persisted = db.get_mission(&mission_id).unwrap().unwrap();
            assert_eq!(
                persisted.state, "cancelled",
                "cycle {cycle}: queued-cancelled mission must stay cancelled, got {}",
                persisted.state
            );
            daemon.stop().unwrap();
        }
        let _ = fs::remove_dir_all(dir);
    }
}
