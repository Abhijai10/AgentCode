impl ControlPlaneDb {
    pub fn put_mission(&self, mission: &Mission) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO missions (id, original_goal, state, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at_ms = excluded.updated_at_ms",
                params![
                    mission.id.as_str(),
                    mission.original_goal,
                    mission_state(mission.state),
                    millis(mission.created_at),
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn get_mission(&self, id: &StableId) -> AcResult<Option<PersistedMission>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, original_goal, state, created_at_ms FROM missions WHERE id = ?1")
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedMission {
                id: row.get(0).map_err(db_error)?,
                original_goal: row.get(1).map_err(db_error)?,
                state: row.get(2).map_err(db_error)?,
                created_at_ms: row.get(3).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn append_kernel_event(&self, event: &KernelEvent) -> AcResult<()> {
        let evidence_refs = event
            .evidence_refs
            .iter()
            .map(StableId::to_string)
            .collect::<Vec<_>>()
            .join(",");
        self.connection
            .execute(
                "INSERT OR IGNORE INTO kernel_events (id, decision_kind, subject_id, evidence_refs, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event.id.as_str(),
                    decision_kind(event.decision),
                    event.subject.as_str(),
                    evidence_refs,
                    millis(event.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn kernel_event_count(&self) -> AcResult<u64> {
        self.connection
            .query_row("SELECT COUNT(*) FROM kernel_events", [], |row| row.get(0))
            .map_err(db_error)
    }

    pub fn append_evidence(&self, evidence: &EvidenceRecord) -> AcResult<()> {
        let durable_raw_content = (!evidence.sensitive)
            .then(|| evidence.raw_content.clone())
            .flatten();
        self.connection
            .execute(
                "INSERT OR IGNORE INTO evidence_records (
                    id, kind, provenance_json, artifact_uri, content_hash,
                    created_at_ms, raw_content, model_summary, sensitive
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    evidence.id.as_str(),
                    serde_json::to_string(&evidence.kind).map_err(|error| {
                        AcError::validation("DB-EVIDENCE_KIND_SERIALIZE", error.to_string())
                    })?,
                    serde_json::to_string(&evidence.provenance).map_err(|error| {
                        AcError::validation("DB-EVIDENCE_PROVENANCE_SERIALIZE", error.to_string())
                    })?,
                    evidence.artifact_uri.as_str(),
                    evidence.content_hash.as_str(),
                    millis(evidence.created_at),
                    durable_raw_content,
                    evidence.model_summary.as_deref(),
                    evidence.sensitive as i64
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn evidence_records(&self) -> AcResult<Vec<EvidenceRecord>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, kind, provenance_json, artifact_uri, content_hash,
                        created_at_ms, raw_content, model_summary, sensitive
                 FROM evidence_records
                 ORDER BY created_at_ms ASC, id ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
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

    pub fn save_routing_decision(&self, decision: &RoutingDecisionRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO provider_routing_decisions (
                    id, task_id, candidates_json, selected_json, rejected_json, fallback_reason,
                    latency_ms, input_tokens, output_tokens, estimated_cost_micros, created_at_ms
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    candidates_json = excluded.candidates_json,
                    selected_json = excluded.selected_json,
                    rejected_json = excluded.rejected_json,
                    fallback_reason = excluded.fallback_reason,
                    latency_ms = excluded.latency_ms,
                    input_tokens = excluded.input_tokens,
                    output_tokens = excluded.output_tokens,
                    estimated_cost_micros = excluded.estimated_cost_micros",
                params![
                    decision.id.as_str(),
                    decision.task_id.as_str(),
                    decision.candidates_json.as_str(),
                    decision.selected_json.as_deref(),
                    decision.rejected_json.as_str(),
                    decision.fallback_reason.as_deref(),
                    decision.latency_ms,
                    decision.input_tokens,
                    decision.output_tokens,
                    decision.estimated_cost_micros,
                    decision.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_provider_model_record(&self, record: &ProviderModelRecordRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO provider_model_records (
                    id, project_path, conversation_id, mission_id, session_id, task_id,
                    provider_id, provider_account_id, model_id, model_name, routing_mode,
                    attempt_number, success, failure_class, created_at_ms
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    record.id,
                    record.project_path,
                    record.conversation_id,
                    record.mission_id,
                    record.session_id,
                    record.task_id,
                    record.provider_id,
                    record.provider_account_id,
                    record.model_id,
                    record.model_name,
                    record.routing_mode,
                    record.attempt_number,
                    record.success as i64,
                    record.failure_class,
                    record.created_at_ms,
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn provider_model_records_for_mission(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<ProviderModelRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, project_path, conversation_id, mission_id, session_id, task_id,
                        provider_id, provider_account_id, model_id, model_name, routing_mode,
                        attempt_number, success, failure_class, created_at_ms
                 FROM provider_model_records
                 WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![mission_id], |row| {
                Ok(ProviderModelRecordRow {
                    id: row.get(0)?,
                    project_path: row.get(1)?,
                    conversation_id: row.get(2)?,
                    mission_id: row.get(3)?,
                    session_id: row.get(4)?,
                    task_id: row.get(5)?,
                    provider_id: row.get(6)?,
                    provider_account_id: row.get(7)?,
                    model_id: row.get(8)?,
                    model_name: row.get(9)?,
                    routing_mode: row.get(10)?,
                    attempt_number: row.get::<_, i64>(11)? as u32,
                    success: row.get::<_, i64>(12)? != 0,
                    failure_class: row.get(13)?,
                    created_at_ms: row.get(14)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn provider_model_records_for_conversation(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<ProviderModelRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, project_path, conversation_id, mission_id, session_id, task_id,
                        provider_id, provider_account_id, model_id, model_name, routing_mode,
                        attempt_number, success, failure_class, created_at_ms
                 FROM provider_model_records
                 WHERE conversation_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![conversation_id], |row| {
                Ok(ProviderModelRecordRow {
                    id: row.get(0)?,
                    project_path: row.get(1)?,
                    conversation_id: row.get(2)?,
                    mission_id: row.get(3)?,
                    session_id: row.get(4)?,
                    task_id: row.get(5)?,
                    provider_id: row.get(6)?,
                    provider_account_id: row.get(7)?,
                    model_id: row.get(8)?,
                    model_name: row.get(9)?,
                    routing_mode: row.get(10)?,
                    attempt_number: row.get::<_, i64>(11)? as u32,
                    success: row.get::<_, i64>(12)? != 0,
                    failure_class: row.get(13)?,
                    created_at_ms: row.get(14)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn routing_decision(&self, id: &str) -> AcResult<Option<RoutingDecisionRecord>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, task_id, candidates_json, selected_json, rejected_json, fallback_reason,
                        latency_ms, input_tokens, output_tokens, estimated_cost_micros, created_at_ms
                 FROM provider_routing_decisions
                 WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(RoutingDecisionRecord {
                id: row.get(0).map_err(db_error)?,
                task_id: row.get(1).map_err(db_error)?,
                candidates_json: row.get(2).map_err(db_error)?,
                selected_json: row.get(3).map_err(db_error)?,
                rejected_json: row.get(4).map_err(db_error)?,
                fallback_reason: row.get(5).map_err(db_error)?,
                latency_ms: row.get(6).map_err(db_error)?,
                input_tokens: row.get(7).map_err(db_error)?,
                output_tokens: row.get(8).map_err(db_error)?,
                estimated_cost_micros: row.get(9).map_err(db_error)?,
                created_at_ms: row.get(10).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn save_tool_execution(&self, record: &ToolExecutionRecord) -> AcResult<()> {
        self.connection.execute(
            "INSERT INTO tool_execution_records (id, tool_call_id, tool_id, status, manifest_json, raw_output, evidence_ref, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO NOTHING",
            params![record.id, record.tool_call_id, record.tool_id, record.status, record.manifest_json, record.raw_output, record.evidence_ref, record.created_at_ms as i64],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn tool_execution(&self, id: &str) -> AcResult<Option<ToolExecutionRecord>> {
        self.connection.query_row(
            "SELECT id, tool_call_id, tool_id, status, manifest_json, raw_output, evidence_ref, created_at_ms FROM tool_execution_records WHERE id = ?1",
            [id],
            |row| Ok(ToolExecutionRecord {
                id: row.get(0)?, tool_call_id: row.get(1)?, tool_id: row.get(2)?, status: row.get(3)?, manifest_json: row.get(4)?, raw_output: row.get(5)?, evidence_ref: row.get(6)?, created_at_ms: row.get::<_, i64>(7)? as u128,
            }),
        ).optional().map_err(db_error)
    }
}
