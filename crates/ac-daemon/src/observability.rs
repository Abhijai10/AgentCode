/// ── Safe read-only IPC observability for the desktop UI ────────────────
///
/// Every method:
/// 1. validates mission_id via StableId + DB existence
/// 2. reads authoritative SQLite state (never runtime-only)
/// 3. returns bounded output
/// 4. never exposes secrets, raw evidence, or credential values
/// 5. never fabricates events or associations
/// 6. returns typed errors on invalid input
impl DaemonService {
    /// Full mission snapshot including goal, workspace, state, timestamps,
    /// task counts, current task, progress, failure, and completion info.
    pub fn mission_details(&self, mission_id: &str) -> AcResult<Value> {
        let mid = StableId::from_existing(mission_id)?;
        let mission = self.db.get_mission(&mid)?.ok_or_else(|| {
            AcError::validation("DAEMON-MISSION_NOT_FOUND", "mission does not exist")
        })?;
        let session = self.db.session_for_mission(&mid)?;
        let timestamps = self.db.mission_timestamps(&mid)?;
        let tasks = self.db.tasks_for_mission(mission_id)?;
        let status = self.mission_status(mission_id);
        // Prefer the live coordinator status; otherwise fall back to the
        // persisted session state (paused/queued/running/cancelled/failed/
        // completed) and finally to the mission row.  This matches the
        // existing GetMission resolution so the UI sees one consistent state.
        let state = status
            .as_ref()
            .map(|s| s.state.clone())
            .or_else(|| self.persisted_mission_state(mission_id).ok().flatten())
            .unwrap_or_else(|| mission.state.clone());
        let terminal = is_terminal_status(&state);
        let failure_code = state
            .strip_prefix("failed:")
            .map(|s| s.trim().to_string())
            .or_else(|| {
                if state == "failed" {
                    Some("DAEMON-MISSION_FAILED".to_string())
                } else {
                    None
                }
            });

        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.state == "completed").count();
        let failed = tasks.iter().filter(|t| t.state == "failed").count();
        let current_task = tasks
            .iter()
            .find(|t| t.state == "running")
            .or_else(|| tasks.iter().find(|t| t.state == "ready"))
            .map(|t| t.id.clone());
        let progress = if total > 0 {
            Some(completed as f64 / total as f64)
        } else {
            None
        };

        let audits = self.db.final_audits(mission_id)?;
        let latest_audit = audits.last().cloned();
        let completion = latest_audit.map(|a| {
            json!({
                "passed": a.passed,
                "completion_allowed": a.completion_allowed,
                "created_at_ms": a.created_at_ms,
            })
        });

        let state_counts = {
            let mut counts = json!({});
            for t in &tasks {
                counts[&t.state] = json!(counts.get(&t.state).and_then(Value::as_u64).unwrap_or(0) + 1);
            }
            counts
        };

        Ok(json!({
            "mission_id": mission_id,
            "session_id": session.as_ref().map(|s| s.id.as_str()),
            "state": state,
            "goal": mission.original_goal,
            "workspace_root": session.as_ref().and_then(|s| s.workspace_root.as_deref()),
            "created_at_ms": timestamps.map(|t| t.0),
            "updated_at_ms": timestamps.map(|t| t.1),
            "terminal": terminal,
            "failure_code": failure_code,
            "task_count": total,
            "completed_task_count": completed,
            "failed_task_count": failed,
            "current_task": current_task.as_deref(),
            "progress": progress,
            "state_counts": state_counts,
            "completion": completion,
        }))
    }

    /// Per-task detail with attempts, for every task of a mission.
    pub fn task_details(&self, mission_id: &str) -> AcResult<Value> {
        let mid = StableId::from_existing(mission_id)?;
        if self.db.get_mission(&mid)?.is_none() {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission does not exist",
            ));
        }
        let tasks = self.db.tasks_for_mission(mission_id)?;
        let mut task_list = Vec::new();
        for task in tasks {
            let attempts = self.db.task_attempts(&task.id)?;
            let current_outcome = attempts.last().map(|a| a.outcome.clone());
            let current_age_ms = attempts.last().map(|a| {
                (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0)) - a.created_at_ms
            });
            task_list.push(json!({
                "task_id": task.id,
                "title": task.title,
                "state": task.state,
                "dependencies": task.dependencies_json.split(',')
                    .map(str::trim)
                    .filter(|d| !d.is_empty())
                    .collect::<Vec<_>>(),
                "retry_count": task.retry_count,
                "max_retries": task.max_retries,
                "assigned_worker_id": task.assigned_worker_id,
                "updated_at_ms": task.updated_at_ms,
                "attempts": attempts.into_iter().map(|a| json!({
                    "attempt_id": a.id,
                    "outcome": a.outcome,
                    "failure_class": a.failure_class,
                    "evidence_refs": a.evidence_refs,
                    "created_at_ms": a.created_at_ms,
                })).collect::<Vec<_>>(),
                "current_attempt_outcome": current_outcome,
                "current_attempt_age_ms": current_age_ms,
            }));
        }
        Ok(json!({ "tasks": task_list }))
    }

    /// Bounded mission activity stream.  `limit` is clamped to [1, 200].
    pub fn mission_events(&self, mission_id: &str, limit: Option<usize>) -> AcResult<Value> {
        let mid = StableId::from_existing(mission_id)?;
        if self.db.get_mission(&mid)?.is_none() {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission does not exist",
            ));
        }
        let limit = limit.unwrap_or(100).clamp(1, 200);
        let events = self.db.mission_activity(mission_id, limit)?;
        Ok(json!({
            "events": events.into_iter().map(|e| json!({
                "id": e.id,
                "kind": e.kind,
                "subject_id": e.subject_id,
                "created_at_ms": e.created_at_ms,
                "detail": e.detail,
            })).collect::<Vec<_>>()
        }))
    }

    /// Read-only ChangeSet summary.  Files are sourced from the authoritative
    /// edit_operations table (structured per-file rows).  Raw file contents
    /// are never exposed; only metadata (path, strategy, additions/removals).
    pub fn changeset_summary(&self, mission_id: &str) -> AcResult<Value> {
        let mid = StableId::from_existing(mission_id)?;
        if self.db.get_mission(&mid)?.is_none() {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission does not exist",
            ));
        }
        let changesets = self.db.changesets_for_mission(mission_id, 200)?;
        let mut list = Vec::new();
        for cs in changesets {
            let files = self.db.edit_operations_for_changeset(&cs.id)?;
            list.push(json!({
                "changeset_id": cs.id,
                "state": cs.state,
                "created_at_ms": cs.created_at_ms,
                "applied": matches!(cs.state.as_str(), "Applied" | "Accepted" | "Validating"),
                "files": files.into_iter().map(|f| json!({
                    "path": f.path,
                    "strategy": f.strategy,
                    "additions": f.additions,
                    "removals": f.removals,
                })).collect::<Vec<_>>(),
            }));
        }
        Ok(json!({ "changesets": list }))
    }

    /// Safe evidence summary.  Exposes id, kind, timestamps, content_hash,
    /// safe model_summary (already redacted and bounded), and provenance
    /// metadata.  Raw content is never returned.  Mission association is
    /// derived from the durable reference graph (task attempts, verification
    /// runs, and final audits) — see mission_evidence_ids.
    /// F1: project memory for ANY mode and the UI — repository-scoped facts,
    /// decisions, and task memories persisted by previous missions.  The
    /// repository identity is derived from the project path (same
    /// deterministic identity as mission persistence), so memory is shared
    /// across all modes working on the same project and never leaks across
    /// projects.
    pub fn project_memory_get(&self, project_path: &str) -> AcResult<Value> {
        if project_path.trim().is_empty() {
            return Err(AcError::validation(
                "MEMORY-PROJECT_PATH_REQUIRED",
                "project path is required to read project memory",
            ));
        }
        let identity = crate::project_repository_identity(project_path);
        let facts: Vec<Value> = self
            .db
            .memory_facts_for(&identity, 100)?
            .iter()
            .filter(|row| row.valid_until_ms.is_none())
            .map(|row| {
                json!({
                    "id": row.id,
                    "statement": row.statement,
                    "fact_type": row.fact_type,
                    "source": row.source,
                    "confidence": row.confidence,
                    "freshness": row.freshness,
                    "memory_class": row.memory_class,
                    "last_validation_ms": row.last_validation_ms,
                })
            })
            .collect();
        let decisions: Vec<Value> = self
            .db
            .memory_decisions_for(&identity, 50)?
            .iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "decision": row.decision,
                    "rationale": row.rationale,
                    "created_at_ms": row.created_at_ms,
                })
            })
            .collect();
        let task_memories: Vec<Value> = self
            .db
            .task_memories_newest(50)?
            .iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "task_id": row.task_id,
                    "summary": row.summary,
                    "created_at_ms": row.created_at_ms,
                })
            })
            .collect();
        Ok(json!({
            "project_path": project_path,
            "facts": facts,
            "decisions": decisions,
            "task_memories": task_memories,
            "counts": {
                "facts": facts.len(),
                "decisions": decisions.len(),
                "task_memories": task_memories.len(),
            },
        }))
    }

    /// Honesty inspector (final-audit recommendation): the full WHY chain
    /// for a mission — each task, its attempts, the evidence each attempt
    /// captured, and which final audits consumed which evidence.  This is
    /// the "Why this result?" walk: a user can trace any conclusion back
    /// to the tool output that produced it, with nothing hidden.
    pub fn evidence_chain(&self, mission_id: &str) -> AcResult<Value> {
        let mid = StableId::from_existing(mission_id)?;
        if self.db.get_mission(&mid)?.is_none() {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission does not exist",
            ));
        }
        let tasks = self.db.tasks_for_mission(mission_id)?;
        let audits = self.db.final_audits(mission_id)?;
        let mut chains = Vec::new();
        for task in &tasks {
            let attempts = self.db.task_attempts(&task.id)?;
            let mut attempt_links = Vec::new();
            for attempt in &attempts {
                let refs: Vec<String> = ac_db::split_refs(&attempt.evidence_refs);
                let evidence: Vec<Value> = if refs.is_empty() {
                    Vec::new()
                } else {
                    self.db
                        .evidence_records_by_ids(&refs)?
                        .into_iter()
                        .map(|r| {
                            json!({
                                "evidence_id": r.id.to_string(),
                                "kind": format!("{:?}", r.kind),
                                "source": r.provenance.source,
                                "tool": r.provenance.tool,
                                "summary": r.model_summary
                                    .as_deref()
                                    .map(|s| bounded_ui_summary(s, 160)),
                                "content_hash": r.content_hash,
                            })
                        })
                        .collect()
                };
                attempt_links.push(json!({
                    "attempt_id": attempt.id,
                    "outcome": attempt.outcome,
                    "failure_class": attempt.failure_class,
                    "evidence": evidence,
                }));
            }
            chains.push(json!({
                "task_id": task.id,
                "title": task.title,
                "state": task.state,
                "attempts": attempt_links,
            }));
        }
        // Final audits: the requirements they checked against, their
        // verdict, and the finding codes recorded with them (the audit row
        // stores finding codes, not evidence ids — report that honestly
        // rather than relabeling it 'consumed evidence').
        let audit_links: Vec<Value> = audits
            .iter()
            .map(|a| {
                let finding_codes: Vec<String> = ac_db::split_refs(&a.evidence_refs);
                json!({
                    "audit_id": a.id,
                    "passed": a.passed,
                    "completion_allowed": a.completion_allowed,
                    "remaining_uncertainty": a.remaining_uncertainty,
                    "finding_codes": finding_codes,
                    "created_at_ms": a.created_at_ms,
                })
            })
            .collect();
        Ok(json!({
            "mission_id": mission_id,
            "tasks": chains,
            "final_audits": audit_links,
        }))
    }

    pub fn evidence_summary(&self, mission_id: &str) -> AcResult<Value> {
        let mid = StableId::from_existing(mission_id)?;
        if self.db.get_mission(&mid)?.is_none() {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission does not exist",
            ));
        }
        let ids = self.db.mission_evidence_ids(mission_id)?;
        let records = self.db.evidence_records_by_ids(&ids)?;
        let mut list = Vec::new();
        for record in records {
            let kind_str = format!("{:?}", record.kind);
            let summary = record.model_summary.as_deref().map(|s| bounded_ui_summary(s, 1024));
            // Never expose raw_content.  Respect existing redaction: the
            // model_summary is already bounded and secrets-replaced by the
            // evidence layer's bound_summary/redact.
            list.push(json!({
                "evidence_id": record.id.to_string(),
                "kind": kind_str,
                "created_at_ms": record.created_at.as_millis() as i64,
                "content_hash": record.content_hash,
                "sensitive": record.sensitive,
                "summary": summary,
                "provenance_source": record.provenance.source,
                "provenance_tool": record.provenance.tool,
            }));
        }
        Ok(json!({ "evidence": list }))
    }

    /// Batch N6 (G6): export a mission's full result context as a
    /// self-contained Markdown document — goal, state, task DAG with
    /// attempts, activity events, ChangeSets with files, evidence
    /// summaries (already redacted and bounded — raw content never
    /// leaves the evidence layer), and the final verification audit.
    /// The file is written through the GOVERNED ChangeSet path
    /// (EditEngine prepare → validate → approve → apply with journal,
    /// rollback plan, and content-hash preconditions) — never a bare
    /// filesystem write — into the mission's workspace root.
    pub fn mission_export(&self, mission_id: &str) -> AcResult<Value> {
        let details = self.mission_details(mission_id)?;
        let tasks = self.task_details(mission_id)?;
        let events = self.mission_events(mission_id, Some(200))?;
        let changesets = self.changeset_summary(mission_id)?;
        let evidence = self.evidence_summary(mission_id)?;

        let goal = details["goal"].as_str().unwrap_or("");
        let state = details["state"].as_str().unwrap_or("unknown");
        let workspace = details["workspace_root"].as_str().unwrap_or("");
        if workspace.is_empty() {
            return Err(AcError::validation(
                "MISSION-EXPORT_NO_WORKSPACE",
                "mission has no workspace root; cannot export",
            ));
        }

        let mut doc = String::new();
        doc.push_str("# Mission Export\n\n");
        doc.push_str(&format!("**Mission:** `{mission_id}`\n\n"));
        doc.push_str(&format!("**State:** {state}\n\n"));
        if let Some(created) = details["created_at_ms"].as_i64() {
            doc.push_str(&format!(
                "**Created:** {}\n\n",
                // Millis since epoch; rendered as a number — no TZ guessing.
                created
            ));
        }
        doc.push_str("## Goal\n\n```\n");
        // Goal text came from the user; bound it defensively.
        doc.push_str(&bounded_ui_summary(goal, 4096));
        doc.push_str("\n```\n\n");

        // Tasks with attempts.
        doc.push_str("## Tasks\n\n");
        let task_list = tasks["tasks"].as_array().cloned().unwrap_or_default();
        if task_list.is_empty() {
            doc.push_str("_No tasks recorded._\n\n");
        }
        for task in &task_list {
            let title = task["title"].as_str().unwrap_or("");
            let t_state = task["state"].as_str().unwrap_or("unknown");
            let retries = task["retry_count"].as_u64().unwrap_or(0);
            let attempts = task["attempts"].as_array().cloned().unwrap_or_default();
            doc.push_str(&format!(
                "- **{title}** — {t_state} ({} attempt(s), {retries} retry(ies))\n",
                attempts.len()
            ));
            for attempt in attempts.iter().take(3) {
                let outcome = attempt["outcome"].as_str().unwrap_or("unknown");
                let class = attempt["failure_class"].as_str().unwrap_or("-");
                doc.push_str(&format!("  - attempt: {outcome} (class: {class})\n"));
            }
        }
        doc.push('\n');

        // Activity events (bounded).
        doc.push_str("## Activity (last events)\n\n```\n");
        let event_list = events["events"].as_array().cloned().unwrap_or_default();
        if event_list.is_empty() {
            doc.push_str("(no events)\n");
        }
        for event in event_list.iter().take(50) {
            let kind = event.get("kind").and_then(Value::as_str).unwrap_or("event");
            let summary = event
                .get("summary")
                .or_else(|| event.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("");
            doc.push_str(&format!("- {kind}: {}\n", bounded_ui_summary(summary, 200)));
        }
        doc.push_str("```\n\n");

        // ChangeSets with files.
        doc.push_str("## ChangeSets\n\n");
        let cs_list = changesets["changesets"].as_array().cloned().unwrap_or_default();
        if cs_list.is_empty() {
            doc.push_str("_No changesets._\n\n");
        }
        for cs in &cs_list {
            let cs_id = cs["changeset_id"].as_str().unwrap_or("?");
            let cs_state = cs["state"].as_str().unwrap_or("?");
            doc.push_str(&format!("- `{cs_id}` — {cs_state}\n"));
            for file in cs["files"].as_array().cloned().unwrap_or_default() {
                let path = file["path"].as_str().unwrap_or("?");
                let add = file["additions"].as_u64().unwrap_or(0);
                let rem = file["removals"].as_u64().unwrap_or(0);
                doc.push_str(&format!("  - `{path}` (+{add}/-{rem})\n"));
            }
        }
        doc.push('\n');

        // Evidence summaries (already redacted/bounded upstream).
        doc.push_str("## Evidence\n\n");
        let ev_list = evidence["evidence"].as_array().cloned().unwrap_or_default();
        if ev_list.is_empty() {
            doc.push_str("_No evidence recorded._\n\n");
        }
        for ev in ev_list.iter().take(40) {
            let id = ev["evidence_id"].as_str().unwrap_or("?");
            let kind = ev["kind"].as_str().unwrap_or("?");
            let summary = ev["summary"].as_str().unwrap_or("(no summary)");
            let sensitive = ev["sensitive"].as_bool().unwrap_or(false);
            doc.push_str(&format!(
                "- `{id}` ({kind}){}: {}\n",
                if sensitive { " [sensitive]" } else { "" },
                bounded_ui_summary(summary, 200)
            ));
        }
        doc.push('\n');

        // Final verification audit.
        doc.push_str("## Verification\n\n");
        if let Some(completion) = details["completion"].as_object() {
            let passed = completion.get("passed").and_then(Value::as_bool);
            let allowed = completion
                .get("completion_allowed")
                .and_then(Value::as_bool);
            doc.push_str(&format!(
                "Final audit: passed={:?}, completion_allowed={:?}\n\n",
                passed, allowed
            ));
        } else {
            doc.push_str("_No final audit recorded._\n\n");
        }
        doc.push_str(
            "---\n_Generated by AgentCode mission export. SQLite remains the authoritative source; this file is a derived snapshot._\n",
        );

        // Governed write through the ChangeSet path.
        let rel_path = "MISSION_EXPORT.md".to_string();
        let mut repo = ac_changeset::LocalWorkspaceFileRepository::new(
            std::path::PathBuf::from(workspace),
            "daemon",
        );
        let expected_hash = <ac_changeset::LocalWorkspaceFileRepository as ac_changeset::FileRepository>::read(&repo, &rel_path)
            .ok()
            .map(|current| ac_changeset::content_hash(&current));
        let engine = ac_changeset::EditEngine;
        let mut transaction = engine
            .prepare(
                &repo,
                vec![ac_changeset::EditRequest {
                    path: rel_path.clone(),
                    precondition: ac_changeset::EditPrecondition {
                        path: rel_path.clone(),
                        expected_hash,
                        base_revision: "daemon".to_string(),
                        symbol_fingerprint: None,
                    },
                    strategy: ac_changeset::EditStrategy::WholeFile {
                        content: doc.clone(),
                    },
                }],
            )
            .map_err(|error| {
                AcError::validation(
                    "MISSION-EXPORT_TRANSACTION",
                    format!("governed transaction failed to prepare: {error}"),
                )
            })?;
        transaction.changeset
            .attach_metadata(ac_changeset::ChangeSetMetadata {
                originating_task: StableId::new("missionexport"),
                originating_agent_session: StableId::new("daemon"),
                files_changed: vec![ac_changeset::FileChangeSummary {
                    path: rel_path.clone(),
                    additions: doc.lines().count() as u32,
                    removals: 0,
                }],
                additions: doc.lines().count() as u32,
                removals: 0,
                evidence_refs: Vec::new(),
                verification_passed: Some(true),
            })
            .map_err(|error| {
                AcError::validation(
                    "MISSION-EXPORT_TRANSACTION",
                    format!("governed transaction metadata failed: {error}"),
                )
            })?;
        transaction.changeset.validate().map_err(|error| {
            AcError::validation(
                "MISSION-EXPORT_TRANSACTION",
                format!("governed transaction failed validation: {error}"),
            )
        })?;
        transaction.changeset.approve().map_err(|error| {
            AcError::validation(
                "MISSION-EXPORT_TRANSACTION",
                format!("governed transaction approval failed: {error}"),
            )
        })?;
        engine
            .apply(&mut repo, &mut transaction)
            .map_err(|error| {
                AcError::validation(
                    "MISSION-EXPORT_TRANSACTION",
                    format!("governed write failed and rolled back: {error}"),
                )
            })?;

        Ok(json!({
            "exported": true,
            "mission_id": mission_id,
            "path": std::path::Path::new(workspace).join("MISSION_EXPORT.md").to_string_lossy(),
            "changeset_id": transaction.changeset.id.to_string(),
            "content_lines": doc.lines().count(),
            "note": "governed write through the ChangeSet path; evidence summaries are redacted upstream",
        }))
    }

    /// Safe verification results.  Exposes id, task association, environment,
    /// normalized_result, command (bounded), tool_version, evidence_ref, and
    /// timestamps.  raw_artifact (unbounded command output) is never returned.
    pub fn verification_summary(&self, mission_id: &str) -> AcResult<Value> {        let mid = StableId::from_existing(mission_id)?;
        if self.db.get_mission(&mid)?.is_none() {
            return Err(AcError::validation(
                "DAEMON-MISSION_NOT_FOUND",
                "mission does not exist",
            ));
        }
        let runs = self.db.verification_runs_for_mission(mission_id, 200)?;
        let mut list = Vec::new();
        for run in runs {
            // Command is safe metadata (the verification command string).
            // Raw_artifact is unbounded command output and is never exposed.
            let command = bounded_ui_summary(&run.command, 512);
            let passed = run.normalized_result == "Passed";
            list.push(json!({
                "verification_id": run.id,
                "task_id": run.task_id,
                "commit_ref": run.commit_ref,
                "environment": run.environment,
                "command": command,
                "tool_version": run.tool_version,
                "status": run.normalized_result,
                "passed": passed,
                "evidence_ref": run.evidence_ref,
                "created_at_ms": run.created_at_ms,
            }));
        }
        // Include final audits for completion gate info.
        let audits = self.db.final_audits(mission_id)?;
        let audit_list: Vec<Value> = audits
            .into_iter()
            .map(|a| {
                json!({
                    "audit_id": a.id,
                    "passed": a.passed,
                    "completion_allowed": a.completion_allowed,
                    "remaining_uncertainty": a.remaining_uncertainty,
                    "created_at_ms": a.created_at_ms,
                })
            })
            .collect();
        Ok(json!({ "verifications": list, "final_audits": audit_list }))
    }

    /// Mission→conversation projection: the structured execution activity for
    /// every mission referenced by this conversation's messages.  All data is
    /// read from authoritative backend state (missions, tasks, attempts,
    /// events, changesets, evidence, verification) — nothing is fabricated.
    /// A message whose `mission_ref` points at a real mission yields a block;
    /// messages without a mission reference are plain chat and are skipped.
    /// Doc 06 H4: the event-push subscribe protocol.  Long-poll semantics:
    /// returns immediately when events exist after the caller's cursor;
    /// otherwise holds the request (bounded by wait_ms, polled server-side
    /// at 250ms) until a new kernel event lands.  The UI therefore issues
    /// ONE cheap blocked request instead of hammering projections on a
    /// timer — idle cost drops to zero per 10s window.
    pub fn events_subscribe(
        &self,
        after_created_at_ms: i64,
        after_id: &str,
        wait_ms: u64,
    ) -> AcResult<Value> {
        const MAX_WAIT_MS: u64 = 10_000;
        const POLL_INTERVAL_MS: u64 = 250;
        const MAX_EVENTS: usize = 200;
        let bounded_wait = wait_ms.min(MAX_WAIT_MS);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(bounded_wait);
        let (mut cursor_ms, mut cursor_id) = (after_created_at_ms, after_id.to_string());
        // First subscriber without a cursor starts at the CURRENT tail: it
        // gets events from NOW on, never the full history replay (the
        // caller fetches a snapshot separately per H4).
        if cursor_ms < 0 {
            let (ms, id) = self.db.kernel_events_tail_cursor()?;
            cursor_ms = ms;
            cursor_id = id;
        }
        let events = loop {
            let batch = self.db.kernel_events_after(cursor_ms, &cursor_id, MAX_EVENTS)?;
            if !batch.is_empty() {
                break batch;
            }
            if std::time::Instant::now() >= deadline {
                break Vec::new();
            }
            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));
        };
        let items: Vec<Value> = events
            .iter()
            .map(|event| {
                json!({
                    "id": event.id,
                    "decision_kind": event.decision_kind,
                    "subject_id": event.subject_id,
                    "created_at_ms": event.created_at_ms,
                    "evidence_refs": event.evidence_refs,
                })
            })
            .collect();
        let (tail_ms, tail_id) = match events.last() {
            Some(last) => (last.created_at_ms, last.id.clone()),
            None => self.db.kernel_events_tail_cursor()?,
        };
        Ok(json!({
            "events": items,
            "cursor": {"created_at_ms": tail_ms, "id": tail_id},
            "waited": events.is_empty(),
        }))
    }

    /// F11 (final audit): a CHEAP cursor for change detection.  Polling the
    /// heavy ConversationActivity projection every 2.5s recomputes six
    /// mission projections; views poll this instead and only refetch the
    /// full activity when the cursor changes.  The cursor is content-based
    /// (message count, latest message id, per-mission state + latest event
    /// id), so any state change moves it.
    pub fn conversation_changes_cursor(&self, conversation_id: &str) -> AcResult<Value> {
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let messages = self.db.messages_for_conversation(conversation_id)?;
        let latest_message_id = messages.last().map(|m| m.id.clone()).unwrap_or_default();
        let mut mission_states: Vec<Value> = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for message in &messages {
            let Some(mission_ref) = &message.mission_ref else {
                continue;
            };
            if !seen.insert(mission_ref.clone()) {
                continue;
            }
            let mid = match StableId::from_existing(mission_ref) {
                Ok(id) => id,
                Err(_) => continue,
            };
            if self.db.get_mission(&mid)?.is_none() {
                continue;
            }
            let state = self
                .mission_status(mission_ref)
                .map(|s| json!({"state": s.state}))
                .unwrap_or(Value::Null);
            let latest_event = self
                .db
                .mission_activity(mission_ref, 1)?
                .first()
                .map(|e| json!({"id": e.id, "at_ms": e.created_at_ms}))
                .unwrap_or(Value::Null);
            mission_states.push(json!({"mission_id": mission_ref, "state": state, "latest_event": latest_event}));
        }
        Ok(json!({
            "conversation_id": conversation_id,
            "updated_at_ms": conv.updated_at_ms,
            "message_count": messages.len(),
            "latest_message_id": latest_message_id,
            "missions": mission_states,
        }))
    }

    pub fn conversation_activity(&self, conversation_id: &str) -> AcResult<Value> {
        let conv = self.db.conversation(conversation_id)?.ok_or_else(|| {
            AcError::validation("CONVERSATION-NOT_FOUND", "conversation not found")
        })?;
        let messages = self.db.messages_for_conversation(conversation_id)?;
        let mut seen = std::collections::BTreeSet::new();
        let mut blocks = Vec::new();
        for message in messages {
            let Some(mission_ref) = message.mission_ref.clone() else {
                continue;
            };
            // A conversation may reference the same mission from multiple
            // messages (follow-ups, retries).  Project each mission once.
            if !seen.insert(mission_ref.clone()) {
                continue;
            }
            let mid = match StableId::from_existing(&mission_ref) {
                Ok(id) => id,
                Err(_) => continue,
            };
            if self.db.get_mission(&mid)?.is_none() {
                continue;
            }
            let details = self.mission_details(&mission_ref).unwrap_or_else(|_| json!({}));
            let tasks = self.task_details(&mission_ref).unwrap_or_else(|_| json!({}));
            let events = self.mission_events(&mission_ref, Some(200)).unwrap_or_else(|_| json!({}));
            let changesets = self.changeset_summary(&mission_ref).unwrap_or_else(|_| json!({}));
            let evidence = self.evidence_summary(&mission_ref).unwrap_or_else(|_| json!({}));
            let verification = self.verification_summary(&mission_ref).unwrap_or_else(|_| json!({}));
            let status = self.mission_status(&mission_ref);
            let summary = mission_activity_summary(&tasks, &changesets, &evidence, &verification);
            let provider_models = self
                .db
                .provider_model_records_for_mission(&mission_ref)
                .unwrap_or_default();
            let remaining_uncertainty = verification
                .get("final_audits")
                .and_then(Value::as_array)
                .and_then(|audits| audits.last())
                .and_then(|audit| audit.get("remaining_uncertainty"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            blocks.push(json!({
                "mission_id": mission_ref,
                "conversation_id": conversation_id,
                "status": status.map(|s| s.state.clone()),
                "details": details,
                "tasks": tasks,
                "events": events,
                "changesets": changesets,
                "evidence": evidence,
                "verification": verification,
                "summary": summary,
                "provider_models": provider_models.into_iter().map(|pm| json!({
                    "provider_id": pm.provider_id,
                    "provider_account_id": pm.provider_account_id,
                    "model_id": pm.model_id,
                    "model_name": pm.model_name,
                    "routing_mode": pm.routing_mode,
                    "attempt_number": pm.attempt_number,
                    "success": pm.success,
                    "failure_class": pm.failure_class,
                    "created_at_ms": pm.created_at_ms,
                })).collect::<Vec<_>>(),
                "remaining_uncertainty": remaining_uncertainty,
            }));
        }
        Ok(json!({
            "conversation_id": conversation_id,
            "project_path": conv.project_path,
            "missions": blocks,
        }))
    }
}

/// Derive a factual execution summary from authoritative persisted state:
/// which tools were used, which commands/tests ran, which files changed,
/// whether any failure occurred, and how much recovery (retries) happened.
/// Nothing is synthesized beyond what the persisted rows contain.
fn mission_activity_summary(
    tasks: &Value,
    changesets: &Value,
    evidence: &Value,
    verification: &Value,
) -> Value {
    // Tools used: provenance.tool on evidence records + verification tool_version.
    let mut tools = std::collections::BTreeSet::new();
    if let Some(list) = evidence.get("evidence").and_then(Value::as_array) {
        for item in list {
            if let Some(tool) = item.get("provenance_tool").and_then(Value::as_str) {
                if !tool.trim().is_empty() {
                    tools.insert(tool.to_string());
                }
            }
        }
    }
    if let Some(list) = verification.get("verifications").and_then(Value::as_array) {
        for item in list {
            if let Some(tool) = item.get("tool_version").and_then(Value::as_str) {
                if !tool.trim().is_empty() {
                    tools.insert(tool.to_string());
                }
            }
        }
    }

    // Commands / tests run: verification run command strings (bounded).
    let mut commands = Vec::new();
    if let Some(list) = verification.get("verifications").and_then(Value::as_array) {
        for item in list {
            if let Some(command) = item.get("command").and_then(Value::as_str) {
                commands.push(bounded_ui_summary(command, 512));
            }
        }
    }

    // Files changed: edit_operations paths from changesets.
    let mut files = Vec::new();
    if let Some(list) = changesets.get("changesets").and_then(Value::as_array) {
        for changeset in list {
            if let Some(file_list) = changeset.get("files").and_then(Value::as_array) {
                for file in file_list {
                    if let Some(path) = file.get("path").and_then(Value::as_str) {
                        if !files.iter().any(|existing: &String| existing == path) {
                            files.push(path.to_string());
                        }
                    }
                }
            }
        }
    }

    // Failures: task attempts with a failure_class.
    let mut failure_classes = std::collections::BTreeSet::new();
    let mut failed_task_count = 0;
    if let Some(list) = tasks.get("tasks").and_then(Value::as_array) {
        for task in list {
            if let Some(attempts) = task.get("attempts").and_then(Value::as_array) {
                for attempt in attempts {
                    if let Some(failure) = attempt.get("failure_class").and_then(Value::as_str) {
                        if !failure.trim().is_empty() {
                            failure_classes.insert(failure.to_string());
                            failed_task_count += 1;
                        }
                    }
                }
            }
        }
    }

    // Recovery: total retries across tasks (retry_count > 0 proves a retry).
    let mut retry_count = 0_i64;
    if let Some(list) = tasks.get("tasks").and_then(Value::as_array) {
        for task in list {
            retry_count += task.get("retry_count").and_then(Value::as_i64).unwrap_or(0);
        }
    }

    json!({
        "tools_used": tools.into_iter().collect::<Vec<_>>(),
        "commands": commands,
        "files_changed": files,
        "failure_classes": failure_classes.into_iter().collect::<Vec<_>>(),
        "failed_task_count": failed_task_count,
        "retry_count": retry_count,
    })
}

/// UTF-8-safe bounded summary helper (mirrors the evidence crate's
/// character-boundary-safe truncation for model-facing text).
fn bounded_ui_summary(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }
    let boundary = value
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= limit)
        .last()
        .unwrap_or(0);
    format!("{}…[+{}]", &value[..boundary], value.len() - boundary)
}