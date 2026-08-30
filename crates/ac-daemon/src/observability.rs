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

    /// Safe verification results.  Exposes id, task association, environment,
    /// normalized_result, command (bounded), tool_version, evidence_ref, and
    /// timestamps.  raw_artifact (unbounded command output) is never returned.
    pub fn verification_summary(&self, mission_id: &str) -> AcResult<Value> {
        let mid = StableId::from_existing(mission_id)?;
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
                    "created_at_ms": a.created_at_ms,
                })
            })
            .collect();
        Ok(json!({ "verifications": list, "final_audits": audit_list }))
    }
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