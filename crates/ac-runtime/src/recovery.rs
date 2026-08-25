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
pub struct HydratedSession {
    pub session: ac_db::PersistedSession,
    pub graph: TaskGraph,
    pub checkpoints: Vec<ac_db::PersistedCheckpoint>,
    pub reconciled_tasks: Vec<StableId>,
}

pub struct RuntimeHydrator;

impl RuntimeHydrator {
    /// Rebuilds the scheduler-facing graph from the authoritative control plane.
    /// A persisted running task is never blindly replayed: its stale owner is fenced
    /// and it returns to retryable state, after which dependency refresh determines
    /// whether it can become ready.
    pub fn hydrate_session(db: &ControlPlaneDb, session: ac_db::PersistedSession) -> AcResult<HydratedSession> {
        let mut graph = TaskGraph::default();
        let mut reconciled_tasks = Vec::new();
        for row in db.tasks_for_mission(&session.mission_id)? {
            let id = StableId::from_existing(&row.id)?;
            let mission_id = StableId::from_existing(&row.mission_id)?;
            let dependencies = row.dependencies_json.split(',').filter(|value| !value.is_empty()).map(StableId::from_existing).collect::<AcResult<Vec<_>>>()?;
            let mut state = task_state(&row.state)?;
            if state == TaskState::Running {
                state = TaskState::Retryable;
                reconciled_tasks.push(id.clone());
            }
            graph.tasks.insert(id.clone(), WorkerTask { id, mission_id, title: row.title, dependencies, state, assigned_worker: None, retry_count: row.retry_count, max_retries: row.max_retries, evidence_refs: Vec::new() });
        }
        graph.refresh_ready();
        for task in graph.tasks.values() {
            if reconciled_tasks.contains(&task.id) {
                db.save_task(&TaskRecord { id: task.id.to_string(), mission_id: task.mission_id.to_string(), title: task.title.clone(), state: task_state_name(task.state).to_string(), dependencies_json: task.dependencies.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), assigned_worker_id: None, retry_count: task.retry_count, max_retries: task.max_retries, updated_at_ms: TimestampMillis::now().as_millis() as i64 })?;
            }
        }
        let session_id = StableId::from_existing(&session.id)?;
        Ok(HydratedSession { checkpoints: db.checkpoints_for_session(&session_id)?, session, graph, reconciled_tasks })
    }

    pub fn hydrate_interrupted(db: &ControlPlaneDb) -> AcResult<Vec<HydratedSession>> {
        db.interrupted_sessions()?.into_iter().map(|session| Self::hydrate_session(db, session)).collect()
    }
}

fn task_state(value: &str) -> AcResult<TaskState> {
    match value { "pending" => Ok(TaskState::Pending), "ready" => Ok(TaskState::Ready), "running" => Ok(TaskState::Running), "retryable" => Ok(TaskState::Retryable), "completed" => Ok(TaskState::Completed), "failed" => Ok(TaskState::Failed), "cancelled" => Ok(TaskState::Cancelled), _ => Err(AcError::validation("RUNTIME-HYDRATE_TASK_STATE", format!("unknown task state {value}"))) }
}
fn task_state_name(value: TaskState) -> &'static str { match value { TaskState::Pending => "pending", TaskState::Ready => "ready", TaskState::Running => "running", TaskState::Retryable => "retryable", TaskState::Completed => "completed", TaskState::Failed => "failed", TaskState::Cancelled => "cancelled" } }
