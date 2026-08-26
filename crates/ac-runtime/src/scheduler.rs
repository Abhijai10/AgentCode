impl TaskGraph {
    pub fn from_runtime_plan(plan: &RuntimePlan) -> AcResult<Self> {
        validate_plan_dag(plan)?;
        let mut graph = Self::default();
        for task in &plan.tasks {
            graph.tasks.insert(task.id.clone(), task.clone());
        }
        graph.refresh_ready();
        Ok(graph)
    }

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
                acceptance_criteria_json: serde_json::to_string(&task.acceptance_criteria).map_err(|error| AcError::validation("RUNTIME-TASK_CRITERIA_SERIALIZE", error.to_string()))?,
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
                acceptance_criteria: Vec::new(),
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
