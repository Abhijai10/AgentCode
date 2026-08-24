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
        self.connection
            .execute(
                "INSERT INTO evidence_records (id, kind, provenance_json, artifact_uri, content_hash, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    evidence.id.as_str(),
                    format!("{:?}", evidence.kind),
                    format!("{:?}", evidence.provenance),
                    evidence.artifact_uri,
                    evidence.content_hash,
                    millis(evidence.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
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
