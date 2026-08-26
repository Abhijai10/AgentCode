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

    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
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
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub description: String,
    pub required: bool,
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
