#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerRole {
    Planner,
    Worker,
    Researcher,
    Verifier,
}

impl WorkerRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planner => "PLANNER",
            Self::Worker => "WORKER",
            Self::Researcher => "RESEARCHER",
            Self::Verifier => "VERIFIER",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredWorker {
    pub id: StableId,
    pub role: WorkerRole,
    pub state: WorkerState,
    pub active_task: Option<StableId>,
    pub lease_epoch: u64,
    pub last_heartbeat: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskLease {
    pub task_id: StableId,
    pub worker_id: StableId,
    pub lease_epoch: u64,
    pub expires_at: TimestampMillis,
    pub heartbeat_interval_ms: u64,
    pub state: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressKind {
    CheckpointCreated,
    TestImproved,
    RequirementEvidenceAdded,
    SubtaskCompleted,
    MeaningfulDiff,
    ToolActivity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressEvent {
    pub id: StableId,
    pub task_id: StableId,
    pub worker_id: StableId,
    pub kind: ProgressKind,
    pub fingerprint: String,
    pub evidence_ref: Option<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureClass {
    ProviderFailure,
    ContextExhaustion,
    WorkerCrash,
    ToolCrash,
    LogicFailure,
    Stall,
    Loop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    SwitchProvider,
    CompactContext,
    ReplaceWorker,
    RestartTool,
    Replan,
    EscalateHuman,
}

impl RecoveryAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SwitchProvider => "SWITCH_PROVIDER",
            Self::CompactContext => "COMPACT_CONTEXT",
            Self::ReplaceWorker => "REPLACE_WORKER",
            Self::RestartTool => "RESTART_TOOL",
            Self::Replan => "REPLAN",
            Self::EscalateHuman => "ESCALATE_HUMAN",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryDecision {
    pub id: StableId,
    pub task_id: StableId,
    pub failure_reason: String,
    pub strategy_change: String,
    pub model_or_provider: Option<String>,
    pub context_change: Option<String>,
    pub attempt: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResearchResult {
    pub id: StableId,
    pub question: String,
    pub sources: Vec<String>,
    pub findings: Vec<String>,
    pub recommendation: String,
    pub risks: Vec<String>,
    pub version_date: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierAssignment {
    pub id: StableId,
    pub task_id: StableId,
    pub requirements: Vec<StableId>,
    pub diff_ref: Option<StableId>,
    pub test_evidence_refs: Vec<StableId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxMessageType {
    Blocker,
    ResearchResult,
    Finding,
    TaskRequest,
    ReplanRequest,
}

impl MailboxMessageType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocker => "BLOCKER",
            Self::ResearchResult => "RESEARCH_RESULT",
            Self::Finding => "FINDING",
            Self::TaskRequest => "TASK_REQUEST",
            Self::ReplanRequest => "REPLAN_REQUEST",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailboxMessage {
    pub id: StableId,
    pub mission_id: StableId,
    pub sender_worker_id: StableId,
    pub recipient_worker_id: Option<StableId>,
    pub message_type: MailboxMessageType,
    pub subject_id: Option<StableId>,
    pub payload: String,
    pub delivered: bool,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlackboardEntry {
    pub id: StableId,
    pub mission_id: StableId,
    pub category: String,
    pub subject_id: Option<StableId>,
    pub content: String,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryPressure {
    Normal,
    Pressure,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceSnapshot {
    pub memory_pressure: MemoryPressure,
    pub cpu_busy: bool,
    pub active_builds: u8,
    pub browser_sessions: u8,
    pub lsp_sessions: u8,
    pub local_model_loaded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConflictPrediction {
    pub left_task: StableId,
    pub right_task: StableId,
    pub risk: u8,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchedulerDecision {
    pub task_id: StableId,
    pub worker_id: Option<StableId>,
    pub admitted: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplanRecord {
    pub id: StableId,
    pub mission_id: StableId,
    pub old_plan_id: StableId,
    pub new_plan_id: StableId,
    pub reason: String,
    pub preserved_evidence_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HumanEscalation {
    pub id: StableId,
    pub mission_id: StableId,
    pub reason: String,
    pub subject_id: Option<StableId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissionControlState {
    Running,
    Paused,
    Resuming,
    Cancelled,
}

pub struct AutonomyKernel {
    mission_id: StableId,
    contract: MissionContract,
    plan: RuntimePlan,
    workers: BTreeMap<StableId, RegisteredWorker>,
    leases: BTreeMap<StableId, TaskLease>,
    progress: Vec<ProgressEvent>,
    mailbox: Vec<MailboxMessage>,
    blackboard: Vec<BlackboardEntry>,
    recovery_events: Vec<(FailureClass, RecoveryAction)>,
    retry_decisions: Vec<RetryDecision>,
    research_results: Vec<ResearchResult>,
    verifier_assignments: Vec<VerifierAssignment>,
    replans: Vec<ReplanRecord>,
    escalations: Vec<HumanEscalation>,
    control_state: MissionControlState,
}

impl AutonomyKernel {
    pub fn from_goal(mission_id: StableId, goal: impl Into<String>) -> AcResult<Self> {
        let contract = MissionContract::extract(mission_id.clone(), goal)?;
        let plan = RuntimePlan::from_contract(&contract)?;
        Ok(Self {
            mission_id,
            contract,
            plan,
            workers: BTreeMap::new(),
            leases: BTreeMap::new(),
            progress: Vec::new(),
            mailbox: Vec::new(),
            blackboard: Vec::new(),
            recovery_events: Vec::new(),
            retry_decisions: Vec::new(),
            research_results: Vec::new(),
            verifier_assignments: Vec::new(),
            replans: Vec::new(),
            escalations: Vec::new(),
            control_state: MissionControlState::Running,
        })
    }

    pub fn contract(&self) -> &MissionContract {
        &self.contract
    }

    pub fn plan(&self) -> &RuntimePlan {
        &self.plan
    }

    pub fn register_worker(&mut self, role: WorkerRole) -> StableId {
        let id = StableId::new("worker");
        self.workers.insert(
            id.clone(),
            RegisteredWorker {
                id: id.clone(),
                role,
                state: WorkerState::Created,
                active_task: None,
                lease_epoch: 0,
                last_heartbeat: TimestampMillis::now(),
            },
        );
        id
    }

    pub fn ready_tasks(&self) -> Vec<StableId> {
        let completed = self
            .plan
            .tasks
            .iter()
            .filter_map(|task| (task.state == TaskState::Completed).then_some(task.id.clone()))
            .collect::<BTreeSet<_>>();
        self.plan
            .tasks
            .iter()
            .filter(|task| {
                matches!(task.state, TaskState::Pending | TaskState::Ready)
                    && task
                        .dependencies
                        .iter()
                        .all(|dependency| completed.contains(dependency))
            })
            .map(|task| task.id.clone())
            .collect()
    }

    pub fn schedule(
        &mut self,
        resources: &ResourceSnapshot,
        conflicts: &[ConflictPrediction],
    ) -> Vec<SchedulerDecision> {
        if self.control_state != MissionControlState::Running {
            return Vec::new();
        }
        let mut decisions = Vec::new();
        let mut active_impl_workers = self
            .workers
            .values()
            .filter(|worker| worker.active_task.is_some() && worker.role == WorkerRole::Worker)
            .count();
        for task_id in self.ready_tasks() {
            if conflict_blocks(&task_id, conflicts) {
                decisions.push(SchedulerDecision {
                    task_id,
                    worker_id: None,
                    admitted: false,
                    reason: "conflict risk requires serialization".to_string(),
                });
                continue;
            }
            if !resource_allows(resources, active_impl_workers) {
                decisions.push(SchedulerDecision {
                    task_id,
                    worker_id: None,
                    admitted: false,
                    reason: "resource governor delayed task".to_string(),
                });
                continue;
            }
            let worker = self
                .workers
                .values_mut()
                .find(|worker| worker.active_task.is_none() && worker.role == WorkerRole::Worker);
            if let Some(worker) = worker {
                worker.state = WorkerState::Assigned;
                worker.active_task = Some(task_id.clone());
                active_impl_workers += 1;
                decisions.push(SchedulerDecision {
                    task_id,
                    worker_id: Some(worker.id.clone()),
                    admitted: true,
                    reason: "ready dependencies satisfied".to_string(),
                });
            }
        }
        decisions
    }

    pub fn acquire_lease(
        &mut self,
        task_id: StableId,
        worker_id: StableId,
        lease_ms: u64,
        heartbeat_interval_ms: u64,
    ) -> AcResult<TaskLease> {
        if self
            .leases
            .get(&task_id)
            .is_some_and(|lease| lease.expires_at.as_millis() > TimestampMillis::now().as_millis())
        {
            return Err(AcError::conflict(
                "RUNTIME-LEASE_HELD",
                "task already has a live lease",
            ));
        }
        let epoch = self
            .leases
            .get(&task_id)
            .map(|lease| lease.lease_epoch + 1)
            .unwrap_or(1);
        let lease = TaskLease {
            task_id: task_id.clone(),
            worker_id: worker_id.clone(),
            lease_epoch: epoch,
            expires_at: TimestampMillis::from_millis(
                TimestampMillis::now().as_millis() + u128::from(lease_ms),
            ),
            heartbeat_interval_ms,
            state: "active".to_string(),
        };
        if let Some(worker) = self.workers.get_mut(&worker_id) {
            worker.active_task = Some(task_id.clone());
            worker.state = WorkerState::Running;
            worker.lease_epoch = epoch;
        }
        self.leases.insert(task_id, lease.clone());
        Ok(lease)
    }

    pub fn heartbeat(&mut self, task_id: &StableId, worker_id: &StableId) -> AcResult<()> {
        let lease = self
            .leases
            .get_mut(task_id)
            .ok_or_else(|| AcError::validation("RUNTIME-NO_LEASE", "task has no lease"))?;
        if &lease.worker_id != worker_id {
            return Err(AcError::conflict(
                "RUNTIME-ZOMBIE_WORKER",
                "old lease holder cannot mutate after replacement",
            ));
        }
        lease.expires_at = TimestampMillis::from_millis(
            TimestampMillis::now().as_millis() + u128::from(lease.heartbeat_interval_ms * 3),
        );
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.last_heartbeat = TimestampMillis::now();
        }
        Ok(())
    }

    pub fn reject_zombie_write(
        &self,
        task_id: &StableId,
        worker_id: &StableId,
        lease_epoch: u64,
    ) -> AcResult<()> {
        let lease = self
            .leases
            .get(task_id)
            .ok_or_else(|| AcError::validation("RUNTIME-NO_LEASE", "task has no lease"))?;
        if &lease.worker_id != worker_id || lease.lease_epoch != lease_epoch {
            return Err(AcError::conflict(
                "RUNTIME-ZOMBIE_WORKER",
                "stale worker lease cannot mutate task state",
            ));
        }
        Ok(())
    }

    pub fn recover_expired_leases(&mut self) -> Vec<StableId> {
        let now = TimestampMillis::now().as_millis();
        let expired = self
            .leases
            .values_mut()
            .filter(|lease| lease.expires_at.as_millis() <= now && lease.state == "active")
            .map(|lease| {
                lease.state = "expired".to_string();
                lease.task_id.clone()
            })
            .collect::<Vec<_>>();
        for task_id in &expired {
            self.recovery_events
                .push((FailureClass::WorkerCrash, RecoveryAction::ReplaceWorker));
            self.mailbox.push(MailboxMessage {
                id: StableId::new("msg"),
                mission_id: self.mission_id.clone(),
                sender_worker_id: StableId::from_existing("scheduler").expect("valid"),
                recipient_worker_id: None,
                message_type: MailboxMessageType::TaskRequest,
                subject_id: Some(task_id.clone()),
                payload: "lease expired; replacement worker required".to_string(),
                delivered: false,
                created_at: TimestampMillis::now(),
            });
        }
        expired
    }

    pub fn record_progress(&mut self, event: ProgressEvent) {
        self.progress.push(event);
    }

    pub fn stalled_workers(&self, max_no_progress_ms: u128) -> Vec<StableId> {
        let now = TimestampMillis::now().as_millis();
        self.workers
            .values()
            .filter(|worker| {
                worker.active_task.is_some()
                    && self
                        .progress
                        .iter()
                        .filter(|event| event.worker_id == worker.id && is_meaningful(event.kind))
                        .map(|event| event.created_at.as_millis())
                        .max()
                        .is_none_or(|last| now.saturating_sub(last) > max_no_progress_ms)
            })
            .map(|worker| worker.id.clone())
            .collect()
    }

    pub fn repeated_loop_detected(&self, window: usize) -> bool {
        if self.progress.len() < window || window == 0 {
            return false;
        }
        let tail = &self.progress[self.progress.len() - window..];
        tail.iter()
            .map(|event| &event.fingerprint)
            .collect::<BTreeSet<_>>()
            .len()
            == 1
    }

    pub fn recovery_action(failure: FailureClass) -> RecoveryAction {
        match failure {
            FailureClass::ProviderFailure => RecoveryAction::SwitchProvider,
            FailureClass::ContextExhaustion => RecoveryAction::CompactContext,
            FailureClass::WorkerCrash => RecoveryAction::ReplaceWorker,
            FailureClass::ToolCrash => RecoveryAction::RestartTool,
            FailureClass::LogicFailure | FailureClass::Stall | FailureClass::Loop => {
                RecoveryAction::Replan
            }
        }
    }

    pub fn retry_decision(
        &mut self,
        task_id: StableId,
        failure_reason: impl Into<String>,
        strategy_change: impl Into<String>,
        attempt: u32,
    ) -> AcResult<RetryDecision> {
        let strategy_change = strategy_change.into();
        if strategy_change.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-RETRY_NO_CHANGE",
                "retry must change strategy, model/provider, or context",
            ));
        }
        let decision = RetryDecision {
            id: StableId::new("retry"),
            task_id,
            failure_reason: failure_reason.into(),
            strategy_change,
            model_or_provider: None,
            context_change: None,
            attempt,
        };
        self.retry_decisions.push(decision.clone());
        Ok(decision)
    }

    pub fn record_research(&mut self, result: ResearchResult) {
        self.research_results.push(result);
    }

    pub fn schedule_verifier(&mut self, task_id: StableId) -> AcResult<VerifierAssignment> {
        if !self
            .workers
            .values()
            .any(|worker| worker.role == WorkerRole::Verifier)
        {
            return Err(AcError::validation(
                "RUNTIME-NO_VERIFIER",
                "verifier role is not registered",
            ));
        }
        let assignment = VerifierAssignment {
            id: StableId::new("verify"),
            task_id,
            requirements: self
                .contract
                .requirements
                .iter()
                .map(|req| req.id.clone())
                .collect(),
            diff_ref: None,
            test_evidence_refs: Vec::new(),
        };
        self.verifier_assignments.push(assignment.clone());
        Ok(assignment)
    }

    pub fn send_message(&mut self, message: MailboxMessage) {
        self.mailbox.push(message);
    }

    pub fn mailbox(&self) -> &[MailboxMessage] {
        &self.mailbox
    }

    pub fn add_blackboard_entry(&mut self, entry: BlackboardEntry) {
        self.blackboard.push(entry);
    }

    pub fn relevant_blackboard(&self, subject_id: &StableId) -> Vec<BlackboardEntry> {
        self.blackboard
            .iter()
            .filter(|entry| entry.subject_id.as_ref().is_none_or(|id| id == subject_id))
            .cloned()
            .collect()
    }

    pub fn predict_conflict(
        left: &PlannerTaskProposal,
        right: &PlannerTaskProposal,
    ) -> ConflictPrediction {
        let mut reasons = Vec::new();
        if left
            .target_files
            .iter()
            .any(|file| right.target_files.contains(file))
        {
            reasons.push("same file".to_string());
        }
        if left
            .target_files
            .iter()
            .any(|file| file.ends_with("Cargo.lock") || file.ends_with("package-lock.json"))
            || right
                .target_files
                .iter()
                .any(|file| file.ends_with("Cargo.lock") || file.ends_with("package-lock.json"))
        {
            reasons.push("lockfile".to_string());
        }
        ConflictPrediction {
            left_task: left
                .requirement_ids
                .first()
                .cloned()
                .unwrap_or_else(|| StableId::new("task")),
            right_task: right
                .requirement_ids
                .first()
                .cloned()
                .unwrap_or_else(|| StableId::new("task")),
            risk: if reasons.is_empty() { 10 } else { 90 },
            reasons,
        }
    }

    pub fn pause(&mut self) {
        self.control_state = MissionControlState::Paused;
    }

    pub fn resume(&mut self) {
        self.control_state = MissionControlState::Resuming;
        self.blackboard.push(BlackboardEntry {
            id: StableId::new("bb"),
            mission_id: self.mission_id.clone(),
            category: "resume_reconcile".to_string(),
            subject_id: None,
            content: "refresh git, indexes, provider health, and stale evidence".to_string(),
            created_at: TimestampMillis::now(),
        });
        self.control_state = MissionControlState::Running;
    }

    pub fn cancel(&mut self) {
        self.control_state = MissionControlState::Cancelled;
        for worker in self.workers.values_mut() {
            if !matches!(worker.state, WorkerState::Completed | WorkerState::Failed) {
                worker.state = WorkerState::Cancelled;
                worker.active_task = None;
            }
        }
        for lease in self.leases.values_mut() {
            lease.state = "cancelled".to_string();
        }
    }

    pub fn replan(&mut self, reason: impl Into<String>) -> AcResult<ReplanRecord> {
        let old_plan = self.plan.id.clone();
        let next_contract = self
            .contract
            .add_requirement("dynamic follow-up task", "dynamic task discovery")?;
        let mut next_plan = RuntimePlan::from_contract(&next_contract)?;
        next_plan.supersedes = Some(old_plan.clone());
        let preserved = self
            .contract
            .requirements
            .iter()
            .flat_map(|requirement| requirement.evidence_refs.clone())
            .collect::<Vec<_>>();
        self.contract = next_contract;
        self.plan = next_plan;
        let record = ReplanRecord {
            id: StableId::new("replan"),
            mission_id: self.mission_id.clone(),
            old_plan_id: old_plan,
            new_plan_id: self.plan.id.clone(),
            reason: reason.into(),
            preserved_evidence_refs: preserved,
        };
        self.replans.push(record.clone());
        Ok(record)
    }

    pub fn escalate_human(
        &mut self,
        reason: impl Into<String>,
        subject_id: Option<StableId>,
    ) -> AcResult<HumanEscalation> {
        let reason = reason.into();
        let allowed = [
            "missing credentials",
            "irreversible decision",
            "production action",
            "budget limit",
            "persistent blocker",
        ]
        .iter()
        .any(|allowed| reason.contains(allowed));
        if !allowed {
            return Err(AcError::validation(
                "RUNTIME-ESCALATION_NOT_ALLOWED",
                "human escalation is reserved for explicit high-stakes blockers",
            ));
        }
        let escalation = HumanEscalation {
            id: StableId::new("escalation"),
            mission_id: self.mission_id.clone(),
            reason,
            subject_id,
        };
        self.escalations.push(escalation.clone());
        Ok(escalation)
    }
}
