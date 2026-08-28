impl ControlPlaneDb {
    pub fn save_session(
        &self,
        session_id: &StableId,
        mission_id: &StableId,
        state: &str,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO agent_sessions (id, mission_id, state, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at_ms = excluded.updated_at_ms",
                params![
                    session_id.as_str(),
                    mission_id.as_str(),
                    state,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_checkpoint(
        &self,
        id: &StableId,
        session_id: &StableId,
        next_step: u32,
        state: &str,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO agent_checkpoints (id, session_id, next_step, state, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id.as_str(),
                    session_id.as_str(),
                    next_step,
                    state,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn update_session_state(&self, session_id: &StableId, state: &str) -> AcResult<()> {
        let changed = self
            .connection
            .execute(
                "UPDATE agent_sessions
                 SET state = ?2, updated_at_ms = ?3
                 WHERE id = ?1",
                params![session_id.as_str(), state, millis(TimestampMillis::now())],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err(AcError::conflict(
                "DB-SESSION_NOT_FOUND",
                format!("agent session {} was not found", session_id),
            ));
        }
        Ok(())
    }

    pub fn interrupted_sessions(&self) -> AcResult<Vec<PersistedSession>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, state, updated_at_ms
                 FROM agent_sessions
                 WHERE state NOT IN ('completed', 'cancelled', 'failed')",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PersistedSession {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    state: row.get(2)?,
                    updated_at_ms: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn active_sessions(&self) -> AcResult<Vec<PersistedSession>> {
        let mut stmt = self.connection.prepare(
            "SELECT id, mission_id, state, updated_at_ms FROM agent_sessions WHERE state NOT IN ('completed', 'cancelled', 'failed') ORDER BY updated_at_ms DESC",
        ).map_err(db_error)?;
        let rows = stmt.query_map([], |row| Ok(PersistedSession { id: row.get(0)?, mission_id: row.get(1)?, state: row.get(2)?, updated_at_ms: row.get(3)? })).map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn get_session(&self, session_id: &StableId) -> AcResult<Option<PersistedSession>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, state, updated_at_ms FROM agent_sessions WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![session_id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedSession {
                id: row.get(0).map_err(db_error)?,
                mission_id: row.get(1).map_err(db_error)?,
                state: row.get(2).map_err(db_error)?,
                updated_at_ms: row.get(3).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn session_for_mission(&self, mission_id: &StableId) -> AcResult<Option<PersistedSession>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, state, updated_at_ms
                 FROM agent_sessions
                 WHERE mission_id = ?1
                 ORDER BY updated_at_ms DESC
                 LIMIT 1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![mission_id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedSession {
                id: row.get(0).map_err(db_error)?,
                mission_id: row.get(1).map_err(db_error)?,
                state: row.get(2).map_err(db_error)?,
                updated_at_ms: row.get(3).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn checkpoints_for_session(
        &self,
        session_id: &StableId,
    ) -> AcResult<Vec<PersistedCheckpoint>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, session_id, next_step, state, created_at_ms
                 FROM agent_checkpoints
                 WHERE session_id = ?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![session_id.as_str()], |row| {
                Ok(PersistedCheckpoint {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    next_step: row.get(2)?,
                    state: row.get(3)?,
                    created_at_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_worker(&self, record: &WorkerRecord) -> AcResult<()> {
        self.connection.execute(
            "INSERT INTO workers (id, mission_id, session_id, state, workspace_ref, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET state = excluded.state, workspace_ref = excluded.workspace_ref, updated_at_ms = excluded.updated_at_ms",
            params![record.id, record.mission_id, record.session_id, record.state, record.workspace_ref, record.updated_at_ms],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn worker(&self, id: &str) -> AcResult<Option<WorkerRecord>> {
        self.connection.query_row(
            "SELECT id, mission_id, session_id, state, workspace_ref, updated_at_ms FROM workers WHERE id = ?1", [id],
            |row| Ok(WorkerRecord { id: row.get(0)?, mission_id: row.get(1)?, session_id: row.get(2)?, state: row.get(3)?, workspace_ref: row.get(4)?, updated_at_ms: row.get(5)? }),
        ).optional().map_err(db_error)
    }

    pub fn save_task(&self, record: &TaskRecord) -> AcResult<()> {
        self.connection.execute(
                "INSERT INTO tasks (id, mission_id, title, state, dependencies_json, assigned_worker_id, retry_count, max_retries, updated_at_ms, acceptance_criteria_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET state = excluded.state, assigned_worker_id = excluded.assigned_worker_id, retry_count = excluded.retry_count, updated_at_ms = excluded.updated_at_ms, acceptance_criteria_json = excluded.acceptance_criteria_json",
            params![record.id, record.mission_id, record.title, record.state, record.dependencies_json, record.assigned_worker_id, record.retry_count, record.max_retries, record.updated_at_ms, record.acceptance_criteria_json],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn tasks_for_mission(&self, mission_id: &str) -> AcResult<Vec<TaskRecord>> {
        let mut stmt = self.connection.prepare("SELECT id, mission_id, title, state, dependencies_json, assigned_worker_id, retry_count, max_retries, updated_at_ms, acceptance_criteria_json FROM tasks WHERE mission_id = ?1 ORDER BY id").map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(TaskRecord {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    title: row.get(2)?,
                    state: row.get(3)?,
                    dependencies_json: row.get(4)?,
                    assigned_worker_id: row.get(5)?,
                    retry_count: row.get(6)?,
                    max_retries: row.get(7)?,
                    updated_at_ms: row.get(8)?,
                    acceptance_criteria_json: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_task_attempt(&self, record: &TaskAttemptRecord) -> AcResult<()> {
        self.connection.execute(
            "INSERT INTO task_attempts (id, task_id, worker_id, outcome, evidence_refs, failure_class, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                outcome=excluded.outcome,
                evidence_refs=excluded.evidence_refs,
                failure_class=excluded.failure_class",
            params![record.id, record.task_id, record.worker_id, record.outcome, record.evidence_refs, record.failure_class, record.created_at_ms],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn task_attempts(&self, task_id: &str) -> AcResult<Vec<TaskAttemptRecord>> {
        let mut stmt = self.connection.prepare("SELECT id, task_id, worker_id, outcome, evidence_refs, failure_class, created_at_ms FROM task_attempts WHERE task_id = ?1 ORDER BY created_at_ms").map_err(db_error)?;
        let rows = stmt
            .query_map([task_id], |row| {
                Ok(TaskAttemptRecord {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    worker_id: row.get(2)?,
                    outcome: row.get(3)?,
                    evidence_refs: row.get(4)?,
                    failure_class: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_mission_contract_revision(&self, row: &MissionContractRevisionRow) -> AcResult<()> {
        if row.original_goal.trim().is_empty() || row.reason.trim().is_empty() {
            return Err(AcError::validation(
                "DB-MISSION_CONTRACT_INVALID",
                "mission contract revisions require original goal and reason",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO mission_contract_revisions VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    row.id,
                    row.mission_id,
                    row.revision,
                    row.original_goal,
                    row.reason,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn mission_contract_revisions(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<MissionContractRevisionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, revision, original_goal, reason, created_at_ms
             FROM mission_contract_revisions WHERE mission_id=?1 ORDER BY revision ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(MissionContractRevisionRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    revision: row.get(2)?,
                    original_goal: row.get(3)?,
                    reason: row.get(4)?,
                    created_at_ms: row.get(5)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_requirement_matrix_entry(&self, row: &RequirementMatrixEntryRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO requirement_matrix_entries VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    row.id,
                    row.mission_id,
                    row.contract_revision,
                    row.description,
                    row.requirement_type,
                    row.priority,
                    row.source,
                    row.verification_strategy,
                    if row.blocking { 1_i64 } else { 0_i64 },
                    row.implementation_status,
                    row.verification_status,
                    row.evidence_refs,
                    row.linked_task_ids,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn requirement_matrix_entries(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<RequirementMatrixEntryRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, contract_revision, description, requirement_type, priority,
                    source, verification_strategy, blocking, implementation_status,
                    verification_status, evidence_refs, linked_task_ids, created_at_ms
             FROM requirement_matrix_entries WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(RequirementMatrixEntryRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    contract_revision: row.get(2)?,
                    description: row.get(3)?,
                    requirement_type: row.get(4)?,
                    priority: row.get(5)?,
                    source: row.get(6)?,
                    verification_strategy: row.get(7)?,
                    blocking: row.get::<_, i64>(8)? != 0,
                    implementation_status: row.get(9)?,
                    verification_status: row.get(10)?,
                    evidence_refs: row.get(11)?,
                    linked_task_ids: row.get(12)?,
                    created_at_ms: row.get(13)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_task_lease(&self, row: &TaskLeaseRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO task_leases VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(task_id) DO UPDATE SET
                    worker_id=excluded.worker_id,
                    lease_epoch=excluded.lease_epoch,
                    expires_at_ms=excluded.expires_at_ms,
                    heartbeat_interval_ms=excluded.heartbeat_interval_ms,
                    state=excluded.state,
                    updated_at_ms=excluded.updated_at_ms",
                params![
                    row.task_id,
                    row.worker_id,
                    row.lease_epoch,
                    row.expires_at_ms,
                    row.heartbeat_interval_ms,
                    row.state,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn task_lease(&self, task_id: &str) -> AcResult<Option<TaskLeaseRow>> {
        self.connection
            .query_row(
                "SELECT task_id, worker_id, lease_epoch, expires_at_ms, heartbeat_interval_ms,
                        state, updated_at_ms FROM task_leases WHERE task_id=?1",
                params![task_id],
                |row| {
                    Ok(TaskLeaseRow {
                        task_id: row.get(0)?,
                        worker_id: row.get(1)?,
                        lease_epoch: row.get(2)?,
                        expires_at_ms: row.get(3)?,
                        heartbeat_interval_ms: row.get(4)?,
                        state: row.get(5)?,
                        updated_at_ms: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_autonomy_mailbox_message(&self, row: &AutonomyMailboxMessageRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO autonomy_mailbox_messages VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    row.id,
                    row.mission_id,
                    row.sender_worker_id,
                    row.recipient_worker_id,
                    row.message_type,
                    row.subject_id,
                    row.payload,
                    if row.delivered { 1_i64 } else { 0_i64 },
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn autonomy_mailbox_messages(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<AutonomyMailboxMessageRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, sender_worker_id, recipient_worker_id, message_type,
                    subject_id, payload, delivered, created_at_ms
             FROM autonomy_mailbox_messages WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(AutonomyMailboxMessageRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    sender_worker_id: row.get(2)?,
                    recipient_worker_id: row.get(3)?,
                    message_type: row.get(4)?,
                    subject_id: row.get(5)?,
                    payload: row.get(6)?,
                    delivered: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_autonomy_record(&self, row: &AutonomyRecordRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO autonomy_records VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    row.id,
                    row.mission_id,
                    row.category,
                    row.subject_id,
                    row.payload,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn autonomy_records(
        &self,
        mission_id: &str,
        category: &str,
    ) -> AcResult<Vec<AutonomyRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, category, subject_id, payload, created_at_ms
             FROM autonomy_records WHERE mission_id=?1 AND category=?2 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![mission_id, category], |row| {
                Ok(AutonomyRecordRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    category: row.get(2)?,
                    subject_id: row.get(3)?,
                    payload: row.get(4)?,
                    created_at_ms: row.get(5)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}
