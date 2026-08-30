use std::collections::BTreeSet;

/// Kernel event row projection: (id, decision_kind, subject_id, created_at_ms,
/// evidence_refs).
type KernelEventProjection = (String, String, String, i64, String);

impl ControlPlaneDb {
    /// Mission-level timestamps (created_at_ms, updated_at_ms) from the
    /// authoritative missions row.  `updated_at_ms` is not exposed by
    /// `get_mission`, so this is the dedicated observability read.
    pub fn mission_timestamps(&self, mission_id: &StableId) -> AcResult<Option<(i64, i64)>> {
        self.connection
            .query_row(
                "SELECT created_at_ms, updated_at_ms FROM missions WHERE id = ?1",
                params![mission_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(db_error)
    }

    /// The full set of persisted task ids belonging to a mission.
    pub fn mission_task_ids(&self, mission_id: &str) -> AcResult<Vec<String>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id FROM tasks WHERE mission_id = ?1 ORDER BY id")
            .map_err(db_error)?;
        let rows = stmt.query_map(params![mission_id], |row| row.get(0)).map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// Persisted worktree ids owned by a mission (used to associate changesets
    /// and verification runs that reference a worktree instead of a task).
    pub fn mission_worktree_ids(&self, mission_id: &str) -> AcResult<Vec<String>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id FROM worktrees WHERE owner_mission_id = ?1 ORDER BY id",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![mission_id], |row| row.get(0))
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// Changesets associated with a mission, discovered through the durable
    /// edit-transaction graph (edit_transactions → tasks/worktrees).  A
    /// changeset is authoritative only when it was reached through the edit
    /// engine apply path, which is exactly the production mutation path.
    pub fn changesets_for_mission(
        &self,
        mission_id: &str,
        limit: usize,
    ) -> AcResult<Vec<PersistedChangeSet>> {
        let task_ids = self.mission_task_ids(mission_id)?;
        let worktree_ids = self.mission_worktree_ids(mission_id)?;
        if task_ids.is_empty() && worktree_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut placeholders = String::new();
        let mut params_vec: Vec<String> = Vec::new();
        for id in task_ids.iter().chain(worktree_ids.iter()) {
            if !placeholders.is_empty() {
                placeholders.push(',');
            }
            placeholders.push('?');
            params_vec.push(id.clone());
        }
        // The task_id and worktree_id IN clauses each consume a full set of
        // placeholders, so the bound parameters must be duplicated for the
        // second IN clause.
        for id in task_ids.iter().chain(worktree_ids.iter()) {
            params_vec.push(id.clone());
        }
        let query = format!(
            "SELECT DISTINCT c.id, c.state, c.operations_json, c.metadata_json, c.rollback_json, c.created_at_ms
             FROM changesets c
             JOIN edit_transactions et ON et.changeset_id = c.id
             WHERE et.task_id IN ({placeholders}) OR et.worktree_id IN ({placeholders})
             ORDER BY c.created_at_ms DESC
             LIMIT ?"
        );
        params_vec.push(limit.to_string());
        let param_refs = params_vec
            .iter()
            .map(|value| value as &dyn rusqlite::types::ToSql)
            .collect::<Vec<_>>();
        let mut stmt = self.connection.prepare(&query).map_err(db_error)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
                Ok(PersistedChangeSet {
                    id: row.get(0)?,
                    state: row.get(1)?,
                    operations_json: row.get(2)?,
                    metadata_json: row.get(3)?,
                    rollback_json: row.get(4)?,
                    created_at_ms: row.get(5)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// Structured affected-file rows for a changeset from the durable
    /// edit-operations graph (authoritative structured paths/strategy).
    pub fn edit_operations_for_changeset(
        &self,
        changeset_id: &str,
    ) -> AcResult<Vec<EditOperationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT eo.id, eo.transaction_id, eo.path, eo.strategy, eo.before_hash,
                        eo.after_hash, eo.symbol_fingerprint, eo.additions, eo.removals
                 FROM edit_operations eo
                 JOIN edit_transactions et ON et.id = eo.transaction_id
                 WHERE et.changeset_id = ?1
                 ORDER BY eo.path ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![changeset_id], |row| {
                Ok(EditOperationRow {
                    id: row.get(0)?,
                    transaction_id: row.get(1)?,
                    path: row.get(2)?,
                    strategy: row.get(3)?,
                    before_hash: row.get(4)?,
                    after_hash: row.get(5)?,
                    symbol_fingerprint: row.get(6)?,
                    additions: row.get(7)?,
                    removals: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// Evidence ids reachable from a mission through the persisted reference
    /// graph: task attempts, verification runs, and the final audit.  These
    /// are real persisted references — nothing is fabricated.  Sensitive
    /// evidence rows remain rows; only the ids are returned here.
    pub fn mission_evidence_ids(&self, mission_id: &str) -> AcResult<Vec<String>> {
        let mut ids = BTreeSet::new();
        for task_id in self.mission_task_ids(mission_id)? {
            for attempt in self.task_attempts(&task_id)? {
                for ref_id in split_refs(&attempt.evidence_refs) {
                    ids.insert(ref_id);
                }
            }
        }
        for run in self.verification_runs_for_mission(mission_id, 200)? {
            if !run.evidence_ref.trim().is_empty() {
                ids.insert(run.evidence_ref.clone());
            }
        }
        for audit in self.final_audits(mission_id)? {
            for ref_id in split_refs(&audit.evidence_refs) {
                ids.insert(ref_id);
            }
        }
        Ok(ids.into_iter().collect())
    }

    /// Load evidence records by id list (bounded by the caller's collected
    /// reference set).  Raw content is included for non-sensitive records only
    /// so the caller can decide whether to expose it; sensitive records never
    /// carry raw content (they were never durably persisted with it).
    pub fn evidence_records_by_ids(&self, ids: &[String]) -> AcResult<Vec<EvidenceRecord>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = (0..ids.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT id, kind, provenance_json, artifact_uri, content_hash, created_at_ms,
                    raw_content, model_summary, sensitive
             FROM evidence_records
             WHERE id IN ({placeholders})
             ORDER BY created_at_ms ASC, id ASC"
        );
        let param_refs = ids
            .iter()
            .map(|value| value as &dyn rusqlite::types::ToSql)
            .collect::<Vec<_>>();
        let mut stmt = self.connection.prepare(&query).map_err(db_error)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
                let id: String = row.get(0)?;
                let kind_json: String = row.get(1)?;
                let provenance_json: String = row.get(2)?;
                let created_at_ms: i64 = row.get(5)?;
                let kind = serde_json::from_str::<EvidenceKind>(&kind_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                let provenance =
                    serde_json::from_str::<Provenance>(&provenance_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(EvidenceRecord {
                    id: StableId::from_existing(&id).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
                    kind,
                    provenance,
                    artifact_uri: row.get(3)?,
                    content_hash: row.get(4)?,
                    raw_content: row.get(6)?,
                    model_summary: row.get(7)?,
                    sensitive: row.get::<_, i64>(8)? != 0,
                    created_at: TimestampMillis::from_millis(created_at_ms as u128),
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// Verification runs reachable from a mission through its task ids
    /// (the verification engine always associates a run with a task).
    pub fn verification_runs_for_mission(
        &self,
        mission_id: &str,
        limit: usize,
    ) -> AcResult<Vec<VerificationRunRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT vr.id, vr.profile_id, vr.task_id, vr.commit_ref, vr.worktree_id,
                        vr.environment, vr.command, vr.tool_version, vr.normalized_result,
                        vr.raw_artifact, vr.evidence_ref, vr.freshness_dependencies, vr.created_at_ms
                 FROM verification_runs vr
                 JOIN tasks t ON t.id = vr.task_id
                 WHERE t.mission_id = ?1
                 ORDER BY vr.created_at_ms DESC
                 LIMIT ?2",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![mission_id, limit as i64], |row| {
                Ok(VerificationRunRow {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    task_id: row.get(2)?,
                    commit_ref: row.get(3)?,
                    worktree_id: row.get(4)?,
                    environment: row.get(5)?,
                    command: row.get(6)?,
                    tool_version: row.get(7)?,
                    normalized_result: row.get(8)?,
                    raw_artifact: row.get(9)?,
                    evidence_ref: row.get(10)?,
                    freshness_dependencies: row.get(11)?,
                    created_at_ms: row.get(12)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    /// A unified, bounded, timestamp-ordered mission activity stream assembled
    /// exclusively from already-persisted rows: kernel events, task attempts,
    /// verification runs, changesets, and evidence records.  No event is
    /// synthesized that does not correspond to a durable row.  Returns the
    /// most recent `limit` events.
    pub fn mission_activity(&self, mission_id: &str, limit: usize) -> AcResult<Vec<MissionActivityRow>> {
        let mut events: Vec<MissionActivityRow> = Vec::new();

        // Kernel lifecycle/approval events.
        for event in self.kernel_events_for_mission(mission_id, 200)? {
            events.push(MissionActivityRow {
                id: format!("kernel-{}", event.0),
                kind: format!("kernel.{}", event.1),
                mission_id: mission_id.to_string(),
                subject_id: event.2,
                created_at_ms: event.3,
                detail: event.4,
            });
        }

        // Task attempts → attempt lifecycle events.
        for task_id in self.mission_task_ids(mission_id)? {
            for attempt in self.task_attempts(&task_id)? {
                events.push(MissionActivityRow {
                    id: format!("attempt-{}", attempt.id),
                    kind: format!("attempt.{}", attempt.outcome),
                    mission_id: mission_id.to_string(),
                    subject_id: task_id.clone(),
                    created_at_ms: attempt.created_at_ms,
                    detail: attempt
                        .failure_class
                        .unwrap_or_else(|| "attempt recorded".to_string()),
                });
            }
        }

        // Verification runs → verification events.
        for run in self.verification_runs_for_mission(mission_id, 200)? {
            events.push(MissionActivityRow {
                id: format!("verification-{}", run.id),
                kind: format!("verification.{}", run.normalized_result),
                mission_id: mission_id.to_string(),
                subject_id: run.task_id.clone().unwrap_or_default(),
                created_at_ms: run.created_at_ms,
                detail: bounded_detail(&run.environment),
            });
        }

        // Changesets → changeset lifecycle events.
        for cs in self.changesets_for_mission(mission_id, 200)? {
            events.push(MissionActivityRow {
                id: format!("changeset-{}", cs.id),
                kind: format!("changeset.{}", cs.state),
                mission_id: mission_id.to_string(),
                subject_id: cs.id.clone(),
                created_at_ms: cs.created_at_ms,
                detail: "changeset recorded".to_string(),
            });
        }

        // Evidence records → evidence events (bounded to the mission's own refs).
        for id in self.mission_evidence_ids(mission_id)? {
            let ids = vec![id.clone()];
            for record in self.evidence_records_by_ids(&ids)? {
                events.push(MissionActivityRow {
                    id: format!("evidence-{}", record.id),
                    kind: format!("evidence.{:?}", record.kind),
                    mission_id: mission_id.to_string(),
                    subject_id: record.id.to_string(),
                    created_at_ms: record.created_at.as_millis() as i64,
                    detail: bounded_detail(&record.model_summary.unwrap_or_default()),
                });
            }
        }

        events.sort_by_key(|b| std::cmp::Reverse(b.created_at_ms));
        events.truncate(limit);
        Ok(events)
    }

    /// Kernel events whose subject is the mission (mission lifecycle) or one
    /// of the mission's changesets (approval events).  Returns
    /// (id, decision_kind, subject_id, created_at_ms, evidence_refs).
    fn kernel_events_for_mission(
        &self,
        mission_id: &str,
        limit: usize,
    ) -> AcResult<Vec<KernelEventProjection>> {
        let mut subject_ids = vec![mission_id.to_string()];
        for cs in self.changesets_for_mission(mission_id, 200)? {
            subject_ids.push(cs.id);
        }
        if subject_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = (0..subject_ids.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT id, decision_kind, subject_id, created_at_ms, evidence_refs
             FROM kernel_events
             WHERE subject_id IN ({placeholders})
             ORDER BY created_at_ms DESC
             LIMIT ?"
        );
        let mut param_vec = subject_ids.clone();
        param_vec.push(limit.to_string());
        let param_refs = param_vec
            .iter()
            .map(|value| value as &dyn rusqlite::types::ToSql)
            .collect::<Vec<_>>();
        let mut stmt = self.connection.prepare(&query).map_err(db_error)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}

fn split_refs(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn bounded_detail(value: &str) -> String {
    const LIMIT: usize = 256;
    if value.len() <= LIMIT {
        value.to_string()
    } else {
        let boundary = value
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index <= LIMIT)
            .last()
            .unwrap_or(0);
        format!("{}…", &value[..boundary])
    }
}
