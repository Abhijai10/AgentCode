impl ControlPlaneDb {
    pub fn save_desktop_session(&self, row: &DesktopSessionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO desktop_sessions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET active_project_id=excluded.active_project_id,
                 active_mission_id=excluded.active_mission_id, selected_view=excluded.selected_view,
                 window_open=excluded.window_open, daemon_connected=excluded.daemon_connected,
                 updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.active_project_id,
                    row.active_mission_id,
                    row.selected_view,
                    row.window_open,
                    row.daemon_connected,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn desktop_session(&self, id: &str) -> AcResult<Option<DesktopSessionRow>> {
        self.connection
            .query_row(
                "SELECT id, active_project_id, active_mission_id, selected_view, window_open,
                 daemon_connected, created_at_ms, updated_at_ms FROM desktop_sessions WHERE id=?1",
                [id],
                |row| {
                    Ok(DesktopSessionRow {
                        id: row.get(0)?,
                        active_project_id: row.get(1)?,
                        active_mission_id: row.get(2)?,
                        selected_view: row.get(3)?,
                        window_open: row.get(4)?,
                        daemon_connected: row.get(5)?,
                        created_at_ms: row.get(6)?,
                        updated_at_ms: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_desktop_project(&self, row: &DesktopProjectRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO desktop_projects VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(path) DO UPDATE SET name=excluded.name,
                 repository_id=excluded.repository_id, last_opened_at_ms=excluded.last_opened_at_ms",
                params![
                    row.id,
                    row.name,
                    row.path,
                    row.repository_id,
                    row.last_opened_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn recent_desktop_projects(&self) -> AcResult<Vec<DesktopProjectRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, name, path, repository_id, last_opened_at_ms
                 FROM desktop_projects ORDER BY last_opened_at_ms DESC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DesktopProjectRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    repository_id: row.get(3)?,
                    last_opened_at_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_desktop_preference(&self, row: &DesktopPreferenceRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO desktop_preferences VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(session_id) DO UPDATE SET appearance=excluded.appearance,
                 notifications_enabled=excluded.notifications_enabled,
                 completion_sound_enabled=excluded.completion_sound_enabled,
                 reduced_motion=excluded.reduced_motion,
                 budget_limit_micros=excluded.budget_limit_micros",
                params![
                    row.session_id,
                    row.appearance,
                    row.notifications_enabled,
                    row.completion_sound_enabled,
                    row.reduced_motion,
                    row.budget_limit_micros
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn desktop_preference(&self, session_id: &str) -> AcResult<Option<DesktopPreferenceRow>> {
        self.connection
            .query_row(
                "SELECT session_id, appearance, notifications_enabled, completion_sound_enabled,
                 reduced_motion, budget_limit_micros FROM desktop_preferences WHERE session_id=?1",
                [session_id],
                |row| {
                    Ok(DesktopPreferenceRow {
                        session_id: row.get(0)?,
                        appearance: row.get(1)?,
                        notifications_enabled: row.get(2)?,
                        completion_sound_enabled: row.get(3)?,
                        reduced_motion: row.get(4)?,
                        budget_limit_micros: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_desktop_ui_state(&self, row: &DesktopUiStateRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO desktop_ui_state VALUES (?1, ?2, ?3)
                 ON CONFLICT(session_id) DO UPDATE SET serialized_state=excluded.serialized_state,
                 updated_at_ms=excluded.updated_at_ms",
                params![row.session_id, row.serialized_state, row.updated_at_ms],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_desktop_approval_record(&self, row: &DesktopApprovalRecordRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO desktop_approval_records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.approval_id,
                    row.mission_id,
                    row.approval_kind,
                    row.decision,
                    row.explanation,
                    row.evidence_refs,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn desktop_approval_records(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<DesktopApprovalRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, approval_id, mission_id, approval_kind, decision, explanation,
                 evidence_refs, created_at_ms FROM desktop_approval_records
                 WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(DesktopApprovalRecordRow {
                    id: row.get(0)?,
                    approval_id: row.get(1)?,
                    mission_id: row.get(2)?,
                    approval_kind: row.get(3)?,
                    decision: row.get(4)?,
                    explanation: row.get(5)?,
                    evidence_refs: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_token_usage_record(&self, row: &TokenUsageRecordRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO token_usage_records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    row.id,
                    row.task_id,
                    row.provider_call_id,
                    row.input_tokens,
                    row.output_tokens,
                    row.context_tokens,
                    row.compressed_tokens,
                    row.estimated_cost_micros,
                    row.verified,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn token_usage_records(&self, task_id: &str) -> AcResult<Vec<TokenUsageRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, task_id, provider_call_id, input_tokens, output_tokens,
                 context_tokens, compressed_tokens, estimated_cost_micros, verified, created_at_ms
                 FROM token_usage_records WHERE task_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([task_id], |row| {
                Ok(TokenUsageRecordRow {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    provider_call_id: row.get(2)?,
                    input_tokens: row.get(3)?,
                    output_tokens: row.get(4)?,
                    context_tokens: row.get(5)?,
                    compressed_tokens: row.get(6)?,
                    estimated_cost_micros: row.get(7)?,
                    verified: row.get(8)?,
                    created_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_resource_telemetry_record(
        &self,
        row: &ResourceTelemetryRecordRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO resource_telemetry_records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    row.id,
                    row.component,
                    row.rss_bytes,
                    row.cpu_millis,
                    row.disk_bytes,
                    row.process_count,
                    row.worker_count,
                    row.browser_sessions,
                    row.lsp_sessions,
                    row.local_model_loaded,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn resource_telemetry_records(
        &self,
        component: &str,
    ) -> AcResult<Vec<ResourceTelemetryRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, component, rss_bytes, cpu_millis, disk_bytes, process_count,
                 worker_count, browser_sessions, lsp_sessions, local_model_loaded, created_at_ms
                 FROM resource_telemetry_records WHERE component=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([component], |row| {
                Ok(ResourceTelemetryRecordRow {
                    id: row.get(0)?,
                    component: row.get(1)?,
                    rss_bytes: row.get(2)?,
                    cpu_millis: row.get(3)?,
                    disk_bytes: row.get(4)?,
                    process_count: row.get(5)?,
                    worker_count: row.get(6)?,
                    browser_sessions: row.get(7)?,
                    lsp_sessions: row.get(8)?,
                    local_model_loaded: row.get(9)?,
                    created_at_ms: row.get(10)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_optimization_report(&self, row: &OptimizationReportRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO optimization_reports VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.total_tokens,
                    row.verified_tokens,
                    row.total_cost_micros,
                    row.cost_per_verified_task_micros,
                    row.average_compression_ratio,
                    row.before_after,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }
}
