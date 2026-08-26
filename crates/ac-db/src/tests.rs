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
    use ac_security::{
        ActiveAuthorization, ActiveEnvironment, ActiveSecurityAction, ActiveSecurityInput,
        ActiveValidationFixture, AiHarnessKind, AiSecurityInput, BaselineSecurityOrchestrator,
        SecurityPolicy,
    };
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
        assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);

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
    fn durable_evidence_survives_reopen_without_secret_payloads() {
        let path =
            std::env::temp_dir().join(format!("agentcode-evidence-{}.sqlite", StableId::new("db")));
        let mission_id = StableId::from_existing("mission-evidence").unwrap();
        let session_id = StableId::from_existing("session-evidence").unwrap();
        let task_id = StableId::from_existing("task-evidence").unwrap();
        let evidence_id;
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_session(&session_id, &mission_id, "running").unwrap();
            let mut evidence = ac_evidence::EvidenceStore::new();
            evidence_id = evidence
                .append_tool_output(
                    ac_evidence::Provenance {
                        source: format!("mission:{mission_id};session:{session_id}"),
                        commit: None,
                        worktree: Some("worktree-evidence".to_string()),
                        tool: Some("dev.test".to_string()),
                    },
                    format!("mem://mission/{mission_id}/session/{session_id}/task/{task_id}"),
                    "status:0\nstdout:SECRET_CANARY\nstderr:",
                    &["SECRET_CANARY".to_string()],
                )
                .unwrap();
            db.append_evidence(evidence.get(&evidence_id).unwrap()).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let records = db.evidence_records().unwrap();
            assert_eq!(records.len(), 1);
            let restored = &records[0];
            assert_eq!(restored.id, evidence_id);
            assert_eq!(restored.kind, ac_evidence::EvidenceKind::CommandOutput);
            assert_eq!(restored.provenance.tool.as_deref(), Some("dev.test"));
            assert!(restored.provenance.source.contains(mission_id.as_str()));
            assert!(restored.artifact_uri.contains(task_id.as_str()));
            assert!(restored.sensitive);
            assert!(restored.raw_content.is_none());
            assert!(!restored
                .model_summary
                .as_deref()
                .unwrap_or_default()
                .contains("SECRET_CANARY"));
            let hydrated = ac_evidence::EvidenceStore::from_records(records.clone());
            assert_eq!(hydrated.get(&evidence_id).unwrap(), restored);
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
    fn phase18_phase19_security_reports_survive_reopen_and_ai_enters_common_findings() {
        let path =
            std::env::temp_dir().join(format!("agentcode-security-{}.sqlite", StableId::new("db")));
        let orchestrator = BaselineSecurityOrchestrator::new(SecurityPolicy::baseline());
        let active_input = ActiveSecurityInput {
            repository_id: StableId::new("repo"),
            commit: "p18db".to_string(),
            authorization: ActiveAuthorization {
                id: StableId::new("authz"),
                target: "http://fixture.local".to_string(),
                environment: ActiveEnvironment::AuthorizedLab,
                allowed_targets: vec!["http://fixture.local".to_string()],
                cloud_accounts: vec!["acct-lab".to_string()],
                credential_ref: Some(StableId::new("cred")),
                rate_limit_per_minute: 20,
                concurrency_limit: 1,
                forbidden_actions: vec!["destructive-production-change".to_string()],
                expires_at: TimestampMillis::from_millis(
                    TimestampMillis::now().as_millis() + 60_000,
                ),
                cleanup_required: true,
            },
            requested_actions: vec![
                ActiveSecurityAction::DastSpider,
                ActiveSecurityAction::TemplateProbe,
                ActiveSecurityAction::LabTechnique,
            ],
            fixture: Some(ActiveValidationFixture {
                id: StableId::new("fixture"),
                vulnerable_route: "http://fixture.local/admin".to_string(),
                synthetic_account: "synthetic-user".to_string(),
                canary_record: "canary-db".to_string(),
            }),
            redirect_observations: Vec::new(),
            cloud_resources: Vec::new(),
        };
        let active_report = orchestrator.run_active_security(&active_input).unwrap();
        let ai_report = orchestrator
            .run_ai_security(&AiSecurityInput {
                repository_id: StableId::new("repo"),
                commit: "p19db".to_string(),
                files: vec![(
                    "src/agent.rs".to_string(),
                    "openai user_prompt rag tool_call mcp agent memory SECRET=synthetic"
                        .to_string(),
                )],
                selected_harnesses: vec![AiHarnessKind::Promptfoo],
            })
            .unwrap();
        let active_id = active_report.id.to_string();
        let ai_id = ai_report.id.to_string();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_active_security_report(&active_input, &active_report)
                .unwrap();
            db.save_ai_security_report(&ai_report).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let loaded_active = db.active_security_report(&active_id).unwrap().unwrap();
            assert!(loaded_active.cleanup_verified);
            let loaded_ai = db.ai_security_report(&ai_id).unwrap().unwrap();
            assert!(loaded_ai.surfaces.contains("ModelGateway"));
            assert!(loaded_ai.findings_count > 0);
            let common_findings = db.security_findings(&ai_id).unwrap();
            assert!(common_findings
                .iter()
                .any(|finding| finding.root_cause == "ai-tool-abuse"));
        }
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
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
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
                python: false,
                go: false,
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
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
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
        let mut runtime = ac_verification::BrowserRuntime::deterministic_harness_for_tests(
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
                ac_verification::BrowserAction::OpenHtmlForTest {
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
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
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
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
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
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
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
                .any(|finding| finding.root_cause == "builtin-secret-pattern"));
            assert!(findings
                .iter()
                .all(|finding| finding.root_cause != "vulnerable-dependency"));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase20_21_discuss_and_design_state_survive_reopen() {
        let path = std::env::temp_dir().join(format!(
            "agentcode-phase20-21-{}.sqlite",
            StableId::new("db")
        ));
        let discuss_id = StableId::new("discuss").to_string();
        let message_id = StableId::new("dmsg").to_string();
        let plan_id = StableId::new("dplan").to_string();
        let design_id = StableId::new("design").to_string();
        let artifact_id = StableId::new("dartifact").to_string();
        let version_id = StableId::new("dversion").to_string();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
            db.save_discuss_session(&DiscussSessionRow {
                id: discuss_id.clone(),
                repository_id: "repo-a".to_string(),
                title: "Architecture discussion".to_string(),
                state: "promoted_to_plan".to_string(),
                context_manifest_refs: "ctx-a".to_string(),
                accepted_decision_refs: "decision-a".to_string(),
                created_at_ms: 1,
                updated_at_ms: 2,
            })
            .unwrap();
            db.save_discuss_message(&DiscussMessageRow {
                id: message_id.clone(),
                session_id: discuss_id.clone(),
                role: "assistant".to_string(),
                content: "Kernel owns mission truth".to_string(),
                context_ref: Some("ctx-a".to_string()),
                evidence_refs: "ev-a".to_string(),
                created_at_ms: 3,
            })
            .unwrap();
            db.save_discuss_decision_candidate(&DiscussDecisionCandidateRow {
                id: "candidate-a".to_string(),
                session_id: discuss_id.clone(),
                decision: "Keep discuss read-only".to_string(),
                rationale: "prevents silent edits".to_string(),
                evidence_refs: "ev-a".to_string(),
                accepted_decision_ref: Some("decision-a".to_string()),
                created_at_ms: 4,
            })
            .unwrap();
            db.save_discuss_plan(&DiscussPlanRow {
                id: plan_id.clone(),
                session_id: discuss_id.clone(),
                requirements: "preserve context".to_string(),
                tasks: "create mission".to_string(),
                constraints_json: "read-only".to_string(),
                open_questions: String::new(),
                accepted_decision_refs: "decision-a".to_string(),
                promoted_mission_id: Some("mission-a".to_string()),
                created_at_ms: 5,
            })
            .unwrap();
            db.save_design_session(&DesignSessionRow {
                id: design_id.clone(),
                repository_id: "repo-a".to_string(),
                product: "Design Console".to_string(),
                state: "iterating".to_string(),
                hard_constraints: "preserve flow".to_string(),
                created_at_ms: 6,
                updated_at_ms: 7,
            })
            .unwrap();
            db.save_design_artifact(&DesignArtifactRow {
                id: artifact_id.clone(),
                session_id: design_id.clone(),
                name: "Review screen".to_string(),
                artifact_type: "screen".to_string(),
                current_version: 2,
                created_at_ms: 8,
            })
            .unwrap();
            db.save_design_artifact_version(&DesignArtifactVersionRow {
                id: version_id.clone(),
                artifact_id: artifact_id.clone(),
                version: 1,
                summary: "baseline".to_string(),
                content_hash: "hash-a".to_string(),
                evidence_refs: "ev-b".to_string(),
                created_at_ms: 9,
            })
            .unwrap();
            db.save_design_visual_evaluation(&DesignVisualEvaluationRow {
                id: "eval-a".to_string(),
                artifact_version_id: version_id.clone(),
                passed: false,
                findings: "generic hero".to_string(),
                responsive_viewports: "mobile,desktop".to_string(),
                accessibility_checks: "labels,focus".to_string(),
                functional_flows: "primary flow".to_string(),
                evidence_refs: "ev-c".to_string(),
                created_at_ms: 10,
            })
            .unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let session = db.discuss_session(&discuss_id).unwrap().unwrap();
            assert_eq!(session.accepted_decision_refs, "decision-a");
            let messages = db.discuss_messages(&discuss_id).unwrap();
            assert_eq!(messages[0].id, message_id);
            assert_eq!(
                db.discuss_plan(&plan_id)
                    .unwrap()
                    .unwrap()
                    .promoted_mission_id,
                Some("mission-a".to_string())
            );
            assert_eq!(
                db.design_session(&design_id).unwrap().unwrap().product,
                "Design Console"
            );
            assert_eq!(
                db.design_artifact_versions(&artifact_id).unwrap()[0].id,
                version_id
            );
            let evaluations = db.design_visual_evaluations(&version_id).unwrap();
            assert!(!evaluations[0].passed);
            assert_eq!(evaluations[0].functional_flows, "primary flow");
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase22_23_desktop_and_optimization_state_survive_reopen() {
        let path = std::env::temp_dir().join(format!(
            "agentcode-phase22-23-{}.sqlite",
            StableId::new("db")
        ));
        let session_id = StableId::new("desktop").to_string();
        let project_id = StableId::new("project").to_string();
        let mission_id = StableId::new("mission").to_string();
        let task_id = StableId::new("task").to_string();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
            db.save_desktop_session(&DesktopSessionRow {
                id: session_id.clone(),
                active_project_id: Some(project_id.clone()),
                active_mission_id: Some(mission_id.clone()),
                selected_view: "mission".to_string(),
                window_open: false,
                daemon_connected: true,
                created_at_ms: 1,
                updated_at_ms: 2,
            })
            .unwrap();
            db.save_desktop_project(&DesktopProjectRow {
                id: project_id.clone(),
                name: "AgentCode".to_string(),
                path: "/repo".to_string(),
                repository_id: "repo-a".to_string(),
                last_opened_at_ms: 3,
            })
            .unwrap();
            db.save_desktop_preference(&DesktopPreferenceRow {
                session_id: session_id.clone(),
                appearance: "dark".to_string(),
                notifications_enabled: true,
                completion_sound_enabled: false,
                reduced_motion: true,
                budget_limit_micros: Some(10_000),
            })
            .unwrap();
            db.save_desktop_ui_state(&DesktopUiStateRow {
                session_id: session_id.clone(),
                serialized_state: "view=mission".to_string(),
                updated_at_ms: 4,
            })
            .unwrap();
            db.save_desktop_approval_record(&DesktopApprovalRecordRow {
                id: "approval-record-a".to_string(),
                approval_id: "approval-a".to_string(),
                mission_id: mission_id.clone(),
                approval_kind: "changeset".to_string(),
                decision: "approved".to_string(),
                explanation: "apply verified changes".to_string(),
                evidence_refs: "ev-a".to_string(),
                created_at_ms: 5,
            })
            .unwrap();
            db.save_token_usage_record(&TokenUsageRecordRow {
                id: "usage-a".to_string(),
                task_id: task_id.clone(),
                provider_call_id: Some("routing-a".to_string()),
                input_tokens: 100,
                output_tokens: 50,
                context_tokens: 400,
                compressed_tokens: 200,
                estimated_cost_micros: 900,
                verified: true,
                created_at_ms: 6,
            })
            .unwrap();
            db.save_resource_telemetry_record(&ResourceTelemetryRecordRow {
                id: "resource-a".to_string(),
                component: "runtime".to_string(),
                rss_bytes: 700_000_000,
                cpu_millis: 42,
                disk_bytes: 1024,
                process_count: 5,
                worker_count: 2,
                browser_sessions: 1,
                lsp_sessions: 2,
                local_model_loaded: true,
                created_at_ms: 7,
            })
            .unwrap();
            db.save_optimization_report(&OptimizationReportRow {
                id: "report-a".to_string(),
                total_tokens: 550,
                verified_tokens: 550,
                total_cost_micros: 900,
                cost_per_verified_task_micros: Some(900),
                average_compression_ratio: 50,
                before_after: "before 800; after 550".to_string(),
                created_at_ms: 8,
            })
            .unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let session = db.desktop_session(&session_id).unwrap().unwrap();
            assert!(!session.window_open);
            assert_eq!(session.active_mission_id, Some(mission_id.clone()));
            assert_eq!(db.recent_desktop_projects().unwrap()[0].path, "/repo");
            assert_eq!(
                db.desktop_preference(&session_id)
                    .unwrap()
                    .unwrap()
                    .appearance,
                "dark"
            );
            assert_eq!(
                db.desktop_approval_records(&mission_id).unwrap()[0].decision,
                "approved"
            );
            assert!(db.token_usage_records(&task_id).unwrap()[0].verified);
            assert_eq!(
                db.resource_telemetry_records("runtime").unwrap()[0].worker_count,
                2
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase24_25_chaos_and_dogfood_state_survive_reopen() {
        let path = std::env::temp_dir().join(format!(
            "agentcode-phase24-25-{}.sqlite",
            StableId::new("db")
        ));
        let chaos_id = StableId::new("chaos").to_string();
        let mission_id = StableId::new("mission").to_string();
        let dogfood_id = StableId::new("dogfood").to_string();
        let finding_id = StableId::new("dogfinding").to_string();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
            db.save_chaos_experiment(&ChaosExperimentRow {
                id: chaos_id.clone(),
                gate_id: "P24-G5".to_string(),
                test_id: "ACCEPT-P24_WORKER_DEATH".to_string(),
                mission_id: mission_id.clone(),
                fault_kind: "worker_death".to_string(),
                expected_recovery: "RECOVER_AUTOMATICALLY".to_string(),
                seed: 42,
                runs: 3,
                passes: 3,
                final_result: "recovered without silent corruption".to_string(),
                state_equivalent: true,
                unresolved_failures: String::new(),
                created_at_ms: 1,
            })
            .unwrap();
            db.save_chaos_recovery_event(&ChaosRecoveryEventRow {
                id: StableId::new("chaosevent").to_string(),
                experiment_id: chaos_id.clone(),
                sequence_no: 1,
                phase: "recover".to_string(),
                observed_behavior: "lease expired".to_string(),
                recovery_action: "replacement worker".to_string(),
                evidence_ref: "ev-chaos".to_string(),
                created_at_ms: 2,
            })
            .unwrap();
            db.save_chaos_report(&ChaosReportRow {
                id: StableId::new("chaosreport").to_string(),
                scope: "phase-24".to_string(),
                experiments: 1,
                recovered: 1,
                recovery_percent: 100,
                unresolved_failures: String::new(),
                regression_list: String::new(),
                created_at_ms: 3,
            })
            .unwrap();
            db.save_dogfood_mission(&DogfoodMissionRow {
                id: dogfood_id.clone(),
                repository_id: "repo-agentcode".to_string(),
                repository_path: "/repo".to_string(),
                mission_kind: "bug_fix".to_string(),
                objective: "repair contained bug".to_string(),
                status: "verified".to_string(),
                change_set_id: Some("cs-a".to_string()),
                verification_report_id: Some("verify-a".to_string()),
                evidence_refs: "ev-dogfood".to_string(),
                human_interventions: 0,
                provider_switches: 1,
                worker_replacements: 1,
                context_compactions: 0,
                verifier_rejections: 0,
                token_total: 1200,
                paid_cost_micros: 0,
                wall_time_ms: 15000,
                created_at_ms: 4,
            })
            .unwrap();
            db.save_dogfood_finding(&DogfoodFindingRow {
                id: finding_id.clone(),
                mission_id: dogfood_id.clone(),
                severity: "medium".to_string(),
                title: "self improvement".to_string(),
                evidence_refs: "ev-dogfood".to_string(),
                status: "proposed".to_string(),
            })
            .unwrap();
            db.save_dogfood_proposal(&DogfoodProposalRow {
                id: StableId::new("dogproposal").to_string(),
                mission_id: dogfood_id.clone(),
                finding_id,
                summary: "normal mission proposal".to_string(),
                affected_files: "crates/ac-agent/src/lib.rs".to_string(),
                change_set_id: "cs-a".to_string(),
                decision: "accepted".to_string(),
                reason: "verified".to_string(),
            })
            .unwrap();
            db.save_dogfood_report(&DogfoodReportRow {
                id: StableId::new("dogreport").to_string(),
                scope: "phase-25".to_string(),
                missions_executed: 1,
                findings: 1,
                accepted_improvements: 1,
                rejected_proposals: 0,
                regressions: String::new(),
                recommendations: "continue".to_string(),
                created_at_ms: 5,
            })
            .unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let chaos = db.chaos_experiments().unwrap();
            assert_eq!(chaos[0].passes, 3);
            assert!(chaos[0].state_equivalent);
            assert_eq!(db.chaos_recovery_events(&chaos_id).unwrap()[0].phase, "recover");
            assert_eq!(db.chaos_reports().unwrap()[0].recovery_percent, 100);
            let missions = db.dogfood_missions().unwrap();
            assert_eq!(missions[0].status, "verified");
            assert_eq!(db.dogfood_findings(&dogfood_id).unwrap()[0].severity, "medium");
            assert_eq!(db.dogfood_proposals(&dogfood_id).unwrap()[0].decision, "accepted");
            assert_eq!(db.dogfood_reports().unwrap()[0].accepted_improvements, 1);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase26_27_security_and_release_state_survive_reopen() {
        let path = std::env::temp_dir().join(format!(
            "agentcode-phase26-27-{}.sqlite",
            StableId::new("db")
        ));
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
            db.save_dependency_audit(&DependencyAuditRow {
                id: "dep-a".to_string(),
                name: "rusqlite".to_string(),
                version: "0.31.0".to_string(),
                license: "MIT".to_string(),
                source: "crates.io".to_string(),
                checksum: "sha256:abc".to_string(),
                security_status: "reviewed".to_string(),
                vulnerability_refs: String::new(),
                release_blocking: false,
                created_at_ms: 1,
            })
            .unwrap();
            db.save_hardening_report(&HardeningReportRow {
                id: "hardening-a".to_string(),
                report_type: "production-security".to_string(),
                findings: "none critical".to_string(),
                mitigations: "redaction verified".to_string(),
                unresolved_risks: String::new(),
                accepted_limitations: "manual notarization credentials unavailable".to_string(),
                release_blocked: false,
                created_at_ms: 2,
            })
            .unwrap();
            db.save_release_build(&ReleaseBuildRow {
                id: "build-a".to_string(),
                version: "1.0.0".to_string(),
                commit_ref: "commit-a".to_string(),
                build_profile: "release".to_string(),
                environment: "macos-arm64".to_string(),
                reproducible: true,
                created_at_ms: 3,
            })
            .unwrap();
            db.save_release_artifact(&ReleaseArtifactRow {
                id: "artifact-a".to_string(),
                version: "1.0.0".to_string(),
                platform: "macos-arm64".to_string(),
                artifact_kind: "app-bundle".to_string(),
                build_hash: "fnv1a64:build".to_string(),
                integrity_hash: "fnv1a64:artifact".to_string(),
                source_commit: "commit-a".to_string(),
                created_at_ms: 4,
            })
            .unwrap();
            db.save_update_record(&UpdateRecordRow {
                id: "update-a".to_string(),
                current_version: "1.0.0".to_string(),
                available_version: "1.0.1".to_string(),
                decision: "install".to_string(),
                verified: true,
                rollback_ref: Some("rollback:commit-a".to_string()),
                recovery_action: "restore previous artifact".to_string(),
                created_at_ms: 5,
            })
            .unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.dependency_audits().unwrap()[0].license, "MIT");
            assert!(!db.hardening_reports().unwrap()[0].release_blocked);
            assert!(db.release_builds().unwrap()[0].reproducible);
            assert_eq!(db.release_artifacts().unwrap()[0].platform, "macos-arm64");
            assert!(db.update_records().unwrap()[0].verified);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase28_29_release_candidate_and_v1_state_survive_reopen() {
        let path = std::env::temp_dir().join(format!(
            "agentcode-phase28-29-{}.sqlite",
            StableId::new("db")
        ));
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
            db.save_release_candidate(&ReleaseCandidateRow {
                id: "rc-a".to_string(),
                version: "1.0.0-rc.1".to_string(),
                candidate_id: "rc.1".to_string(),
                build_id: "build-a".to_string(),
                commit_hash: "commit-a".to_string(),
                platform_target: "macos-arm64".to_string(),
                validation_status: "accepted".to_string(),
                evidence_refs: "release/rc/manifest.md".to_string(),
                created_at_ms: 1,
            })
            .unwrap();
            db.save_release_validation_run(&ReleaseValidationRunRow {
                id: "validation-a".to_string(),
                candidate_id: "rc-a".to_string(),
                security_status: "pass".to_string(),
                tests_status: "pass".to_string(),
                migration_status: "pass".to_string(),
                artifact_status: "pass".to_string(),
                performance_status: "pass".to_string(),
                release_approval_status: "pass".to_string(),
                evidence_refs: "security,tests,migration,artifact,performance,approval"
                    .to_string(),
                created_at_ms: 2,
            })
            .unwrap();
            db.save_release_approval_decision(&ReleaseApprovalDecisionRow {
                id: "decision-a".to_string(),
                approved_version: "1.0.0".to_string(),
                validation_evidence_refs: "release/validation/final.md".to_string(),
                security_status: "pass".to_string(),
                approval_timestamp_ms: 3,
            })
            .unwrap();
            db.save_final_release_manifest(&FinalReleaseManifestRow {
                id: "manifest-a".to_string(),
                version: "1.0.0".to_string(),
                features: "missions,discuss,design,security".to_string(),
                migrations: "0001..0018".to_string(),
                artifacts: "artifact-a".to_string(),
                checksums: "fnv1a64:artifact".to_string(),
                known_limitations: "notarization prerequisite-bound".to_string(),
                manifest_hash: "fnv1a64:manifest".to_string(),
                created_at_ms: 4,
            })
            .unwrap();
            db.save_release_evidence_bundle(&ReleaseEvidenceBundleRow {
                id: "bundle-a".to_string(),
                version: "1.0.0".to_string(),
                audit_report_ref: "release/audit.md".to_string(),
                security_report_ref: "release/security.md".to_string(),
                validation_report_ref: "release/validation.md".to_string(),
                artifact_report_ref: "release/artifact.md".to_string(),
                migration_report_ref: "release/migration.md".to_string(),
                created_at_ms: 5,
            })
            .unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(
                db.release_candidates().unwrap()[0].validation_status,
                "accepted"
            );
            assert_eq!(
                db.release_validation_runs().unwrap()[0].release_approval_status,
                "pass"
            );
            assert_eq!(
                db.release_approval_decisions().unwrap()[0].approved_version,
                "1.0.0"
            );
            assert_eq!(
                db.final_release_manifests().unwrap()[0].manifest_hash,
                "fnv1a64:manifest"
            );
            assert_eq!(
                db.release_evidence_bundles().unwrap()[0].migration_report_ref,
                "release/migration.md"
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn semantic_vectors_survive_reopen_and_drop_corrupt_rows() {
        let path = std::env::temp_dir().join(format!("agentcode-semantic-{}.sqlite", StableId::new("db")));
        let fact = MemoryFactRow { id: "memory-1".to_string(), repository_id: "repo-1".to_string(), mission_id: None, task_id: None, branch: None, statement: "scheduler is authoritative".to_string(), fact_type: "ARCHITECTURE_FACT".to_string(), source: "TEST".to_string(), confidence: 100, freshness: "FRESH".to_string(), memory_class: "DECISION".to_string(), observed_commit: "abc".to_string(), conflict_set_id: None, valid_from_ms: 1, valid_until_ms: None, superseded_by: None, last_validation_ms: 1 };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_memory_fact(&fact, &[MemoryEvidenceRow { fact_id: fact.id.clone(), evidence_ref: "evidence-1".to_string(), file_path: None, symbol: None, content_hash: None }]).unwrap();
            db.save_semantic_chunk(&SemanticChunkRow { id: "chunk-1".to_string(), repository_id: "repo-1".to_string(), fact_id: fact.id.clone(), content: fact.statement.clone(), content_hash: "hash-1".to_string(), model_id: "fastembed/all-MiniLM-L6-v2".to_string(), dimension: 3, vector: vec![0.1, 0.2, 0.3], freshness: "FRESH".to_string(), created_at_ms: 1 }).unwrap();
        }
        let db = ControlPlaneDb::open(&path).unwrap();
        assert_eq!(db.semantic_chunks("repo-1", "fastembed/all-MiniLM-L6-v2").unwrap()[0].vector, vec![0.1, 0.2, 0.3]);
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

    #[test]
    fn schema_21_repairs_schema_20_acceptance_columns_without_losing_rows() {
        let path = std::env::temp_dir().join(format!(
            "agentcode-schema20-repair-{}.sqlite",
            StableId::new("db")
        ));
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            for sql in [
                include_str!("../../../migrations/0001_kernel_schema.sql"),
                include_str!("../../../migrations/0002_git_worktree_hardening.sql"),
                include_str!("../../../migrations/0003_code_intelligence.sql"),
                include_str!("../../../migrations/0004_semantic_repository_graph.sql"),
                include_str!("../../../migrations/0005_persistent_memory.sql"),
                include_str!("../../../migrations/0006_context_engine.sql"),
                include_str!("../../../migrations/0007_full_autonomy_kernel.sql"),
                include_str!("../../../migrations/0008_advanced_edit_engine.sql"),
                include_str!("../../../migrations/0009_verification_evidence_engine.sql"),
                include_str!("../../../migrations/0010_browser_runtime.sql"),
                include_str!("../../../migrations/0011_extensions_skills_hooks_mcp.sql"),
                include_str!("../../../migrations/0012_baseline_security.sql"),
                include_str!("../../../migrations/0013_advanced_ai_security.sql"),
                include_str!("../../../migrations/0014_discuss_design_modes.sql"),
                include_str!("../../../migrations/0015_desktop_optimization.sql"),
                include_str!("../../../migrations/0016_chaos_dogfood.sql"),
                include_str!("../../../migrations/0017_security_release.sql"),
                include_str!("../../../migrations/0018_release_candidate_v1.sql"),
                include_str!("../../../migrations/0019_daemon_semantic_memory.sql"),
            ] {
                connection.execute_batch(sql).unwrap();
            }
            connection.execute("INSERT INTO missions VALUES ('mission-old', 'upgrade', 'active', 1, 1)", []).unwrap();
            connection.execute("INSERT INTO agent_sessions VALUES ('session-old', 'mission-old', 'running', 1)", []).unwrap();
            connection.execute("INSERT INTO tasks VALUES ('task-old', 'mission-old', 'old task', 'running', '', NULL, 0, 3, 1)", []).unwrap();
            connection.execute("INSERT INTO evidence_records VALUES ('evidence-old', 'TestReport', '{}', 'mem://old', 'hash', 1)", []).unwrap();
            connection.pragma_update(None, "user_version", 20).unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), CURRENT_SCHEMA_VERSION);
            assert_eq!(db.tasks_for_mission("mission-old").unwrap()[0].id, "task-old");
            assert!(table_columns(&db, "tasks").contains(&"acceptance_criteria_json".to_string()));
            let evidence_columns = table_columns(&db, "evidence_records");
            assert!(evidence_columns.contains(&"raw_content".to_string()));
            assert!(evidence_columns.contains(&"model_summary".to_string()));
            assert!(evidence_columns.contains(&"sensitive".to_string()));
        }
        let _ = fs::remove_file(path);
    }

    fn table_columns(db: &ControlPlaneDb, table: &str) -> Vec<String> {
        let mut stmt = db
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        stmt.query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }
}
