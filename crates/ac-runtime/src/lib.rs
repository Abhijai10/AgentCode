use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_context::{ContextNode, ContextPack};
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
}

impl Worker {
    pub fn new() -> Self {
        Self {
            id: StableId::new("worker"),
            workspace_ref: None,
        }
    }

    pub fn assigned_to(workspace_ref: StableId) -> Self {
        Self {
            id: StableId::new("worker"),
            workspace_ref: Some(workspace_ref),
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
}
