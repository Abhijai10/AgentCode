fn memory_fact_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryFactRow> {
    Ok(MemoryFactRow {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        mission_id: row.get(2)?,
        task_id: row.get(3)?,
        branch: row.get(4)?,
        statement: row.get(5)?,
        fact_type: row.get(6)?,
        source: row.get(7)?,
        confidence: row.get(8)?,
        freshness: row.get(9)?,
        memory_class: row.get(10)?,
        observed_commit: row.get(11)?,
        conflict_set_id: row.get(12)?,
        valid_from_ms: row.get(13)?,
        valid_until_ms: row.get(14)?,
        superseded_by: row.get(15)?,
        last_validation_ms: row.get(16)?,
    })
}

fn millis(ts: TimestampMillis) -> i64 {
    ts.as_millis().min(i64::MAX as u128) as i64
}

fn stable_ids_csv(ids: &[StableId]) -> String {
    ids.iter()
        .map(StableId::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn mission_state(state: MissionState) -> &'static str {
    match state {
        MissionState::Created => "created",
        MissionState::Active => "active",
        MissionState::Completed => "completed",
        MissionState::Cancelled => "cancelled",
    }
}

fn decision_kind(kind: KernelDecisionKind) -> &'static str {
    match kind {
        KernelDecisionKind::CreateMission => "create_mission",
        KernelDecisionKind::ActivateMission => "activate_mission",
        KernelDecisionKind::CompleteMission => "complete_mission",
        KernelDecisionKind::CancelMission => "cancel_mission",
        KernelDecisionKind::ApproveChangeSet => "approve_changeset",
    }
}

fn db_error(error: rusqlite::Error) -> AcError {
    AcError::new(
        "DB-SQLITE",
        error.to_string(),
        ac_common::ErrorKind::Internal,
        ac_common::Retryability::NotRetryable,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_changeset::{ChangeOperation, ChangeSet};
    use ac_git::GitCoordinator;
    use ac_kernel::{AllowAllPolicy, Kernel};
    use std::collections::BTreeSet;
    use std::fs;
    use std::process::Command;

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn sqlite_store_persists_kernel_state() {
        let mut db = ControlPlaneDb::open_memory().unwrap();
        db.migrate().unwrap();
        assert_eq!(db.user_version().unwrap(), 12);

        let mut kernel = Kernel::new(AllowAllPolicy);
        kernel.start().unwrap();
        let mission_id = kernel.create_mission("persist me").unwrap();
        let mission = kernel.mission(&mission_id).unwrap();
        db.put_mission(mission).unwrap();
        for event in kernel.events() {
            db.append_kernel_event(event).unwrap();
        }

        let persisted = db.get_mission(&mission_id).unwrap().unwrap();
        assert_eq!(persisted.original_goal, "persist me");
        assert_eq!(persisted.state, "created");
    }

    #[test]
    fn sqlite_file_survives_reopen_with_event_history() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let mission_id;
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let mut kernel = Kernel::new(AllowAllPolicy);
            kernel.start().unwrap();
            mission_id = kernel.create_mission("durable goal").unwrap();
            db.put_mission(kernel.mission(&mission_id).unwrap())
                .unwrap();
            for event in kernel.events() {
                db.append_kernel_event(event).unwrap();
            }
            assert_eq!(db.kernel_event_count().unwrap(), 1);
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let mission = db.get_mission(&mission_id).unwrap().unwrap();
            assert_eq!(mission.original_goal, "durable goal");
            assert_eq!(db.kernel_event_count().unwrap(), 1);
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn interrupted_sessions_are_recovered_after_reopen() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let session_id = StableId::new("session");
        let mission_id = StableId::new("mission");
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_session(&session_id, &mission_id, "executing")
                .unwrap();
            db.save_checkpoint(&StableId::new("cp"), &session_id, 2, "executing")
                .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let interrupted = db.interrupted_sessions().unwrap();
            assert_eq!(interrupted.len(), 1);
            assert_eq!(interrupted[0].id, session_id.to_string());
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn lifecycle_state_recovers_after_reopen() {
        let db_path =
            std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree_path =
            std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree_path);
        fs::create_dir_all(source.join("src")).unwrap();
        fs::write(source.join("src/lib.rs"), "pub fn answer() -> u32 { 41 }\n").unwrap();
        run_git(&source, ["init"]);
        run_git(&source, ["add", "."]);
        run_git(
            &source,
            [
                "-c",
                "user.name=AgentCode Test",
                "-c",
                "user.email=agentcode@example.test",
                "commit",
                "-m",
                "initial",
            ],
        );

        let session_id = StableId::new("session");
        let mission_id = StableId::new("mission");
        let changeset_id;
        let worktree_id;
        let checkpoint_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.save_session(&session_id, &mission_id, "executing")
                .unwrap();

            let mut git = GitCoordinator::new();
            worktree_id = git
                .create_task_workspace(
                    source.clone(),
                    worktree_path.clone(),
                    mission_id.clone(),
                    session_id.clone(),
                )
                .unwrap();
            fs::write(
                worktree_path.join("src/lib.rs"),
                "pub fn answer() -> u32 { 42 }\n",
            )
            .unwrap();
            checkpoint_id = git.checkpoint_current(&worktree_id, "fix answer").unwrap();
            db.save_worktree(git.worktree(&worktree_id).unwrap())
                .unwrap();
            db.save_git_checkpoint(git.checkpoint_record(&checkpoint_id).unwrap())
                .unwrap();

            let mut changeset = ChangeSet::propose(
                vec![ChangeOperation::WriteFile {
                    path: "src/lib.rs".to_string(),
                    expected_hash: None,
                    new_hash: "len:29".to_string(),
                }],
                None,
            )
            .unwrap();
            changeset.validate().unwrap();
            changeset_id = changeset.id.clone();
            db.save_changeset(&changeset).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&db_path).unwrap();
            let session = db.get_session(&session_id).unwrap().unwrap();
            let worktree = db.get_worktree(&worktree_id).unwrap().unwrap();
            let checkpoints = db.git_checkpoints_for_worktree(&worktree_id).unwrap();
            let changeset = db.get_changeset(&changeset_id).unwrap().unwrap();

            assert_eq!(session.state, "executing");
            assert_eq!(worktree.owner_mission_id, mission_id.to_string());
            assert_eq!(checkpoints[0].commit_ref, worktree.current_commit);
            assert_eq!(checkpoints[0].reason, "fix answer");
            assert_eq!(checkpoints[0].id, checkpoint_id.to_string());
            assert_eq!(changeset.state, "Validated");
            assert!(changeset.operations_json.contains("src/lib.rs"));
        }
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(worktree_path);
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn routing_decision_evidence_survives_reopen_without_secrets() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let decision = RoutingDecisionRecord {
            id: StableId::new("routing").to_string(),
            task_id: StableId::new("task").to_string(),
            candidates_json: "[{\"connection\":\"free\",\"score\":90}]".to_string(),
            selected_json: Some("{\"connection\":\"free\"}".to_string()),
            rejected_json: "[\"paid_disallowed\"]".to_string(),
            fallback_reason: Some("connection-1:RateLimit".to_string()),
            latency_ms: 12,
            input_tokens: 7,
            output_tokens: 11,
            estimated_cost_micros: 0,
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_routing_decision(&decision).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let loaded = db.routing_decision(&decision.id).unwrap().unwrap();
            assert_eq!(loaded.task_id, decision.task_id);
            assert_eq!(loaded.output_tokens, 11);
            assert!(!format!("{:?}", loaded).contains("SECRET"));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tool_output_evidence_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-tool-{}.sqlite", StableId::new("db")));
        let record = ToolExecutionRecord {
            id: StableId::new("toolrun").to_string(),
            tool_call_id: StableId::new("toolreq").to_string(),
            tool_id: "cmd.exec".to_string(),
            status: "Succeeded".to_string(),
            manifest_json: "argv:/usr/bin/env".to_string(),
            raw_output: "status:0\nstdout:ok".to_string(),
            evidence_ref: StableId::new("ev").to_string(),
            created_at_ms: millis(TimestampMillis::now()) as u128,
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_tool_execution(&record).unwrap();
        }
        let db = ControlPlaneDb::open(&path).unwrap();
        let loaded = db.tool_execution(&record.id).unwrap().unwrap();
        assert_eq!(loaded.raw_output, record.raw_output);
        assert_eq!(loaded.evidence_ref, record.evidence_ref);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn semantic_graph_evidence_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-semantic-{}.sqlite", StableId::new("db")));
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_semantic_index(
                "repo-1",
                "abc",
                "wt-1",
                &[(
                    "lsp-1".to_string(),
                    "typescript".to_string(),
                    "/repo".to_string(),
                    None,
                    "running".to_string(),
                    0,
                )],
                &[(
                    "api_route".to_string(),
                    "app/api/users/route.ts".to_string(),
                    "/api/users".to_string(),
                    "db/schema.sql".to_string(),
                    "users".to_string(),
                    "api-route-adapter".to_string(),
                    70,
                )],
                &[(
                    "app/api/users/route.ts".to_string(),
                    1,
                    "warning".to_string(),
                    "sample diagnostic".to_string(),
                    70,
                )],
                &[(
                    "npm-package".to_string(),
                    "apps/web".to_string(),
                    Some("web".to_string()),
                    "apps/web/package.json".to_string(),
                )],
                &[(
                    "scip".to_string(),
                    false,
                    "optional until benchmark threshold".to_string(),
                    3,
                    1,
                )],
            )
            .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(db.semantic_edge_count("repo-1").unwrap(), 1);
            assert_eq!(db.workspace_boundary_count("repo-1").unwrap(), 1);
            assert_eq!(
                db.optional_index_enabled("repo-1", "scip").unwrap(),
                Some(false)
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn persistent_memory_survives_reopen_and_source_changes_stale_facts() {
        let path =
            std::env::temp_dir().join(format!("agentcode-memory-{}.sqlite", StableId::new("db")));
        let fact = MemoryFactRow {
            id: StableId::new("mem").to_string(),
            repository_id: "repo-1".to_string(),
            mission_id: Some("mission-1".to_string()),
            task_id: Some("task-1".to_string()),
            branch: Some("main".to_string()),
            statement: "A calls B".to_string(),
            fact_type: "MODULE_RELATIONSHIP".to_string(),
            source: "LSP".to_string(),
            confidence: 90,
            freshness: "FRESH".to_string(),
            memory_class: "LONG_LIVED_REPO".to_string(),
            observed_commit: "abc".to_string(),
            conflict_set_id: None,
            valid_from_ms: millis(TimestampMillis::now()),
            valid_until_ms: None,
            superseded_by: None,
            last_validation_ms: millis(TimestampMillis::now()),
        };
        let evidence = MemoryEvidenceRow {
            fact_id: fact.id.clone(),
            evidence_ref: "ev-1".to_string(),
            file_path: Some("src/a.rs".to_string()),
            symbol: Some("A".to_string()),
            content_hash: Some("h1".to_string()),
        };
        let decision = MemoryDecisionRow {
            id: StableId::new("decision").to_string(),
            repository_id: "repo-1".to_string(),
            mission_id: Some("mission-1".to_string()),
            task_id: None,
            branch: Some("main".to_string()),
            decision: "Keep CONTEXT.md derived".to_string(),
            rationale: "Summaries are not authority".to_string(),
            authority_refs: "ev-1".to_string(),
            supersedes: None,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let task_memory = TaskMemoryRow {
            id: StableId::new("taskmem").to_string(),
            task_id: "task-1".to_string(),
            summary: "Investigated freshness fixture".to_string(),
            evidence_refs: "ev-1".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let snapshot = ContextSnapshotRow {
            id: StableId::new("snapshot").to_string(),
            repository_id: "repo-1".to_string(),
            mission_id: Some("mission-1".to_string()),
            task_id: Some("task-1".to_string()),
            branch: Some("main".to_string()),
            reason: "replacement-agent".to_string(),
            content: "# CONTEXT.md\nnot authority over current repository".to_string(),
            source_fact_ids: fact.id.clone(),
            decision_refs: decision.id.clone(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_memory_fact(&fact, &[evidence]).unwrap();
            db.save_memory_decision(&decision).unwrap();
            db.save_task_memory(&task_memory).unwrap();
            db.save_context_snapshot(&snapshot).unwrap();
            assert_eq!(
                db.mark_memory_for_source_change("src/a.rs", Some("A"), false)
                    .unwrap(),
                1
            );
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let loaded = db.memory_fact(&fact.id).unwrap().unwrap();
            assert_eq!(loaded.freshness, "POSSIBLY_STALE");
            assert_eq!(db.memory_fact_evidence(&fact.id).unwrap().len(), 1);
            assert_eq!(db.memory_decision_count("repo-1").unwrap(), 1);
            assert_eq!(db.task_memory_count("task-1").unwrap(), 1);
            assert!(db
                .context_snapshot(&snapshot.id)
                .unwrap()
                .unwrap()
                .content
                .contains("not authority"));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn context_engine_receipts_survive_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-context-{}.sqlite", StableId::new("db")));
        let manifest = ContextPackManifestRow {
            id: StableId::new("ctxmanifest").to_string(),
            pack_id: StableId::new("ctx").to_string(),
            task_id: "task-11".to_string(),
            role: "WORKER".to_string(),
            profile: "NORMAL".to_string(),
            source_fragment_ids: "ctxfrag-1,ctxfrag-2".to_string(),
            omitted_fragment_ids: "ctxfrag-3".to_string(),
            raw_evidence_refs: "raw-1".to_string(),
            cache_keys: "commit:abc:src/lib.rs".to_string(),
            score_trace: "ctxfrag-1:TARGET_SOURCE:195".to_string(),
            total_input_tokens: 320,
            hard_ceiling: 500,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let compression = ContextCompressionReceiptRow {
            id: StableId::new("ctxcompress").to_string(),
            raw_evidence_ref: "raw-1".to_string(),
            command_class: "tests".to_string(),
            compressor_id: "agentcode-rtk-fallback-v1".to_string(),
            raw_hash: "hash".to_string(),
            compressed_output: "error: failed assertion".to_string(),
            raw_token_estimate: 80,
            compressed_token_estimate: 12,
            omitted_lines: 9,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let cache = ContextCacheEntryRow {
            cache_key: "commit:abc:src/lib.rs".to_string(),
            content_hash: "hash".to_string(),
            token_estimate: 120,
            source_ref: "src-lib".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let retrieval = ContextRetrievalRecordRow {
            id: StableId::new("ctxret").to_string(),
            pack_id: manifest.pack_id.clone(),
            need: "need_related_tests".to_string(),
            reason: "worker requested tests".to_string(),
            query: "auth".to_string(),
            result_fragment_ids: "ctxfrag-2".to_string(),
            added_tokens: 30,
            degraded: false,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let metrics = ContextPackMetricsRow {
            id: StableId::new("ctxmetric").to_string(),
            pack_id: manifest.pack_id.clone(),
            role: "WORKER".to_string(),
            selected_fragments: 2,
            omitted_fragments: 1,
            total_input_tokens: 320,
            budget_target: 400,
            hard_ceiling: 500,
            deduped_fragments: 1,
            redacted_fragments: 1,
            retrieval_steps: 1,
            cache_hits: 1,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let benchmark = ContextBenchmarkResultRow {
            id: StableId::new("ctxbench").to_string(),
            task_name: "cross-module bug".to_string(),
            broad_tokens: 1_200,
            targeted_tokens: 320,
            broad_success: true,
            targeted_success: true,
            retry_delta: 0,
            latency_delta_ms: -12,
            passed: true,
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_context_pack_manifest(&manifest).unwrap();
            db.save_context_compression_receipt(&compression).unwrap();
            db.save_context_cache_entry(&cache).unwrap();
            db.save_context_retrieval_record(&retrieval).unwrap();
            db.save_context_pack_metrics(&metrics).unwrap();
            db.save_context_benchmark_result(&benchmark).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(
                db.context_pack_manifest(&manifest.id).unwrap().unwrap(),
                manifest
            );
            assert_eq!(
                db.context_compression_receipt(&compression.id)
                    .unwrap()
                    .unwrap()
                    .raw_evidence_ref,
                "raw-1"
            );
            assert_eq!(
                db.context_cache_entry(&cache.cache_key)
                    .unwrap()
                    .unwrap()
                    .source_ref,
                "src-lib"
            );
            assert_eq!(
                db.context_retrieval_records(&metrics.pack_id).unwrap()[0].need,
                "need_related_tests"
            );
            assert_eq!(
                db.context_pack_metrics(&metrics.pack_id)
                    .unwrap()
                    .unwrap()
                    .deduped_fragments,
                1
            );
            assert!(
                db.context_benchmark_result(&benchmark.id)
                    .unwrap()
                    .unwrap()
                    .passed
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase12_autonomy_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p12-{}.sqlite", StableId::new("db")));
        let contract = MissionContractRevisionRow {
            id: StableId::new("contract").to_string(),
            mission_id: "mission-p12".to_string(),
            revision: 1,
            original_goal: "complete long mission".to_string(),
            reason: "initial extraction".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let requirement = RequirementMatrixEntryRow {
            id: StableId::new("req").to_string(),
            mission_id: contract.mission_id.clone(),
            contract_revision: 1,
            description: "scheduler only runs ready tasks".to_string(),
            requirement_type: "FUNCTIONAL".to_string(),
            priority: 100,
            source: "original_goal".to_string(),
            verification_strategy: "unit test".to_string(),
            blocking: true,
            implementation_status: "IMPLEMENTED".to_string(),
            verification_status: "VERIFIED".to_string(),
            evidence_refs: "ev-1".to_string(),
            linked_task_ids: "task-1".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let lease = TaskLeaseRow {
            task_id: "task-1".to_string(),
            worker_id: "worker-1".to_string(),
            lease_epoch: 1,
            expires_at_ms: millis(TimestampMillis::now()) + 5000,
            heartbeat_interval_ms: 1000,
            state: "active".to_string(),
            updated_at_ms: millis(TimestampMillis::now()),
        };
        let message = AutonomyMailboxMessageRow {
            id: StableId::new("msg").to_string(),
            mission_id: contract.mission_id.clone(),
            sender_worker_id: "worker-1".to_string(),
            recipient_worker_id: Some("verifier-1".to_string()),
            message_type: "FINDING".to_string(),
            subject_id: Some("task-1".to_string()),
            payload: "review diff".to_string(),
            delivered: false,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let record = AutonomyRecordRow {
            id: StableId::new("autonomy").to_string(),
            mission_id: contract.mission_id.clone(),
            category: "RECOVERY".to_string(),
            subject_id: Some("task-1".to_string()),
            payload: "provider failure -> switch route".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_mission_contract_revision(&contract).unwrap();
            db.save_requirement_matrix_entry(&requirement).unwrap();
            db.save_task_lease(&lease).unwrap();
            db.save_autonomy_mailbox_message(&message).unwrap();
            db.save_autonomy_record(&record).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(
                db.mission_contract_revisions(&contract.mission_id)
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                db.requirement_matrix_entries(&contract.mission_id).unwrap()[0].verification_status,
                "VERIFIED"
            );
            assert_eq!(
                db.task_lease(&lease.task_id).unwrap().unwrap().worker_id,
                "worker-1"
            );
            assert_eq!(
                db.autonomy_mailbox_messages(&contract.mission_id).unwrap()[0].message_type,
                "FINDING"
            );
            assert_eq!(
                db.autonomy_records(&contract.mission_id, "RECOVERY")
                    .unwrap()[0]
                    .payload,
                "provider failure -> switch route"
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase13_edit_transactions_survive_reopen_and_expose_recovery_rows() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p13-{}.sqlite", StableId::new("db")));
        let mut repo = ac_changeset::MemoryFileRepository::new("rev-p13");
        repo.put("src/lib.rs", "pub fn answer() -> u32 { 41 }\n");
        let request = ac_changeset::EditRequest {
            path: "src/lib.rs".to_string(),
            precondition: ac_changeset::EditPrecondition {
                path: "src/lib.rs".to_string(),
                expected_hash: repo.hash("src/lib.rs").unwrap(),
                base_revision: "rev-p13".to_string(),
                symbol_fingerprint: None,
            },
            strategy: ac_changeset::EditStrategy::SearchReplace {
                search: "41".to_string(),
                replace: "42".to_string(),
                expected_matches: 1,
            },
        };
        let mut transaction = ac_changeset::EditEngine
            .prepare(&repo, vec![request])
            .unwrap();
        ac_changeset::EditEngine
            .apply(&mut repo, &mut transaction)
            .unwrap();
        transaction.changeset.mark_validating().unwrap();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            db.save_changeset_transaction(
                &transaction,
                Some("task-p13"),
                Some("worktree-p13"),
                "rev-p13",
                "rust",
            )
            .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let saved = db
                .edit_transaction(&transaction.id.to_string())
                .unwrap()
                .unwrap();
            assert_eq!(saved.state, "Validating");
            assert_eq!(
                db.edit_journal_entries(&transaction.id.to_string())
                    .unwrap()[0]
                    .state,
                "Applied"
            );
            assert_eq!(
                db.edit_strategy_metrics(&transaction.id.to_string())
                    .unwrap()[0]
                    .strategy,
                "SearchReplace"
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase14_verification_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p14-{}.sqlite", StableId::new("db")));
        let engine = ac_verification::VerificationEngine::new(ac_security::CapabilityPolicy::new());
        let mut evidence = ac_evidence::EvidenceStore::new();
        let profile = engine.derive_profile(
            StableId::new("task"),
            ac_verification::VerificationRisk::High,
            &ac_verification::ProjectCapabilities {
                cargo: true,
                makefile: false,
                package_json: false,
                browser: false,
                security: false,
            },
        );
        let requirement_id = StableId::new("req");
        let manifest = engine
            .record_evidence_manifest(
                "commit-p14",
                "worktree-p14",
                "cargo test --workspace",
                ac_verification::GateStatus::Passed,
                vec![requirement_id.clone()],
                vec!["src/lib.rs".to_string()],
                &mut evidence,
            )
            .unwrap();
        let link = engine.link_requirement_evidence(requirement_id.clone(), &manifest);
        let audit = engine
            .final_audit(
                ac_verification::FinalAuditInput {
                    original_goal: "finish phase 14".to_string(),
                    requirements: vec!["verification evidence exists".to_string()],
                    verified_requirement_ids: vec![requirement_id.clone()],
                    evidence_refs: vec![manifest.evidence_ref.clone()],
                    worker_completion_text: "verified".to_string(),
                    unresolved_limitations: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            db.save_verification_profile(&profile).unwrap();
            db.save_verification_manifest(
                &manifest,
                Some(profile.id.as_str()),
                Some(profile.task_id.as_str()),
            )
            .unwrap();
            db.save_requirement_verification(manifest.id.as_str(), &link)
                .unwrap();
            db.save_final_audit(
                "mission-p14",
                "finish phase 14",
                &["verification evidence exists".to_string()],
                &audit,
                true,
            )
            .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(
                db.verification_run(manifest.id.as_str())
                    .unwrap()
                    .unwrap()
                    .normalized_result,
                "Passed"
            );
            assert!(
                db.requirement_verifications(requirement_id.as_str())
                    .unwrap()[0]
                    .verified
            );
            assert!(db.final_audits("mission-p14").unwrap()[0].completion_allowed);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase15_browser_runtime_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p15-{}.sqlite", StableId::new("db")));
        let mut runtime = ac_verification::BrowserRuntime::new(
            ac_security::CapabilityPolicy::new().allow(ac_security::Capability::BrowserAutomation),
        );
        let mut evidence = ac_evidence::EvidenceStore::new();
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime
            .create_session(task.clone(), process.id.clone())
            .unwrap();
        let dev_server = runtime
            .manage_dev_server(
                task.clone(),
                vec!["npm".to_string(), "run".to_string(), "dev".to_string()],
                3000,
                "http://127.0.0.1:3000/login",
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                ac_verification::BrowserAction::Open {
                    url: dev_server.ready_url.clone(),
                    html: "<h1>Login</h1><button id=\"submit\">Submit</button>".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        let screenshot = runtime
            .capture_screenshot(
                &session.id,
                task.clone(),
                "commit-p15",
                runtime.default_viewports()[2],
                &mut evidence,
            )
            .unwrap();
        let visual = runtime.visual_qa(&screenshot, &mut evidence).unwrap();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            db.save_browser_process(&process).unwrap();
            db.save_browser_session(&session).unwrap();
            db.save_browser_dev_server(&dev_server).unwrap();
            db.save_browser_screenshot(&screenshot).unwrap();
            db.save_browser_visual_qa(&visual).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let loaded = db.browser_session(session.id.as_str()).unwrap().unwrap();
            assert_eq!(loaded.task_id, task.to_string());
            assert!(loaded.sensitive);
            let screenshots = db.browser_screenshots(session.id.as_str()).unwrap();
            assert_eq!(screenshots[0].viewport, "desktop");
            assert_eq!(
                screenshots[0].evidence_ref,
                screenshot.evidence_ref.to_string()
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase16_extension_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-phase16-{}.sqlite", StableId::new("db")));
        let skill_id = StableId::new("skill");
        let hook_id = StableId::new("hook");
        let server_id = StableId::new("mcp");
        let tool_id = StableId::new("mcptool");
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            let skill = SkillManifest {
                id: skill_id.clone(),
                name: "Rust".to_string(),
                description: "Rust testing".to_string(),
                version: "1".to_string(),
                source: "builtin/rust".to_string(),
                scope: ac_security::SkillScope::BuiltIn,
                trust_tier: ac_security::TrustTier::BuiltIn,
                trigger_hints: vec!["rust".to_string()],
                required_capabilities: [ac_security::Capability::ProcessExec("*".to_string())]
                    .into_iter()
                    .collect(),
                context_cost: 7,
                project_id: None,
                task_id: None,
                full_instructions: "Run cargo tests.".to_string(),
                loaded_at: Some(TimestampMillis::now()),
            };
            db.save_skill(&skill).unwrap();
            let hook = HookManifest {
                id: hook_id.clone(),
                extension_id: StableId::new("ext"),
                event: ac_security::HookEvent::BeforeTaskComplete,
                priority: 1,
                timeout_ms: 50,
                idempotency_key: "before-complete".to_string(),
                failure_policy: ac_security::HookFailurePolicy::BlockOperation,
                required_capabilities: BTreeSet::new(),
            };
            db.save_hook_manifest(&hook).unwrap();
            db.save_hook_invocation(&HookInvocation {
                id: StableId::new("hookrun"),
                hook_id: hook_id.clone(),
                event: ac_security::HookEvent::BeforeTaskComplete,
                outcome: ac_security::HookOutcome::Blocked,
                evidence_ref: Some(StableId::new("ev")),
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_mcp_server(&McpServerRecord {
                id: server_id.clone(),
                name: "fixture".to_string(),
                version: "1".to_string(),
                transport: ac_security::McpTransport::Stdio,
                trust_tier: ac_security::TrustTier::Project,
                health: ac_security::McpHealth::Connected,
                restart_count: 1,
            })
            .unwrap();
            db.save_mcp_tool(&McpToolRecord {
                id: tool_id.clone(),
                server_id: server_id.clone(),
                name: "read".to_string(),
                description: "Read fixture".to_string(),
                schema: "{}".to_string(),
                risk: ac_security::RiskClass::R1,
                required_capabilities: [ac_security::Capability::FilesystemRead("*".to_string())]
                    .into_iter()
                    .collect(),
            })
            .unwrap();
            db.save_mcp_invocation(&McpInvocationRecord {
                id: StableId::new("mcpinvoke"),
                server_id: server_id.clone(),
                tool_id,
                status: ac_security::SecurityDecision::Allow,
                output: "structured output".to_string(),
                evidence_ref: StableId::new("ev"),
                created_at: TimestampMillis::now(),
            })
            .unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let loaded_skill = db.skill(skill_id.as_str()).unwrap().unwrap();
            assert_eq!(loaded_skill.scope, "BuiltIn");
            assert!(loaded_skill.full_instructions.contains("cargo tests"));
            let hook_runs = db.hook_invocations(hook_id.as_str()).unwrap();
            assert_eq!(hook_runs[0].outcome, "Blocked");
            let server = db.mcp_server(server_id.as_str()).unwrap().unwrap();
            assert_eq!(server.health, "Connected");
            assert_eq!(server.restart_count, 1);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase17_security_scan_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-phase17-{}.sqlite", StableId::new("db")));
        let repo_id = StableId::new("repo");
        let scan_id;
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            let orchestrator = ac_security::BaselineSecurityOrchestrator::new(
                ac_security::SecurityPolicy::baseline(),
            );
            let input = ac_security::SecurityScanInput {
                repository_id: repo_id.clone(),
                commit: "abc123".to_string(),
                files: vec![(
                    "src/api.rs".to_string(),
                    "fn handler() { let token = \"SECRET=value\"; }".to_string(),
                )],
                dependency_manifest: Some("vulnerable-package = \"0.1.0\"".to_string()),
                include_iac: false,
            };
            let report = orchestrator.run(&input).unwrap();
            let bundle = orchestrator.reports(&report);
            scan_id = report.id.clone();
            db.save_security_scan(&input, &report).unwrap();
            db.save_security_reports(&report.id, &bundle).unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let scan = db.security_scan(scan_id.as_str()).unwrap().unwrap();
            assert_eq!(scan.repository_id, repo_id.to_string());
            let findings = db.security_findings(scan_id.as_str()).unwrap();
            assert!(findings
                .iter()
                .any(|finding| finding.root_cause == "secret-exposure"));
            assert!(findings
                .iter()
                .any(|finding| finding.root_cause == "vulnerable-dependency"));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn future_schema_version_is_rejected() {
        let mut db = ControlPlaneDb::open_memory().unwrap();
        db.connection
            .pragma_update(None, "user_version", 99)
            .unwrap();
        let error = db.migrate().unwrap_err();
        assert_eq!(error.code(), "DB-FUTURE_VERSION");
    }
}
