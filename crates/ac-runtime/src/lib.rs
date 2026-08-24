use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_context::{ContextNode, ContextPack};
use ac_db::{ControlPlaneDb, TaskAttemptRecord, TaskRecord, WorkerRecord};
use ac_git::CheckpointRecord;
use ac_provider::{NormalizedInferenceRequest, ProviderStreamEvent};
use ac_tool::{ToolRequest, ToolResult};

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
}
