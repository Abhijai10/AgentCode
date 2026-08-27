use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_db::{ControlPlaneDb, PersistedSession};
use ac_kernel::{Kernel, KernelDecisionKind, MissionState, PermissionDecision, PolicyBoundary};
use ac_runtime::{
    AgentSession, AgentSessionState, CancellationToken, HydratedSession, RuntimeHydrator, Worker,
};

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
}

#[derive(Default)]
struct CoordinatorState {
    queued: VecDeque<QueuedMission>,
    active: Option<StableId>,
    statuses: BTreeMap<String, MissionExecutionStatus>,
    paused: BTreeMap<String, QueuedMission>,
    cancelled: BTreeMap<String, ()>,
    cancellation: BTreeMap<String, CancellationToken>,
}

/// A deliberately single-slot, daemon-owned executor.  The IPC thread only
/// queues durable work; execution happens on this dedicated worker.
struct MissionCoordinator {
    tx: mpsc::SyncSender<QueuedMission>,
    state: Arc<Mutex<CoordinatorState>>,
}

impl MissionCoordinator {
    fn new(
        db_path: PathBuf,
        workspace_root: PathBuf,
        kernel: Arc<Mutex<Kernel<ProductionKernelPolicy>>>,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel::<QueuedMission>(64);
        let state = Arc::new(Mutex::new(CoordinatorState::default()));
        let worker_state = Arc::clone(&state);
        thread::Builder::new()
            .name("agentcode-mission-worker".to_string())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    let run = {
                        let mut state = match worker_state.lock() {
                            Ok(state) => state,
                            Err(_) => continue,
                        };
                        state
                            .queued
                            .retain(|queued| queued.mission_id != job.mission_id);
                        if state.cancelled.contains_key(job.mission_id.as_str()) {
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
                        let token = worker_state
                            .lock()
                            .ok()
                            .and_then(|state| {
                                state.cancellation.get(job.mission_id.as_str()).cloned()
                            })
                            .unwrap_or_default();
                        execute_mission(&db_path, &workspace_root, Arc::clone(&kernel), &job, token)
                    }));
                    let terminal = match outcome {
                        Ok(Ok(state)) => state,
                        Ok(Err(error)) => format!("failed: {}", error.code()),
                        Err(_) => "failed: DAEMON-MISSION_PANIC".to_string(),
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
                    if let Ok(mut state) = worker_state.lock() {
                        state.active = None;
                        let cancelled = state.cancelled.contains_key(job.mission_id.as_str());
                        if let Some(status) = state.statuses.get_mut(job.mission_id.as_str()) {
                            status.state = if cancelled {
                                "cancelled".to_string()
                            } else {
                                terminal
                            };
                        }
                    }
                }
            })
            .expect("mission coordinator thread must start");
        Self { tx, state }
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
            state.queued.push_back(job.clone());
        }
        self.tx.try_send(job).map_err(|_| {
            AcError::new(
                "DAEMON-QUEUE_FULL",
                "mission queue is full",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::Retryable,
            )
        })
    }

    fn status(&self, mission_id: &str) -> Option<MissionExecutionStatus> {
        self.state.lock().ok()?.statuses.get(mission_id).cloned()
    }

    fn pause(&self, mission_id: &str) -> AcResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
        })?;
        let job = state
            .queued
            .iter()
            .find(|job| job.mission_id.as_str() == mission_id)
            .cloned()
            .ok_or_else(|| {
                AcError::conflict(
                    "DAEMON-MISSION_NOT_PAUSABLE",
                    "mission is already running or terminal",
                )
            })?;
        state.paused.insert(mission_id.to_string(), job);
        if let Some(status) = state.statuses.get_mut(mission_id) {
            status.state = "paused".to_string();
        }
        Ok(())
    }

    fn resume(&self, mission_id: &str) -> AcResult<()> {
        let job = {
            let mut state = self.state.lock().map_err(|_| {
                AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
            })?;
            let job = state.paused.remove(mission_id).ok_or_else(|| {
                AcError::conflict("DAEMON-MISSION_NOT_PAUSED", "mission is not paused")
            })?;
            state.queued.push_back(job.clone());
            if let Some(status) = state.statuses.get_mut(mission_id) {
                status.state = "queued".to_string();
            }
            job
        };
        self.tx.try_send(job).map_err(|_| {
            AcError::new(
                "DAEMON-QUEUE_FULL",
                "mission queue is full",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::Retryable,
            )
        })
    }

    fn cancel(&self, mission_id: &str) -> AcResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            AcError::conflict("DAEMON-COORDINATOR_POISONED", "coordinator lock poisoned")
        })?;
        if !state.statuses.contains_key(mission_id) {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission is not known to coordinator",
            ));
        }
        if let Some(token) = state.cancellation.get(mission_id) {
            token.cancel();
        }
        state.cancelled.insert(mission_id.to_string(), ());
        state.paused.remove(mission_id);
        state
            .queued
            .retain(|job| job.mission_id.as_str() != mission_id);
        if let Some(status) = state.statuses.get_mut(mission_id) {
            status.state = "cancelled".to_string();
        }
        Ok(())
    }
}

fn execute_mission(
    db_path: &Path,
    workspace_root: &Path,
    kernel: Arc<Mutex<Kernel<ProductionKernelPolicy>>>,
    job: &QueuedMission,
    cancellation: CancellationToken,
) -> AcResult<String> {
    let db = ControlPlaneDb::open(db_path)?;
    db.update_session_state(&job.session_id, "running")?;
    let session =
        AgentSession::with_id_and_token(job.session_id.clone(), Worker::new(), cancellation);
    let worktree = workspace_root
        .join(".agentcode-worktrees")
        .join(job.session_id.as_str());
    let mut agent = ac_agent::bound_workspace_agent(
        workspace_root.to_path_buf(),
        worktree,
        Arc::clone(&kernel),
        job.mission_id.clone(),
        session,
        backend_tool_policy(),
    )?;
    let report = agent.run_goal(ac_agent::Goal::new(job.goal.clone())?)?;
    let evidence = agent.into_evidence();
    for record in evidence.records() {
        db.append_evidence(record)?;
    }
    {
        let kernel = kernel.lock().map_err(|_| {
            AcError::conflict(
                "DAEMON-KERNEL_POISONED",
                "kernel lock poisoned after mission",
            )
        })?;
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

fn backend_tool_policy() -> ac_security::CapabilityPolicy {
    ac_security::CapabilityPolicy::new()
        .allow(ac_security::Capability::FilesystemRead("*".to_string()))
        .allow(ac_security::Capability::FilesystemWrite("*".to_string()))
        .allow(ac_security::Capability::ProcessExec("*".to_string()))
        .allow(ac_security::Capability::BrowserAutomation)
        .allow(ac_security::Capability::SecurityScan)
}

include!("release.rs");
include!("ipc.rs");

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
    kernel: Arc<Mutex<Kernel<ProductionKernelPolicy>>>,
    lock_path: PathBuf,
    lock_file: Option<File>,
    instance_id: StableId,
    recovered: Vec<PersistedSession>,
    hydrated: Vec<HydratedSession>,
    coordinator: MissionCoordinator,
}

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
        let kernel = Arc::new(Mutex::new(Kernel::new(ProductionKernelPolicy)));
        let workspace_root = std::env::var_os("AGENTCODE_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or(
                std::env::current_dir()
                    .map_err(|error| AcError::validation("DAEMON-WORKSPACE", error.to_string()))?,
            );
        Ok(Self {
            lifecycle: DaemonLifecycle::Created,
            db,
            coordinator: MissionCoordinator::new(
                db_path,
                workspace_root.clone(),
                Arc::clone(&kernel),
            ),
            kernel,
            lock_path: lock_path.into(),
            lock_file: None,
            instance_id: StableId::new("daemon"),
            recovered: Vec::new(),
            hydrated: Vec::new(),
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
                if !matches!(
                    hydrated.session.state.as_str(),
                    "paused" | "completed" | "cancelled" | "failed"
                ) {
                    self.coordinator.enqueue(QueuedMission {
                        mission_id,
                        session_id: StableId::from_existing(&hydrated.session.id)?,
                        goal: mission.original_goal,
                    })?;
                }
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
        Ok(self.db.get_mission(&id)?.map(|mission| mission.state))
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
            self.db.update_session_state(&status.session_id, "queued")?;
        }
        Ok(())
    }

    pub fn cancel_mission(&mut self, mission_id: &str) -> AcResult<()> {
        self.ensure_running()?;
        self.coordinator.cancel(mission_id)?;
        if let Some(status) = self.coordinator.status(mission_id) {
            self.db
                .update_session_state(&status.session_id, "cancelled")?;
        }
        Ok(())
    }

    pub fn handle(&mut self, command: DaemonCommand) -> AcResult<DaemonResponse> {
        match command {
            DaemonCommand::Health => Ok(DaemonResponse::Health(self.health())),
            DaemonCommand::CreateSession { goal } => {
                self.ensure_running()?;
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
                self.db
                    .save_session(session.id(), &mission_id, session_state(session.state()))?;
                self.coordinator.enqueue(QueuedMission {
                    mission_id: mission_id.clone(),
                    session_id: session.id().clone(),
                    goal,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp_paths() -> (PathBuf, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("agentcode-daemon-{}", StableId::new("tmp")));
        fs::create_dir_all(&dir).unwrap();
        let (db, lock) = default_paths(&dir);
        (dir, db, lock)
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
            db.save_session(&session_id, &mission_id, "executing")
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
            )
            .unwrap();
            db.save_session(
                &StableId::from_existing("session-restart-paused").unwrap(),
                &paused_mission,
                "paused",
            )
            .unwrap();
            db.save_session(
                &StableId::from_existing("session-restart-completed").unwrap(),
                &completed_mission,
                "completed",
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
        assert!(restarted.mission_status(paused_mission.as_str()).is_none());
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
}
