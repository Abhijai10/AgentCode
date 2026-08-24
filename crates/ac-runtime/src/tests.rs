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
