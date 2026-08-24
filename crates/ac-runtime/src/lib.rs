use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_context::{ContextNode, ContextPack};
use ac_db::{ControlPlaneDb, TaskAttemptRecord, TaskRecord, WorkerRecord};
use ac_git::CheckpointRecord;
use ac_provider::{NormalizedInferenceRequest, ProviderStreamEvent};
use ac_tool::{ToolRequest, ToolResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementKind {
    Functional,
    Verification,
    Safety,
    Operational,
}

impl RequirementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Functional => "FUNCTIONAL",
            Self::Verification => "VERIFICATION",
            Self::Safety => "SAFETY",
            Self::Operational => "OPERATIONAL",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementStatus {
    Open,
    Implemented,
    Verified,
    Blocked,
    Superseded,
}

impl RequirementStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "OPEN",
            Self::Implemented => "IMPLEMENTED",
            Self::Verified => "VERIFIED",
            Self::Blocked => "BLOCKED",
            Self::Superseded => "SUPERSEDED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionRequirement {
    pub id: StableId,
    pub description: String,
    pub kind: RequirementKind,
    pub priority: u8,
    pub source: String,
    pub verification_strategy: String,
    pub blocking: bool,
    pub implementation_status: RequirementStatus,
    pub verification_status: RequirementStatus,
    pub evidence_refs: Vec<StableId>,
    pub linked_task_ids: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionContract {
    pub id: StableId,
    pub mission_id: StableId,
    pub revision: u32,
    pub original_goal: String,
    pub reason: String,
    pub requirements: Vec<MissionRequirement>,
    pub created_at: TimestampMillis,
}

impl MissionContract {
    pub fn extract(mission_id: StableId, original_goal: impl Into<String>) -> AcResult<Self> {
        let original_goal = original_goal.into();
        if original_goal.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-EMPTY_CONTRACT_GOAL",
                "mission contract requires the immutable original goal",
            ));
        }
        let mut requirements = Vec::new();
        for (index, part) in original_goal
            .split(|ch| ['\n', ';', '.'].contains(&ch))
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .enumerate()
        {
            requirements.push(MissionRequirement {
                id: StableId::new("req"),
                description: part.to_string(),
                kind: if part.contains("test") || part.contains("verify") {
                    RequirementKind::Verification
                } else {
                    RequirementKind::Functional
                },
                priority: (100_u8).saturating_sub(index as u8),
                source: "original_goal".to_string(),
                verification_strategy: if part.contains("test") {
                    "run specified tests".to_string()
                } else {
                    "evidence-backed verifier review".to_string()
                },
                blocking: true,
                implementation_status: RequirementStatus::Open,
                verification_status: RequirementStatus::Open,
                evidence_refs: Vec::new(),
                linked_task_ids: Vec::new(),
            });
        }
        if requirements.is_empty() {
            requirements.push(MissionRequirement {
                id: StableId::new("req"),
                description: original_goal.clone(),
                kind: RequirementKind::Functional,
                priority: 100,
                source: "original_goal".to_string(),
                verification_strategy: "evidence-backed verifier review".to_string(),
                blocking: true,
                implementation_status: RequirementStatus::Open,
                verification_status: RequirementStatus::Open,
                evidence_refs: Vec::new(),
                linked_task_ids: Vec::new(),
            });
        }
        Ok(Self {
            id: StableId::new("contract"),
            mission_id,
            revision: 1,
            original_goal,
            reason: "initial extraction".to_string(),
            requirements,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn add_requirement(
        &self,
        description: impl Into<String>,
        reason: impl Into<String>,
    ) -> AcResult<Self> {
        let description = description.into();
        if description.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-EMPTY_REQUIREMENT",
                "new requirements need a description",
            ));
        }
        let mut next = self.clone();
        next.id = StableId::new("contract");
        next.revision += 1;
        next.reason = reason.into();
        next.created_at = TimestampMillis::now();
        next.requirements.push(MissionRequirement {
            id: StableId::new("req"),
            description,
            kind: RequirementKind::Functional,
            priority: 80,
            source: format!("contract_revision:{}", next.revision),
            verification_strategy: "evidence-backed verifier review".to_string(),
            blocking: true,
            implementation_status: RequirementStatus::Open,
            verification_status: RequirementStatus::Open,
            evidence_refs: Vec::new(),
            linked_task_ids: Vec::new(),
        });
        Ok(next)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeTaskType {
    Implementation,
    Research,
    Verification,
    Repair,
}

impl RuntimeTaskType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implementation => "IMPLEMENTATION",
            Self::Research => "RESEARCH",
            Self::Verification => "VERIFICATION",
            Self::Repair => "REPAIR",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RuntimeTaskRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannerTaskProposal {
    pub proposal_id: String,
    pub title: String,
    pub description: String,
    pub dependencies: Vec<StableId>,
    pub risk: RuntimeTaskRisk,
    pub task_type: RuntimeTaskType,
    pub acceptance_criteria: Vec<String>,
    pub skills: Vec<String>,
    pub context_profile: String,
    pub target_files: Vec<String>,
    pub requirement_ids: Vec<StableId>,
    pub priority: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePlan {
    pub id: StableId,
    pub mission_id: StableId,
    pub revision: u32,
    pub tasks: Vec<WorkerTask>,
    pub proposals: Vec<PlannerTaskProposal>,
    pub supersedes: Option<StableId>,
}

impl RuntimePlan {
    pub fn from_contract(contract: &MissionContract) -> AcResult<Self> {
        let proposals = contract
            .requirements
            .iter()
            .map(|requirement| PlannerTaskProposal {
                proposal_id: format!("proposal-{}", requirement.id),
                title: requirement.description.clone(),
                description: requirement.description.clone(),
                dependencies: Vec::new(),
                risk: if requirement.blocking {
                    RuntimeTaskRisk::High
                } else {
                    RuntimeTaskRisk::Medium
                },
                task_type: match requirement.kind {
                    RequirementKind::Verification => RuntimeTaskType::Verification,
                    _ => RuntimeTaskType::Implementation,
                },
                acceptance_criteria: vec![requirement.verification_strategy.clone()],
                skills: Vec::new(),
                context_profile: "NORMAL".to_string(),
                target_files: Vec::new(),
                requirement_ids: vec![requirement.id.clone()],
                priority: requirement.priority,
            })
            .collect::<Vec<_>>();
        let tasks = proposals
            .iter()
            .map(|proposal| WorkerTask {
                id: StableId::new("task"),
                mission_id: contract.mission_id.clone(),
                title: proposal.title.clone(),
                dependencies: proposal.dependencies.clone(),
                state: TaskState::Pending,
                assigned_worker: None,
                retry_count: 0,
                max_retries: 3,
                evidence_refs: Vec::new(),
            })
            .collect::<Vec<_>>();
        let plan = Self {
            id: StableId::new("plan"),
            mission_id: contract.mission_id.clone(),
            revision: contract.revision,
            tasks,
            proposals,
            supersedes: None,
        };
        validate_plan_dag(&plan)?;
        Ok(plan)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentSessionState {
    Created,
    Running,
    Cancelling,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEventKind {
    SessionCreated,
    SessionStarted,
    WorkItemProcessed(String),
    ModelEvent(ProviderStreamEvent),
    ToolResult(StableId),
    ContextPackBuilt(StableId),
    CheckpointCreated(StableId),
    StepFailed(String),
    CancellationRequested,
    SessionStopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEvent {
    pub id: StableId,
    pub session_id: StableId,
    pub kind: RuntimeEventKind,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct Worker {
    pub id: StableId,
    pub workspace_ref: Option<StableId>,
    pub mission_id: Option<StableId>,
    pub state: WorkerState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Created,
    Assigned,
    Running,
    Checkpointed,
    Cancelled,
    Completed,
    Failed,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            id: StableId::new("worker"),
            workspace_ref: None,
            mission_id: None,
            state: WorkerState::Created,
        }
    }

    pub fn assigned_to(workspace_ref: StableId) -> Self {
        Self {
            id: StableId::new("worker"),
            workspace_ref: Some(workspace_ref),
            mission_id: None,
            state: WorkerState::Created,
        }
    }
}

impl Worker {
    pub fn assign(&mut self, mission_id: StableId) -> AcResult<()> {
        if self.state != WorkerState::Created {
            return Err(AcError::conflict(
                "RUNTIME-WORKER_ALREADY_ASSIGNED",
                "worker may only be assigned once",
            ));
        }
        self.mission_id = Some(mission_id);
        self.state = WorkerState::Assigned;
        Ok(())
    }

    pub fn transition(&mut self, next: WorkerState) -> AcResult<()> {
        let valid = matches!(
            (self.state, next),
            (WorkerState::Assigned, WorkerState::Running)
                | (WorkerState::Running, WorkerState::Checkpointed)
                | (WorkerState::Checkpointed, WorkerState::Running)
                | (WorkerState::Running, WorkerState::Cancelled)
                | (WorkerState::Running, WorkerState::Completed)
                | (WorkerState::Running, WorkerState::Failed)
        );
        if !valid {
            return Err(AcError::conflict(
                "RUNTIME-WORKER_INVALID_STATE",
                "invalid worker lifecycle transition",
            ));
        }
        self.state = next;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskState {
    Pending,
    Ready,
    Running,
    Retryable,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerTask {
    pub id: StableId,
    pub mission_id: StableId,
    pub title: String,
    pub dependencies: Vec<StableId>,
    pub state: TaskState,
    pub assigned_worker: Option<StableId>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub evidence_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskAttempt {
    pub id: StableId,
    pub task_id: StableId,
    pub worker_id: StableId,
    pub outcome: TaskAttemptOutcome,
    pub evidence_refs: Vec<StableId>,
    pub failure_class: Option<String>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskAttemptOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Default)]
pub struct TaskGraph {
    tasks: BTreeMap<StableId, WorkerTask>,
    attempts: Vec<TaskAttempt>,
}

impl TaskGraph {
    pub fn decompose(mission_id: StableId, goal: &str) -> AcResult<Self> {
        if goal.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-TASK_EMPTY_GOAL",
                "task decomposition requires a goal",
            ));
        }
        let mut graph = Self::default();
        let analyze = graph.add_task(mission_id.clone(), "analyze repository", Vec::new(), 1);
        let identify = graph.add_task(mission_id.clone(), "identify failure", vec![analyze], 1);
        let modify = graph.add_task(mission_id.clone(), "modify code", vec![identify], 2);
        let test = graph.add_task(mission_id.clone(), "run verification", vec![modify], 2);
        graph.add_task(mission_id, "request completion", vec![test], 0);
        graph.refresh_ready();
        Ok(graph)
    }

    pub fn tasks(&self) -> impl Iterator<Item = &WorkerTask> {
        self.tasks.values()
    }
    pub fn attempts(&self) -> &[TaskAttempt] {
        &self.attempts
    }

    pub fn next_ready(&self) -> Option<&WorkerTask> {
        self.tasks
            .values()
            .find(|task| task.state == TaskState::Ready)
    }

    pub fn start(&mut self, task_id: &StableId, worker: &Worker) -> AcResult<()> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| AcError::validation("RUNTIME-TASK_UNKNOWN", "task is not in graph"))?;
        if task.state != TaskState::Ready {
            return Err(AcError::conflict(
                "RUNTIME-TASK_NOT_READY",
                "task dependencies are not complete",
            ));
        }
        if worker.state != WorkerState::Running {
            return Err(AcError::conflict(
                "RUNTIME-WORKER_NOT_RUNNING",
                "task requires a running worker",
            ));
        }
        task.state = TaskState::Running;
        task.assigned_worker = Some(worker.id.clone());
        Ok(())
    }

    pub fn finish(
        &mut self,
        task_id: &StableId,
        worker: &Worker,
        outcome: TaskAttemptOutcome,
        evidence_refs: Vec<StableId>,
        failure_class: Option<String>,
    ) -> AcResult<()> {
        let task = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| AcError::validation("RUNTIME-TASK_UNKNOWN", "task is not in graph"))?;
        if task.state != TaskState::Running || task.assigned_worker.as_ref() != Some(&worker.id) {
            return Err(AcError::conflict(
                "RUNTIME-TASK_OWNERSHIP",
                "only assigned worker may finish task",
            ));
        }
        task.evidence_refs.extend(evidence_refs.iter().cloned());
        let attempt = TaskAttempt {
            id: StableId::new("attempt"),
            task_id: task.id.clone(),
            worker_id: worker.id.clone(),
            outcome,
            evidence_refs,
            failure_class,
            created_at: TimestampMillis::now(),
        };
        match outcome {
            TaskAttemptOutcome::Succeeded => task.state = TaskState::Completed,
            TaskAttemptOutcome::Cancelled => task.state = TaskState::Cancelled,
            TaskAttemptOutcome::Failed if task.retry_count < task.max_retries => {
                task.retry_count += 1;
                task.state = TaskState::Retryable;
            }
            TaskAttemptOutcome::Failed => task.state = TaskState::Failed,
        }
        self.attempts.push(attempt);
        self.refresh_ready();
        Ok(())
    }

    pub fn persist(
        &self,
        db: &ControlPlaneDb,
        worker: &Worker,
        session_id: &StableId,
    ) -> AcResult<()> {
        let mission = worker.mission_id.as_ref().ok_or_else(|| {
            AcError::validation(
                "RUNTIME-WORKER_NO_MISSION",
                "worker is not assigned to a mission",
            )
        })?;
        db.save_worker(&WorkerRecord {
            id: worker.id.to_string(),
            mission_id: mission.to_string(),
            session_id: session_id.to_string(),
            state: format!("{:?}", worker.state).to_lowercase(),
            workspace_ref: worker.workspace_ref.as_ref().map(ToString::to_string),
            updated_at_ms: TimestampMillis::now().as_millis() as i64,
        })?;
        for task in self.tasks.values() {
            db.save_task(&TaskRecord {
                id: task.id.to_string(),
                mission_id: task.mission_id.to_string(),
                title: task.title.clone(),
                state: format!("{:?}", task.state).to_lowercase(),
                dependencies_json: task
                    .dependencies
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                assigned_worker_id: task.assigned_worker.as_ref().map(ToString::to_string),
                retry_count: task.retry_count,
                max_retries: task.max_retries,
                updated_at_ms: TimestampMillis::now().as_millis() as i64,
            })?;
        }
        for attempt in &self.attempts {
            db.save_task_attempt(&TaskAttemptRecord {
                id: attempt.id.to_string(),
                task_id: attempt.task_id.to_string(),
                worker_id: attempt.worker_id.to_string(),
                outcome: format!("{:?}", attempt.outcome).to_lowercase(),
                evidence_refs: attempt
                    .evidence_refs
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                failure_class: attempt.failure_class.clone(),
                created_at_ms: attempt.created_at.as_millis() as i64,
            })?;
        }
        Ok(())
    }

    fn add_task(
        &mut self,
        mission_id: StableId,
        title: &str,
        dependencies: Vec<StableId>,
        max_retries: u32,
    ) -> StableId {
        let id = StableId::new("task");
        self.tasks.insert(
            id.clone(),
            WorkerTask {
                id: id.clone(),
                mission_id,
                title: title.to_string(),
                dependencies,
                state: TaskState::Pending,
                assigned_worker: None,
                retry_count: 0,
                max_retries,
                evidence_refs: Vec::new(),
            },
        );
        id
    }

    fn refresh_ready(&mut self) {
        let completed = self
            .tasks
            .iter()
            .filter_map(|(id, task)| (task.state == TaskState::Completed).then_some(id.clone()))
            .collect::<Vec<_>>();
        for task in self.tasks.values_mut() {
            if matches!(task.state, TaskState::Pending | TaskState::Retryable)
                && task
                    .dependencies
                    .iter()
                    .all(|dependency| completed.contains(dependency))
            {
                task.state = TaskState::Ready;
            }
        }
    }
}

pub fn validate_plan_dag(plan: &RuntimePlan) -> AcResult<()> {
    let task_ids = plan
        .tasks
        .iter()
        .map(|task| task.id.clone())
        .collect::<BTreeSet<_>>();
    for task in &plan.tasks {
        if task.dependencies.contains(&task.id) {
            return Err(AcError::validation(
                "RUNTIME-DAG_SELF_DEPENDENCY",
                "task cannot depend on itself",
            ));
        }
        for dependency in &task.dependencies {
            if !task_ids.contains(dependency) {
                return Err(AcError::validation(
                    "RUNTIME-DAG_MISSING_DEPENDENCY",
                    "task dependency does not exist",
                ));
            }
        }
    }
    for task in &plan.tasks {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        visit_task(task.id.clone(), &plan.tasks, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit_task(
    task_id: StableId,
    tasks: &[WorkerTask],
    visiting: &mut BTreeSet<StableId>,
    visited: &mut BTreeSet<StableId>,
) -> AcResult<()> {
    if visited.contains(&task_id) {
        return Ok(());
    }
    if !visiting.insert(task_id.clone()) {
        return Err(AcError::validation(
            "RUNTIME-DAG_CYCLE",
            "task dependency graph contains a cycle",
        ));
    }
    if let Some(task) = tasks.iter().find(|task| task.id == task_id) {
        for dependency in &task.dependencies {
            visit_task(dependency.clone(), tasks, visiting, visited)?;
        }
    }
    visiting.remove(&task_id);
    visited.insert(task_id);
    Ok(())
}

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

fn is_meaningful(kind: ProgressKind) -> bool {
    !matches!(kind, ProgressKind::ToolActivity)
}

fn resource_allows(resources: &ResourceSnapshot, active_impl_workers: usize) -> bool {
    match resources.memory_pressure {
        MemoryPressure::Critical => false,
        MemoryPressure::Pressure => active_impl_workers < 1 && resources.active_builds == 0,
        MemoryPressure::Normal => active_impl_workers < 2 && !resources.cpu_busy,
    }
}

fn conflict_blocks(task_id: &StableId, conflicts: &[ConflictPrediction]) -> bool {
    conflicts.iter().any(|conflict| {
        conflict.risk >= 80 && (&conflict.left_task == task_id || &conflict.right_task == task_id)
    })
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AgentSession {
    id: StableId,
    worker: Worker,
    state: AgentSessionState,
    token: CancellationToken,
    queue: VecDeque<String>,
    events: Vec<RuntimeEvent>,
    checkpoints: Vec<RuntimeCheckpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeStep {
    ModelTurn(NormalizedInferenceRequest),
    ToolCall(ToolRequest),
    BuildContext(Vec<ContextNode>, u32),
    Checkpoint(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCheckpoint {
    pub id: StableId,
    pub session_id: StableId,
    pub label: String,
    pub created_at: TimestampMillis,
}

pub trait RuntimeServices {
    fn stream_model(
        &mut self,
        request: NormalizedInferenceRequest,
        cancel: &CancellationToken,
    ) -> AcResult<Vec<ProviderStreamEvent>>;

    fn invoke_tool(&mut self, request: ToolRequest) -> AcResult<ToolResult>;

    fn build_context(&mut self, nodes: Vec<ContextNode>, budget: u32) -> AcResult<ContextPack>;

    fn checkpoint(
        &mut self,
        session_id: &StableId,
        label: &str,
    ) -> AcResult<Option<CheckpointRecord>>;
}

impl AgentSession {
    pub fn new(worker: Worker) -> Self {
        let id = StableId::new("session");
        let mut session = Self {
            id,
            worker,
            state: AgentSessionState::Created,
            token: CancellationToken::new(),
            queue: VecDeque::new(),
            events: Vec::new(),
            checkpoints: Vec::new(),
        };
        session.record(RuntimeEventKind::SessionCreated);
        session
    }

    pub fn id(&self) -> &StableId {
        &self.id
    }

    pub fn worker(&self) -> &Worker {
        &self.worker
    }

    pub fn state(&self) -> AgentSessionState {
        self.state
    }

    pub fn enqueue(&mut self, work_item: impl Into<String>) -> AcResult<()> {
        if self.state == AgentSessionState::Stopped {
            return Err(AcError::conflict(
                "RUNTIME-SESSION_STOPPED",
                "cannot enqueue work into a stopped session",
            ));
        }
        let item = work_item.into();
        if item.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-EMPTY_WORK_ITEM",
                "work item cannot be empty",
            ));
        }
        self.queue.push_back(item);
        Ok(())
    }

    pub fn run_until_idle(&mut self) -> AcResult<()> {
        if self.state == AgentSessionState::Created {
            self.state = AgentSessionState::Running;
            self.record(RuntimeEventKind::SessionStarted);
        }
        if self.state != AgentSessionState::Running {
            return Err(AcError::conflict(
                "RUNTIME-SESSION_NOT_RUNNING",
                "session cannot process work in current state",
            ));
        }
        while let Some(item) = self.queue.pop_front() {
            if self.token.is_cancelled() {
                self.state = AgentSessionState::Cancelling;
                self.record(RuntimeEventKind::CancellationRequested);
                break;
            }
            self.record(RuntimeEventKind::WorkItemProcessed(item));
        }
        Ok(())
    }

    pub fn run_plan<S: RuntimeServices>(
        &mut self,
        steps: Vec<RuntimeStep>,
        services: &mut S,
    ) -> AcResult<()> {
        self.ensure_started()?;
        for step in steps {
            if self.token.is_cancelled() {
                self.state = AgentSessionState::Cancelling;
                self.record(RuntimeEventKind::CancellationRequested);
                break;
            }
            match step {
                RuntimeStep::ModelTurn(request) => {
                    for event in services.stream_model(request, &self.token)? {
                        self.record(RuntimeEventKind::ModelEvent(event));
                    }
                }
                RuntimeStep::ToolCall(request) => {
                    let result = services.invoke_tool(request)?;
                    self.record(RuntimeEventKind::ToolResult(result.evidence_ref));
                }
                RuntimeStep::BuildContext(nodes, budget) => {
                    let pack = services.build_context(nodes, budget)?;
                    self.record(RuntimeEventKind::ContextPackBuilt(pack.id));
                }
                RuntimeStep::Checkpoint(label) => {
                    let checkpoint = RuntimeCheckpoint {
                        id: StableId::new("rtcp"),
                        session_id: self.id.clone(),
                        label: label.clone(),
                        created_at: TimestampMillis::now(),
                    };
                    self.checkpoints.push(checkpoint);
                    if let Some(git_checkpoint) = services.checkpoint(&self.id, &label)? {
                        self.record(RuntimeEventKind::CheckpointCreated(git_checkpoint.id));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn request_cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn stop(&mut self) -> AcResult<()> {
        match self.state {
            AgentSessionState::Created
            | AgentSessionState::Running
            | AgentSessionState::Cancelling => {
                self.state = AgentSessionState::Stopped;
                self.record(RuntimeEventKind::SessionStopped);
                Ok(())
            }
            AgentSessionState::Stopped => Err(AcError::conflict(
                "RUNTIME-SESSION_ALREADY_STOPPED",
                "session is already stopped",
            )),
        }
    }

    pub fn events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    pub fn checkpoints(&self) -> &[RuntimeCheckpoint] {
        &self.checkpoints
    }

    fn ensure_started(&mut self) -> AcResult<()> {
        match self.state {
            AgentSessionState::Created => {
                self.state = AgentSessionState::Running;
                self.record(RuntimeEventKind::SessionStarted);
                Ok(())
            }
            AgentSessionState::Running => Ok(()),
            AgentSessionState::Cancelling | AgentSessionState::Stopped => Err(AcError::conflict(
                "RUNTIME-SESSION_NOT_RUNNING",
                "session cannot process plan in current state",
            )),
        }
    }

    fn record(&mut self, kind: RuntimeEventKind) {
        self.events.push(RuntimeEvent {
            id: StableId::new("re"),
            session_id: self.id.clone(),
            kind,
            created_at: TimestampMillis::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_processes_events_without_owning_kernel_state() {
        let mut session = AgentSession::new(Worker::new());
        session.enqueue("model-turn-placeholder").unwrap();
        session.run_until_idle().unwrap();
        assert_eq!(session.state(), AgentSessionState::Running);
        assert!(session
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::WorkItemProcessed(_))));
    }

    #[test]
    fn cancellation_stops_at_session_boundary() {
        let mut session = AgentSession::new(Worker::new());
        session.enqueue("first").unwrap();
        session.request_cancel();
        session.run_until_idle().unwrap();
        assert_eq!(session.state(), AgentSessionState::Cancelling);
    }

    struct FakeServices;

    impl RuntimeServices for FakeServices {
        fn stream_model(
            &mut self,
            _request: NormalizedInferenceRequest,
            _cancel: &CancellationToken,
        ) -> AcResult<Vec<ProviderStreamEvent>> {
            Ok(vec![
                ProviderStreamEvent::Delta("ok".to_string()),
                ProviderStreamEvent::Finished,
            ])
        }

        fn invoke_tool(&mut self, request: ToolRequest) -> AcResult<ToolResult> {
            Ok(ToolResult {
                request_id: request.id,
                status: ac_tool::ToolStatus::Succeeded,
                observation: "ok".to_string(),
                evidence_ref: StableId::new("ev"),
                finished_at: TimestampMillis::now(),
            })
        }

        fn build_context(&mut self, nodes: Vec<ContextNode>, budget: u32) -> AcResult<ContextPack> {
            Ok(ContextPack {
                id: StableId::new("ctx"),
                nodes,
                budget,
                omitted_count: 0,
            })
        }

        fn checkpoint(
            &mut self,
            _session_id: &StableId,
            _label: &str,
        ) -> AcResult<Option<CheckpointRecord>> {
            Ok(None)
        }
    }

    #[test]
    fn run_plan_routes_through_runtime_services() {
        let mut session = AgentSession::new(Worker::new());
        let request = NormalizedInferenceRequest {
            model_id: StableId::new("model"),
            prompt: "hello".to_string(),
            required: Vec::new(),
            max_output_tokens: 64,
        };
        session
            .run_plan(
                vec![
                    RuntimeStep::ModelTurn(request),
                    RuntimeStep::Checkpoint("save".to_string()),
                ],
                &mut FakeServices,
            )
            .unwrap();
        assert!(session
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ModelEvent(_))));
        assert_eq!(session.checkpoints().len(), 1);
    }

    #[test]
    fn durable_task_graph_orders_retries_and_recovers_after_reopen() {
        let mission = StableId::new("mission");
        let mut graph = TaskGraph::decompose(mission.clone(), "fix a three-file fixture").unwrap();
        let mut worker = Worker::new();
        worker.assign(mission.clone()).unwrap();
        worker.transition(WorkerState::Running).unwrap();
        let first = graph.next_ready().unwrap().id.clone();
        graph.start(&first, &worker).unwrap();
        graph
            .finish(
                &first,
                &worker,
                TaskAttemptOutcome::Succeeded,
                vec![StableId::new("ev")],
                None,
            )
            .unwrap();
        let second = graph.next_ready().unwrap().id.clone();
        graph.start(&second, &worker).unwrap();
        graph
            .finish(
                &second,
                &worker,
                TaskAttemptOutcome::Failed,
                vec![StableId::new("ev")],
                Some("tool_failed".to_string()),
            )
            .unwrap();
        assert_eq!(
            graph.tasks().find(|task| task.id == second).unwrap().state,
            TaskState::Ready
        );
        let path =
            std::env::temp_dir().join(format!("agentcode-runtime-{}.sqlite", StableId::new("db")));
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            graph
                .persist(&db, &worker, &StableId::new("session"))
                .unwrap();
        }
        let db = ControlPlaneDb::open(&path).unwrap();
        assert_eq!(db.tasks_for_mission(mission.as_str()).unwrap().len(), 5);
        assert_eq!(db.task_attempts(second.as_str()).unwrap().len(), 1);
        assert_eq!(
            db.worker(worker.id.as_str()).unwrap().unwrap().state,
            "running"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn phase12_contract_planner_dag_scheduler_and_controls_work() {
        let mission_id = StableId::from_existing("mission-p12").unwrap();
        let mut autonomy = AutonomyKernel::from_goal(
            mission_id.clone(),
            "implement durable scheduler; verify with tests",
        )
        .unwrap();
        assert_eq!(
            autonomy.contract().original_goal,
            "implement durable scheduler; verify with tests"
        );
        assert!(autonomy.contract().requirements.len() >= 2);
        assert!(autonomy.plan().tasks.len() >= 2);

        let first = WorkerTask {
            id: StableId::from_existing("cycle-a").unwrap(),
            mission_id: mission_id.clone(),
            title: "a".to_string(),
            dependencies: vec![StableId::from_existing("cycle-b").unwrap()],
            state: TaskState::Pending,
            assigned_worker: None,
            retry_count: 0,
            max_retries: 1,
            evidence_refs: Vec::new(),
        };
        let second = WorkerTask {
            id: StableId::from_existing("cycle-b").unwrap(),
            mission_id: mission_id.clone(),
            title: "b".to_string(),
            dependencies: vec![StableId::from_existing("cycle-a").unwrap()],
            state: TaskState::Pending,
            assigned_worker: None,
            retry_count: 0,
            max_retries: 1,
            evidence_refs: Vec::new(),
        };
        let cyclic = RuntimePlan {
            id: StableId::new("plan"),
            mission_id: mission_id.clone(),
            revision: 1,
            tasks: vec![first, second],
            proposals: Vec::new(),
            supersedes: None,
        };
        assert_eq!(
            validate_plan_dag(&cyclic).unwrap_err().code(),
            "RUNTIME-DAG_CYCLE"
        );

        let worker = autonomy.register_worker(WorkerRole::Worker);
        autonomy.register_worker(WorkerRole::Verifier);
        let decisions = autonomy.schedule(
            &ResourceSnapshot {
                memory_pressure: MemoryPressure::Normal,
                cpu_busy: false,
                active_builds: 0,
                browser_sessions: 0,
                lsp_sessions: 0,
                local_model_loaded: false,
            },
            &[],
        );
        assert!(decisions.iter().any(|decision| decision.admitted));
        assert_eq!(decisions[0].worker_id.as_ref(), Some(&worker));

        autonomy.pause();
        assert!(autonomy
            .schedule(
                &ResourceSnapshot {
                    memory_pressure: MemoryPressure::Normal,
                    cpu_busy: false,
                    active_builds: 0,
                    browser_sessions: 0,
                    lsp_sessions: 0,
                    local_model_loaded: false,
                },
                &[],
            )
            .is_empty());
        autonomy.resume();
        assert!(autonomy
            .relevant_blackboard(&StableId::from_existing("unknown").unwrap())
            .iter()
            .any(|entry| entry.category == "resume_reconcile"));
        autonomy.cancel();
        assert!(autonomy
            .workers
            .values()
            .all(|worker| worker.active_task.is_none()));
    }

    #[test]
    fn phase12_leases_recovery_progress_roles_and_replan_work() {
        let mission_id = StableId::from_existing("mission-p12-recovery").unwrap();
        let mut autonomy = AutonomyKernel::from_goal(mission_id.clone(), "fix api").unwrap();
        let worker_one = autonomy.register_worker(WorkerRole::Worker);
        let worker_two = autonomy.register_worker(WorkerRole::Worker);
        let verifier = autonomy.register_worker(WorkerRole::Verifier);
        let task_id = autonomy.ready_tasks()[0].clone();

        let lease = autonomy
            .acquire_lease(task_id.clone(), worker_one.clone(), 50, 10)
            .unwrap();
        assert_eq!(
            autonomy
                .acquire_lease(task_id.clone(), worker_two.clone(), 50, 10)
                .unwrap_err()
                .code(),
            "RUNTIME-LEASE_HELD"
        );
        autonomy.heartbeat(&task_id, &worker_one).unwrap();
        assert_eq!(
            autonomy
                .heartbeat(&task_id, &worker_two)
                .unwrap_err()
                .code(),
            "RUNTIME-ZOMBIE_WORKER"
        );

        autonomy.leases.get_mut(&task_id).unwrap().expires_at = TimestampMillis::from_millis(0);
        assert_eq!(autonomy.recover_expired_leases(), vec![task_id.clone()]);
        let replacement = autonomy
            .acquire_lease(task_id.clone(), worker_two.clone(), 50, 10)
            .unwrap();
        assert_ne!(lease.lease_epoch, replacement.lease_epoch);
        assert_eq!(
            autonomy
                .reject_zombie_write(&task_id, &worker_one, lease.lease_epoch)
                .unwrap_err()
                .code(),
            "RUNTIME-ZOMBIE_WORKER"
        );

        for _ in 0..3 {
            autonomy.record_progress(ProgressEvent {
                id: StableId::new("progress"),
                task_id: task_id.clone(),
                worker_id: worker_two.clone(),
                kind: ProgressKind::ToolActivity,
                fingerprint: "tool:test:error42".to_string(),
                evidence_ref: None,
                created_at: TimestampMillis::from_millis(0),
            });
        }
        assert!(autonomy.repeated_loop_detected(3));
        assert!(autonomy.stalled_workers(0).contains(&worker_two));
        assert_eq!(
            AutonomyKernel::recovery_action(FailureClass::ProviderFailure),
            RecoveryAction::SwitchProvider
        );
        assert!(autonomy
            .retry_decision(task_id.clone(), "same error", "expanded context", 2)
            .is_ok());
        assert_eq!(
            autonomy
                .retry_decision(task_id.clone(), "same error", "", 3)
                .unwrap_err()
                .code(),
            "RUNTIME-RETRY_NO_CHANGE"
        );

        autonomy.record_research(ResearchResult {
            id: StableId::new("research"),
            question: "Which API changed?".to_string(),
            sources: vec!["local-docs".to_string()],
            findings: vec!["method renamed".to_string()],
            recommendation: "update call site".to_string(),
            risks: vec!["stale docs".to_string()],
            version_date: "2026-08-24".to_string(),
        });
        assert_eq!(
            autonomy
                .schedule_verifier(task_id.clone())
                .unwrap()
                .requirements
                .len(),
            autonomy.contract().requirements.len()
        );
        autonomy.send_message(MailboxMessage {
            id: StableId::new("msg"),
            mission_id: mission_id.clone(),
            sender_worker_id: verifier,
            recipient_worker_id: Some(worker_two),
            message_type: MailboxMessageType::Finding,
            subject_id: Some(task_id.clone()),
            payload: "needs one more test".to_string(),
            delivered: false,
            created_at: TimestampMillis::now(),
        });
        assert_eq!(autonomy.mailbox().len(), 2);
        autonomy.add_blackboard_entry(BlackboardEntry {
            id: StableId::new("bb"),
            mission_id: mission_id.clone(),
            category: "blocker".to_string(),
            subject_id: Some(task_id.clone()),
            content: "integration constraint".to_string(),
            created_at: TimestampMillis::now(),
        });
        assert_eq!(autonomy.relevant_blackboard(&task_id).len(), 1);
        let replan = autonomy.replan("new failing test discovered").unwrap();
        assert_eq!(
            replan.old_plan_id,
            autonomy.plan().supersedes.clone().unwrap()
        );
        assert!(autonomy
            .escalate_human("missing credentials for provider", Some(task_id.clone()))
            .is_ok());
        assert_eq!(
            autonomy
                .escalate_human("ordinary retry", Some(task_id))
                .unwrap_err()
                .code(),
            "RUNTIME-ESCALATION_NOT_ALLOWED"
        );
    }
}
