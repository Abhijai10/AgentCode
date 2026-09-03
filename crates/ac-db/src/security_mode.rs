// ── Security Mode persistence (G5) ─────────────────────────────────────────
// Durable per-conversation security workspace state.  The daemon is the only
// writer (Kernel-owned execution authority); the UI reads projected state
// through the daemon IPC.  Project isolation is enforced by keying rows to a
// conversation, and conversations are project-bound.

impl ControlPlaneDb {
    // ── Security Mode Sessions ──────────────────────────────────────────────

    pub fn save_security_mode_session(&self, row: &SecurityModeSessionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_sessions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(conversation_id) DO UPDATE SET
                   scope_json=excluded.scope_json,
                   threat_model_json=excluded.threat_model_json,
                   audit_status=excluded.audit_status,
                   final_status=excluded.final_status,
                   source_commit=excluded.source_commit,
                   baseline_commit=excluded.baseline_commit,
                   baseline_roots=excluded.baseline_roots,
                   baseline_attack_paths=excluded.baseline_attack_paths,
                   baseline_accepted_risk=excluded.baseline_accepted_risk,
                   updated_at_ms=excluded.updated_at_ms",
                params![
                    row.conversation_id,
                    row.project_path,
                    row.scope_json,
                    row.threat_model_json,
                    row.audit_status,
                    row.final_status,
                    row.source_commit,
                    row.baseline_commit,
                    row.baseline_roots,
                    row.baseline_attack_paths,
                    row.baseline_accepted_risk,
                    row.created_at_ms,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_session(
        &self,
        conversation_id: &str,
    ) -> AcResult<Option<SecurityModeSessionRow>> {
        self.connection
            .query_row(
                "SELECT conversation_id, project_path, scope_json, threat_model_json,
                 audit_status, final_status, source_commit, baseline_commit,
                 baseline_roots, baseline_attack_paths, baseline_accepted_risk,
                 created_at_ms, updated_at_ms
                 FROM security_mode_sessions WHERE conversation_id=?1",
                [conversation_id],
                |row| {
                    Ok(SecurityModeSessionRow {
                        conversation_id: row.get(0)?,
                        project_path: row.get(1)?,
                        scope_json: row.get(2)?,
                        threat_model_json: row.get(3)?,
                        audit_status: row.get(4)?,
                        final_status: row.get(5)?,
                        source_commit: row.get(6)?,
                        baseline_commit: row.get(7)?,
                        baseline_roots: row.get(8)?,
                        baseline_attack_paths: row.get(9)?,
                        baseline_accepted_risk: row.get(10)?,
                        created_at_ms: row.get(11)?,
                        updated_at_ms: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn security_mode_sessions_for_project(
        &self,
        project_path: &str,
    ) -> AcResult<Vec<SecurityModeSessionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT conversation_id, project_path, scope_json, threat_model_json,
                 audit_status, final_status, source_commit, baseline_commit,
                 baseline_roots, baseline_attack_paths, baseline_accepted_risk,
                 created_at_ms, updated_at_ms
                 FROM security_mode_sessions WHERE project_path=?1 ORDER BY updated_at_ms DESC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([project_path], |row| {
                Ok(SecurityModeSessionRow {
                    conversation_id: row.get(0)?,
                    project_path: row.get(1)?,
                    scope_json: row.get(2)?,
                    threat_model_json: row.get(3)?,
                    audit_status: row.get(4)?,
                    final_status: row.get(5)?,
                    source_commit: row.get(6)?,
                    baseline_commit: row.get(7)?,
                    baseline_roots: row.get(8)?,
                    baseline_attack_paths: row.get(9)?,
                    baseline_accepted_risk: row.get(10)?,
                    created_at_ms: row.get(11)?,
                    updated_at_ms: row.get(12)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    // ── Security Mode Findings ───────────────────────────────────────────────

    pub fn save_security_mode_finding(&self, row: &SecurityModeFindingRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_findings VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
                 ON CONFLICT(id) DO UPDATE SET
                   severity=excluded.severity,
                   confidence=excluded.confidence,
                   exploitability=excluded.exploitability,
                   state=excluded.state,
                   affected_code=excluded.affected_code,
                   attack_path_refs=excluded.attack_path_refs,
                   evidence_refs=excluded.evidence_refs,
                   scanner_refs=excluded.scanner_refs,
                   remediation=excluded.remediation,
                   regression_refs=excluded.regression_refs,
                   mission_ref=excluded.mission_ref,
                   updated_at_ms=excluded.updated_at_ms",
                params![
                    row.id,
                    row.conversation_id,
                    row.fingerprint,
                    row.root_cause,
                    row.category,
                    row.severity,
                    row.confidence,
                    row.exploitability,
                    row.state,
                    row.affected_code,
                    row.affected_asset,
                    row.entry_point,
                    row.attack_path_refs,
                    row.evidence_refs,
                    row.scanner_refs,
                    row.remediation,
                    row.regression_refs,
                    row.source_commit,
                    row.environment,
                    row.scope_ref,
                    row.mission_ref,
                    row.created_at_ms,
                    row.updated_at_ms,
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_finding(
        &self,
        id: &str,
    ) -> AcResult<Option<SecurityModeFindingRow>> {
        self.connection
            .query_row(
                "SELECT id, conversation_id, fingerprint, root_cause, category, severity,
                 confidence, exploitability, state, affected_code, affected_asset,
                 entry_point, attack_path_refs, evidence_refs, scanner_refs, remediation,
                 regression_refs, source_commit, environment, scope_ref, mission_ref,
                 created_at_ms, updated_at_ms
                 FROM security_mode_findings WHERE id=?1",
                [id],
                |row| Ok(finding_row(row)),
            )
            .optional()
            .map_err(db_error)
    }

    pub fn security_mode_findings_for_conversation(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<SecurityModeFindingRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, fingerprint, root_cause, category, severity,
                 confidence, exploitability, state, affected_code, affected_asset,
                 entry_point, attack_path_refs, evidence_refs, scanner_refs, remediation,
                 regression_refs, source_commit, environment, scope_ref, mission_ref,
                 created_at_ms, updated_at_ms
                 FROM security_mode_findings WHERE conversation_id=?1
                 ORDER BY updated_at_ms DESC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| Ok(finding_row(row)))
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn security_mode_finding_by_fingerprint(
        &self,
        conversation_id: &str,
        fingerprint: &str,
    ) -> AcResult<Option<SecurityModeFindingRow>> {
        self.connection
            .query_row(
                "SELECT id, conversation_id, fingerprint, root_cause, category, severity,
                 confidence, exploitability, state, affected_code, affected_asset,
                 entry_point, attack_path_refs, evidence_refs, scanner_refs, remediation,
                 regression_refs, source_commit, environment, scope_ref, mission_ref,
                 created_at_ms, updated_at_ms
                 FROM security_mode_findings
                 WHERE conversation_id=?1 AND fingerprint=?2 LIMIT 1",
                params![conversation_id, fingerprint],
                |row| Ok(finding_row(row)),
            )
            .optional()
            .map_err(db_error)
    }

    pub fn set_security_mode_finding_mission(
        &self,
        finding_id: &str,
        mission_ref: &str,
    ) -> AcResult<()> {
        let now = TimestampMillis::now().as_millis() as i64;
        self.connection
            .execute(
                "UPDATE security_mode_findings SET mission_ref=?1, updated_at_ms=?2 WHERE id=?3",
                params![mission_ref, now, finding_id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    // ── Security Mode Attack Paths ───────────────────────────────────────────

    pub fn save_security_mode_attack_path(&self, row: &SecurityModeAttackPathRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_attack_paths VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO NOTHING",
                params![row.id, row.conversation_id, row.path_json, row.created_at_ms],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_attack_paths(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<SecurityModeAttackPathRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, path_json, created_at_ms
                 FROM security_mode_attack_paths WHERE conversation_id=?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(SecurityModeAttackPathRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    path_json: row.get(2)?,
                    created_at_ms: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn delete_security_mode_attack_paths(&self, conversation_id: &str) -> AcResult<()> {
        self.connection
            .execute(
                "DELETE FROM security_mode_attack_paths WHERE conversation_id=?1",
                [conversation_id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    // ── Security Mode Validations ────────────────────────────────────────────

    pub fn save_security_mode_validation(&self, row: &SecurityModeValidationRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_validations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    row.id,
                    row.conversation_id,
                    row.finding_id,
                    row.plan_id,
                    row.state,
                    row.detail,
                    row.evidence_ref,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_validations(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<SecurityModeValidationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, finding_id, plan_id, state, detail, evidence_ref,
                 created_at_ms FROM security_mode_validations
                 WHERE conversation_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(SecurityModeValidationRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    finding_id: row.get(2)?,
                    plan_id: row.get(3)?,
                    state: row.get(4)?,
                    detail: row.get(5)?,
                    evidence_ref: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    // ── Security Mode Regressions ────────────────────────────────────────────

    pub fn save_security_mode_regression(&self, row: &SecurityModeRegressionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_regressions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                   state=excluded.state,
                   last_verified_commit=excluded.last_verified_commit",
                params![
                    row.id,
                    row.conversation_id,
                    row.finding_id,
                    row.regression_type,
                    row.target_refs,
                    row.evidence_ref,
                    row.last_verified_commit,
                    row.state,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_regressions(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<SecurityModeRegressionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, finding_id, regression_type, target_refs,
                 evidence_ref, last_verified_commit, state, created_at_ms
                 FROM security_mode_regressions WHERE conversation_id=?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(SecurityModeRegressionRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    finding_id: row.get(2)?,
                    regression_type: row.get(3)?,
                    target_refs: row.get(4)?,
                    evidence_ref: row.get(5)?,
                    last_verified_commit: row.get(6)?,
                    state: row.get(7)?,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    // ── Security Mode Suppressions ───────────────────────────────────────────

    pub fn save_security_mode_suppression(&self, row: &SecurityModeSuppressionRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_suppressions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET
                   state=excluded.state,
                   expires_at_ms=excluded.expires_at_ms",
                params![
                    row.id,
                    row.conversation_id,
                    row.finding_id,
                    row.scope_ref,
                    row.reason,
                    row.source_actor,
                    row.created_at_ms,
                    row.expires_at_ms,
                    row.state,
                    row.applicability,
                    row.compensating_controls,
                    row.evidence_ref
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_suppressions(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<SecurityModeSuppressionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, finding_id, scope_ref, reason, source_actor,
                 created_at_ms, expires_at_ms, state, applicability, compensating_controls,
                 evidence_ref FROM security_mode_suppressions
                 WHERE conversation_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(SecurityModeSuppressionRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    finding_id: row.get(2)?,
                    scope_ref: row.get(3)?,
                    reason: row.get(4)?,
                    source_actor: row.get(5)?,
                    created_at_ms: row.get(6)?,
                    expires_at_ms: row.get(7)?,
                    state: row.get(8)?,
                    applicability: row.get(9)?,
                    compensating_controls: row.get(10)?,
                    evidence_ref: row.get(11)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    // ── Security Mode Risk Acceptances ───────────────────────────────────────

    pub fn save_security_mode_risk_acceptance(
        &self,
        row: &SecurityModeRiskAcceptanceRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_risk_acceptances VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(id) DO UPDATE SET
                   state=excluded.state,
                   expires_at_ms=excluded.expires_at_ms",
                params![
                    row.id,
                    row.conversation_id,
                    row.finding_id,
                    row.scope_ref,
                    row.severity,
                    row.rationale,
                    row.approver,
                    row.accepted_at_ms,
                    row.review_at_ms,
                    row.expires_at_ms,
                    row.completion_allowed,
                    row.evidence_ref,
                    row.state,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_risk_acceptances(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<SecurityModeRiskAcceptanceRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, finding_id, scope_ref, severity, rationale,
                 approver, accepted_at_ms, review_at_ms, expires_at_ms, completion_allowed,
                 evidence_ref, state, created_at_ms
                 FROM security_mode_risk_acceptances WHERE conversation_id=?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(SecurityModeRiskAcceptanceRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    finding_id: row.get(2)?,
                    scope_ref: row.get(3)?,
                    severity: row.get(4)?,
                    rationale: row.get(5)?,
                    approver: row.get(6)?,
                    accepted_at_ms: row.get(7)?,
                    review_at_ms: row.get(8)?,
                    expires_at_ms: row.get(9)?,
                    completion_allowed: row.get(10)?,
                    evidence_ref: row.get(11)?,
                    state: row.get(12)?,
                    created_at_ms: row.get(13)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    // ── Security Mode Reports ────────────────────────────────────────────────

    pub fn save_security_mode_report(&self, row: &SecurityModeReportRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_mode_reports VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET report_markdown=excluded.report_markdown,
                   final_status=excluded.final_status, created_at_ms=excluded.created_at_ms",
                params![
                    row.id,
                    row.conversation_id,
                    row.report_markdown,
                    row.final_status,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_mode_reports(
        &self,
        conversation_id: &str,
    ) -> AcResult<Vec<SecurityModeReportRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, conversation_id, report_markdown, final_status, created_at_ms
                 FROM security_mode_reports WHERE conversation_id=?1
                 ORDER BY created_at_ms DESC LIMIT 20",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([conversation_id], |row| {
                Ok(SecurityModeReportRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    report_markdown: row.get(2)?,
                    final_status: row.get(3)?,
                    created_at_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}

fn finding_row(row: &rusqlite::Row<'_>) -> SecurityModeFindingRow {
    SecurityModeFindingRow {
        id: row.get(0).unwrap_or_default(),
        conversation_id: row.get(1).unwrap_or_default(),
        fingerprint: row.get(2).unwrap_or_default(),
        root_cause: row.get(3).unwrap_or_default(),
        category: row.get(4).unwrap_or_default(),
        severity: row.get(5).unwrap_or_default(),
        confidence: row.get(6).unwrap_or(0),
        exploitability: row.get(7).unwrap_or(0),
        state: row.get(8).unwrap_or_default(),
        affected_code: row.get(9).unwrap_or_default(),
        affected_asset: row.get(10).unwrap_or_default(),
        entry_point: row.get(11).unwrap_or(None),
        attack_path_refs: row.get(12).unwrap_or_default(),
        evidence_refs: row.get(13).unwrap_or_default(),
        scanner_refs: row.get(14).unwrap_or_default(),
        remediation: row.get(15).unwrap_or_default(),
        regression_refs: row.get(16).unwrap_or_default(),
        source_commit: row.get(17).unwrap_or_default(),
        environment: row.get(18).unwrap_or_default(),
        scope_ref: row.get(19).unwrap_or_default(),
        mission_ref: row.get(20).unwrap_or(None),
        created_at_ms: row.get(21).unwrap_or(0),
        updated_at_ms: row.get(22).unwrap_or(0),
    }
}
