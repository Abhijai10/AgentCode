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
        Self::with_id(StableId::new("session"), worker)
    }

    /// Rehydrates a durable daemon-owned session without allocating a second
    /// identity for the same mission execution.
    pub fn with_id(id: StableId, worker: Worker) -> Self {
        Self::with_id_and_token(id, worker, CancellationToken::new())
    }

    pub fn with_id_and_token(id: StableId, worker: Worker, token: CancellationToken) -> Self {
        let mut session = Self {
            id,
            worker,
            state: AgentSessionState::Created,
            token,
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

    pub fn bind_workspace(&mut self, workspace_id: StableId) -> AcResult<()> {
        if self.state != AgentSessionState::Created || self.worker.workspace_ref.is_some() {
            return Err(AcError::conflict(
                "RUNTIME-WORKSPACE_ALREADY_BOUND",
                "workspace may only be bound before session execution",
            ));
        }
        self.worker.workspace_ref = Some(workspace_id);
        Ok(())
    }

    pub fn start_worker_for_mission(&mut self, mission_id: StableId) -> AcResult<()> {
        if self.worker.mission_id.is_none() {
            self.worker.assign(mission_id)?;
        }
        if self.worker.state == WorkerState::Assigned {
            self.worker.transition(WorkerState::Running)?;
        }
        Ok(())
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

    pub fn cancellation_token(&self) -> CancellationToken { self.token.clone() }

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
