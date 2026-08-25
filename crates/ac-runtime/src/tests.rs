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
    fn provider_runtime_plan_builds_ready_graph_and_persists() {
        let mission = StableId::new("mission");
        let inspect = WorkerTask {
            id: StableId::from_existing("inspect-dynamic").unwrap(),
            mission_id: mission.clone(),
            title: "Inspect dynamic target".to_string(),
            dependencies: Vec::new(),
            state: TaskState::Pending,
            assigned_worker: None,
            retry_count: 0,
            max_retries: 2,
            evidence_refs: Vec::new(),
        };
        let modify = WorkerTask {
            id: StableId::from_existing("modify-dynamic").unwrap(),
            mission_id: mission.clone(),
            title: "Modify dynamic target".to_string(),
            dependencies: vec![inspect.id.clone()],
            state: TaskState::Pending,
            assigned_worker: None,
            retry_count: 0,
            max_retries: 2,
            evidence_refs: Vec::new(),
        };
        let plan = RuntimePlan {
            id: StableId::new("plan"),
            mission_id: mission.clone(),
            revision: 7,
            tasks: vec![inspect.clone(), modify.clone()],
            proposals: Vec::new(),
            supersedes: None,
        };
        let mut graph = TaskGraph::from_runtime_plan(&plan).unwrap();
        assert_eq!(graph.next_ready().unwrap().id, inspect.id);
        let mut worker = Worker::new();
        worker.assign(mission.clone()).unwrap();
        worker.transition(WorkerState::Running).unwrap();
        graph.start(&inspect.id, &worker).unwrap();
        graph
            .finish(
                &inspect.id,
                &worker,
                TaskAttemptOutcome::Succeeded,
                vec![StableId::new("ev")],
                None,
            )
            .unwrap();
        assert_eq!(graph.next_ready().unwrap().id, modify.id);
        let path = std::env::temp_dir().join(format!(
            "agentcode-runtime-dynamic-{}.sqlite",
            StableId::new("db")
        ));
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            graph
                .persist(&db, &worker, &StableId::new("session"))
                .unwrap();
        }
        let db = ControlPlaneDb::open(&path).unwrap();
        let recovered = db.tasks_for_mission(mission.as_str()).unwrap();
        assert_eq!(recovered.len(), 2);
        assert!(recovered
            .iter()
            .any(|task| task.title == "Modify dynamic target" && task.state == "ready"));
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

    #[test]
    fn phase23_optimization_records_metrics_and_reduces_resources() {
        let mut optimization = OptimizationEngine::new();
        let snapshot = ResourceSnapshot {
            memory_pressure: MemoryPressure::Pressure,
            cpu_busy: true,
            active_builds: 1,
            browser_sessions: 1,
            lsp_sessions: 2,
            local_model_loaded: true,
        };
        let telemetry = optimization
            .record_resource_telemetry(
                "runtime",
                &snapshot,
                ResourceTelemetryInput {
                    rss_bytes: 700_000_000,
                    cpu_millis: 42,
                    disk_bytes: 1024,
                    process_count: 5,
                    worker_count: 3,
                },
            )
            .unwrap();
        assert_eq!(telemetry.browser_sessions, 1);
        let task_id = StableId::new("task");
        let usage = optimization
            .record_token_usage(
                task_id.clone(),
                Some(StableId::new("routing")),
                TokenUsageInput {
                    input_tokens: 100,
                    output_tokens: 50,
                    context_tokens: 400,
                    compressed_tokens: 200,
                    estimated_cost_micros: 900,
                    verified: true,
                },
            )
            .unwrap();
        assert_eq!(usage.compression_ratio(), 50);
        let decision = optimization.govern(
            &ResourcePolicy {
                max_workers: 4,
                max_rss_bytes: 1_000_000_000,
                max_cpu_busy_workers: 1,
                budget_limit_micros: Some(800),
            },
            &snapshot,
            3,
            900,
        );
        assert!(decision.degraded_mode);
        assert_eq!(decision.admitted_workers, 1);
        assert!(optimization
            .local_model_lifecycle(&snapshot, 10 * 60 * 1000, 5 * 60 * 1000)
            .unload);
        let lsp = optimization.lsp_lifecycle(&snapshot, 6 * 60 * 1000, true);
        assert!(lsp.stop_idle);
        assert!(lsp.restart_unhealthy);
        assert!(!optimization
            .heavy_index_policy("zoekt", 100, 1, &snapshot)
            .enabled);
        let report = optimization.report(vec!["before 800 tokens; after 550 tokens".to_string()]);
        assert_eq!(report.verified_tokens, usage.total_tokens());
        assert_eq!(report.cost_per_verified_task_micros, Some(900));
        assert_eq!(optimization.token_usage()[0].task_id, task_id);
    }

    #[test]
    fn phase24_chaos_harness_recovers_provider_worker_db_and_verification_faults() {
        let mut harness = ChaosHarness::new();
        let catalog = ChaosHarness::catalog();
        assert_eq!(catalog.len(), 25);
        let selected = [
            ChaosFaultKind::Provider429,
            ChaosFaultKind::WorkerDeath,
            ChaosFaultKind::ZombieWorker,
            ChaosFaultKind::SqliteInterrupt,
            ChaosFaultKind::FalseCompletion,
        ];
        for fault in selected {
            let scenario = catalog
                .iter()
                .find(|scenario| scenario.fault_kind == fault)
                .unwrap();
            let result = harness
                .run_scenario(
                    scenario,
                    ChaosRunConfig {
                        mission_id: StableId::new("mission"),
                        seed: 24,
                        repeats: 3,
                    },
                )
                .unwrap();
            assert_eq!(result.passes, 3);
            assert!(result.state_equivalent);
            assert!(result.unresolved_failures.is_empty());
            assert!(result.timeline.iter().any(|event| event.phase == "recover"));
            assert!(result.timeline.iter().any(|event| event.phase == "oracle"));
        }
        let report = harness.reliability_report("phase-24");
        assert_eq!(report.experiments, selected.len() as u32);
        assert_eq!(report.recovery_percent, 100);
        assert!(report.regression_list.is_empty());
    }

    #[test]
    fn phase24_chaos_requires_repeated_runs() {
        let mut harness = ChaosHarness::new();
        let scenario = ChaosHarness::catalog()
            .into_iter()
            .find(|scenario| scenario.fault_kind == ChaosFaultKind::ProviderTimeout)
            .unwrap();
        let error = harness
            .run_scenario(
                &scenario,
                ChaosRunConfig {
                    mission_id: StableId::new("mission"),
                    seed: 1,
                    repeats: 1,
                },
            )
            .unwrap_err();
        assert_eq!(error.code(), "CHAOS-INSUFFICIENT_REPETITION");
    }
}
