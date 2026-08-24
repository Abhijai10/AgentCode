impl ControlPlaneDb {
    pub fn save_edit_transaction(
        &self,
        transaction: &EditTransactionRow,
        operations: &[EditOperationRow],
        journal: &[EditJournalEntryRow],
        metrics: &[EditStrategyMetricRow],
    ) -> AcResult<()> {
        if transaction.base_revision.trim().is_empty() || operations.is_empty() {
            return Err(AcError::validation(
                "DB-EDIT_TRANSACTION_INVALID",
                "edit transactions require base revision and operations",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO edit_transactions (
                    id, changeset_id, task_id, worktree_id, base_revision, state, formatter,
                    degraded_reason, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    state=excluded.state,
                    formatter=excluded.formatter,
                    degraded_reason=excluded.degraded_reason,
                    updated_at_ms=excluded.updated_at_ms",
                params![
                    transaction.id,
                    transaction.changeset_id,
                    transaction.task_id,
                    transaction.worktree_id,
                    transaction.base_revision,
                    transaction.state,
                    transaction.formatter,
                    transaction.degraded_reason,
                    transaction.created_at_ms,
                    transaction.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        for table in [
            "edit_operations",
            "edit_journal_entries",
            "edit_strategy_metrics",
        ] {
            self.connection
                .execute(
                    &format!("DELETE FROM {table} WHERE transaction_id=?1"),
                    params![transaction.id],
                )
                .map_err(db_error)?;
        }
        for row in operations {
            self.connection
                .execute(
                    "INSERT INTO edit_operations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        row.id,
                        row.transaction_id,
                        row.path,
                        row.strategy,
                        row.before_hash,
                        row.after_hash,
                        row.symbol_fingerprint,
                        row.additions,
                        row.removals
                    ],
                )
                .map_err(db_error)?;
        }
        for row in journal {
            self.connection
                .execute(
                    "INSERT INTO edit_journal_entries VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        row.id,
                        row.transaction_id,
                        row.path,
                        row.state,
                        row.before_hash,
                        row.after_hash,
                        row.created_at_ms
                    ],
                )
                .map_err(db_error)?;
        }
        for row in metrics {
            self.connection
                .execute(
                    "INSERT INTO edit_strategy_metrics VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        row.id,
                        row.transaction_id,
                        row.task_id,
                        row.model_id,
                        row.language,
                        row.strategy,
                        if row.first_apply_success { 1_i64 } else { 0_i64 },
                        row.syntax_failures,
                        row.retries,
                        row.unrelated_diff_files,
                        row.verification_rejections,
                        if row.degraded { 1_i64 } else { 0_i64 },
                        row.created_at_ms
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn save_changeset_transaction(
        &self,
        transaction: &ChangeSetTransaction,
        task_id: Option<&str>,
        worktree_id: Option<&str>,
        base_revision: &str,
        language: &str,
    ) -> AcResult<()> {
        let now = millis(TimestampMillis::now());
        let row = EditTransactionRow {
            id: transaction.id.to_string(),
            changeset_id: transaction.changeset.id.to_string(),
            task_id: task_id.map(ToString::to_string),
            worktree_id: worktree_id.map(ToString::to_string),
            base_revision: base_revision.to_string(),
            state: format!("{:?}", transaction.changeset.state),
            formatter: transaction.format_plan.formatter.clone(),
            degraded_reason: transaction.format_plan.degraded_reason.clone(),
            created_at_ms: millis(transaction.changeset.created_at),
            updated_at_ms: now,
        };
        let operations = transaction
            .edits
            .iter()
            .map(|edit| EditOperationRow {
                id: StableId::new("editop").to_string(),
                transaction_id: row.id.clone(),
                path: edit.path.clone(),
                strategy: format!("{:?}", edit.strategy),
                before_hash: edit.before_hash.clone(),
                after_hash: edit.after_hash.clone(),
                symbol_fingerprint: edit.symbol_fingerprint.clone(),
                additions: edit.additions,
                removals: edit.removals,
            })
            .collect::<Vec<_>>();
        let journal = transaction
            .journal
            .entries
            .iter()
            .map(|entry| EditJournalEntryRow {
                id: entry.id.to_string(),
                transaction_id: row.id.clone(),
                path: entry.path.clone(),
                state: format!("{:?}", entry.state),
                before_hash: entry.before_hash.clone(),
                after_hash: entry.after_hash.clone(),
                created_at_ms: millis(entry.created_at),
            })
            .collect::<Vec<_>>();
        let metrics = vec![EditStrategyMetricRow {
            id: StableId::new("editmetric").to_string(),
            transaction_id: row.id.clone(),
            task_id: task_id.map(ToString::to_string),
            model_id: None,
            language: language.to_string(),
            strategy: format!("{:?}", transaction.metrics.strategy),
            first_apply_success: transaction.metrics.first_apply_success,
            syntax_failures: transaction.metrics.syntax_failures,
            retries: transaction.metrics.retries,
            unrelated_diff_files: transaction.metrics.unrelated_diff_files,
            verification_rejections: transaction.metrics.verification_rejections,
            degraded: transaction.metrics.degraded,
            created_at_ms: now,
        }];
        self.save_edit_transaction(&row, &operations, &journal, &metrics)
    }

    pub fn edit_transaction(&self, id: &str) -> AcResult<Option<EditTransactionRow>> {
        self.connection
            .query_row(
                "SELECT id, changeset_id, task_id, worktree_id, base_revision, state, formatter,
                        degraded_reason, created_at_ms, updated_at_ms
                 FROM edit_transactions WHERE id=?1",
                params![id],
                |row| {
                    Ok(EditTransactionRow {
                        id: row.get(0)?,
                        changeset_id: row.get(1)?,
                        task_id: row.get(2)?,
                        worktree_id: row.get(3)?,
                        base_revision: row.get(4)?,
                        state: row.get(5)?,
                        formatter: row.get(6)?,
                        degraded_reason: row.get(7)?,
                        created_at_ms: row.get(8)?,
                        updated_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn edit_journal_entries(&self, transaction_id: &str) -> AcResult<Vec<EditJournalEntryRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, transaction_id, path, state, before_hash, after_hash, created_at_ms
                 FROM edit_journal_entries WHERE transaction_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![transaction_id], |row| {
                Ok(EditJournalEntryRow {
                    id: row.get(0)?,
                    transaction_id: row.get(1)?,
                    path: row.get(2)?,
                    state: row.get(3)?,
                    before_hash: row.get(4)?,
                    after_hash: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn pending_edit_transactions(&self) -> AcResult<Vec<EditTransactionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, changeset_id, task_id, worktree_id, base_revision, state, formatter,
                        degraded_reason, created_at_ms, updated_at_ms
                 FROM edit_transactions
                 WHERE state IN ('Applying', 'RollingBack', 'UnknownEffect')
                 ORDER BY updated_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EditTransactionRow {
                    id: row.get(0)?,
                    changeset_id: row.get(1)?,
                    task_id: row.get(2)?,
                    worktree_id: row.get(3)?,
                    base_revision: row.get(4)?,
                    state: row.get(5)?,
                    formatter: row.get(6)?,
                    degraded_reason: row.get(7)?,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn edit_strategy_metrics(
        &self,
        transaction_id: &str,
    ) -> AcResult<Vec<EditStrategyMetricRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, transaction_id, task_id, model_id, language, strategy,
                        first_apply_success, syntax_failures, retries, unrelated_diff_files,
                        verification_rejections, degraded, created_at_ms
                 FROM edit_strategy_metrics WHERE transaction_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![transaction_id], |row| {
                Ok(EditStrategyMetricRow {
                    id: row.get(0)?,
                    transaction_id: row.get(1)?,
                    task_id: row.get(2)?,
                    model_id: row.get(3)?,
                    language: row.get(4)?,
                    strategy: row.get(5)?,
                    first_apply_success: row.get::<_, i64>(6)? != 0,
                    syntax_failures: row.get(7)?,
                    retries: row.get(8)?,
                    unrelated_diff_files: row.get(9)?,
                    verification_rejections: row.get(10)?,
                    degraded: row.get::<_, i64>(11)? != 0,
                    created_at_ms: row.get(12)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_verification_profile(&self, profile: &VerificationProfile) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO verification_profiles VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET required_layers=excluded.required_layers",
                params![
                    profile.id.to_string(),
                    profile.task_id.to_string(),
                    format!("{:?}", profile.risk),
                    profile
                        .required_layers
                        .iter()
                        .map(|layer| format!("{:?}", layer))
                        .collect::<Vec<_>>()
                        .join(","),
                    millis(profile.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_verification_manifest(
        &self,
        manifest: &VerificationEvidenceManifest,
        profile_id: Option<&str>,
        task_id: Option<&str>,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO verification_runs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    manifest.id.to_string(),
                    profile_id,
                    task_id,
                    manifest.commit,
                    manifest.worktree,
                    manifest.environment,
                    manifest.command,
                    manifest.tool_version,
                    format!("{:?}", manifest.normalized_result),
                    manifest.raw_artifact,
                    manifest.evidence_ref.to_string(),
                    manifest.freshness_dependencies.join(","),
                    millis(manifest.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_requirement_verification(
        &self,
        run_id: &str,
        link: &RequirementEvidenceLink,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO requirement_verifications VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    StableId::new("reqver").to_string(),
                    link.requirement_id.to_string(),
                    run_id,
                    link.evidence_ref.to_string(),
                    link.evidence_kind,
                    if link.verified { 1_i64 } else { 0_i64 },
                    link.freshness_key,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_final_audit(
        &self,
        mission_id: &str,
        original_goal: &str,
        requirements: &[String],
        audit: &FinalAuditReport,
        completion_allowed: bool,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO final_audits VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    audit.id.to_string(),
                    mission_id,
                    original_goal,
                    requirements.join("\n"),
                    audit
                        .findings
                        .iter()
                        .map(|finding| finding.code.clone())
                        .collect::<Vec<_>>()
                        .join(","),
                    if audit.passed { 1_i64 } else { 0_i64 },
                    if audit.return_to_repair { 1_i64 } else { 0_i64 },
                    if completion_allowed { 1_i64 } else { 0_i64 },
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn verification_run(&self, id: &str) -> AcResult<Option<VerificationRunRow>> {
        self.connection
            .query_row(
                "SELECT id, profile_id, task_id, commit_ref, worktree_id, environment, command,
                        tool_version, normalized_result, raw_artifact, evidence_ref,
                        freshness_dependencies, created_at_ms
                 FROM verification_runs WHERE id=?1",
                params![id],
                |row| {
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
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn requirement_verifications(
        &self,
        requirement_id: &str,
    ) -> AcResult<Vec<RequirementVerificationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, requirement_id, verification_run_id, evidence_ref, evidence_kind,
                        verified, freshness_key, created_at_ms
                 FROM requirement_verifications WHERE requirement_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![requirement_id], |row| {
                Ok(RequirementVerificationRow {
                    id: row.get(0)?,
                    requirement_id: row.get(1)?,
                    verification_run_id: row.get(2)?,
                    evidence_ref: row.get(3)?,
                    evidence_kind: row.get(4)?,
                    verified: row.get::<_, i64>(5)? != 0,
                    freshness_key: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn final_audits(&self, mission_id: &str) -> AcResult<Vec<FinalAuditRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, original_goal, requirements, evidence_refs, passed,
                        return_to_repair, completion_allowed, created_at_ms
                 FROM final_audits WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![mission_id], |row| {
                Ok(FinalAuditRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    original_goal: row.get(2)?,
                    requirements: row.get(3)?,
                    evidence_refs: row.get(4)?,
                    passed: row.get::<_, i64>(5)? != 0,
                    return_to_repair: row.get::<_, i64>(6)? != 0,
                    completion_allowed: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_browser_process(&self, process: &BrowserProcessRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_processes VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET state=excluded.state",
                params![
                    process.id.to_string(),
                    process.task_id.to_string(),
                    format!("{:?}", process.mode),
                    format!("{:?}", process.state),
                    process.profile_dir,
                    millis(process.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_session(&self, session: &BrowserSessionRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_sessions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    current_url=excluded.current_url,
                    stale_evidence_refs=excluded.stale_evidence_refs,
                    updated_at_ms=excluded.updated_at_ms",
                params![
                    session.id.to_string(),
                    session.task_id.to_string(),
                    session.process_id.to_string(),
                    session.current_url,
                    session.profile,
                    session.storage_state_ref,
                    if session.sensitive { 1_i64 } else { 0_i64 },
                    session
                        .stale_evidence_refs
                        .iter()
                        .map(StableId::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                    millis(session.created_at),
                    millis(session.updated_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_dev_server(&self, server: &DevServerRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_dev_servers VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    server.id.to_string(),
                    server.task_id.to_string(),
                    server.command.join("\n"),
                    server.port,
                    server.ready_url,
                    if server.process_alive { 1_i64 } else { 0_i64 },
                    if server.http_ready { 1_i64 } else { 0_i64 },
                    if server.route_loadable { 1_i64 } else { 0_i64 },
                    if server.retained { 1_i64 } else { 0_i64 }
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_screenshot(&self, screenshot: &ScreenshotEvidence) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_screenshots VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    screenshot.id.to_string(),
                    screenshot.session_id.to_string(),
                    screenshot.task_id.to_string(),
                    screenshot.commit,
                    screenshot.viewport.name,
                    screenshot.url,
                    screenshot.artifact_uri,
                    if screenshot.sensitive { 1_i64 } else { 0_i64 },
                    screenshot.evidence_ref.to_string(),
                    millis(screenshot.captured_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_visual_qa(&self, report: &VisualQaReport) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_visual_qa VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    report.id.to_string(),
                    report.screenshot_ref.to_string(),
                    if report.passed { 1_i64 } else { 0_i64 },
                    report
                        .findings
                        .iter()
                        .map(|finding| finding.code.clone())
                        .collect::<Vec<_>>()
                        .join(","),
                    report.adapter,
                    report.evidence_ref.to_string()
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn browser_session(&self, id: &str) -> AcResult<Option<BrowserSessionRow>> {
        self.connection
            .query_row(
                "SELECT id, task_id, process_id, current_url, profile, storage_state_ref,
                        sensitive, stale_evidence_refs, created_at_ms, updated_at_ms
                 FROM browser_sessions WHERE id=?1",
                params![id],
                |row| {
                    Ok(BrowserSessionRow {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        process_id: row.get(2)?,
                        current_url: row.get(3)?,
                        profile: row.get(4)?,
                        storage_state_ref: row.get(5)?,
                        sensitive: row.get::<_, i64>(6)? != 0,
                        stale_evidence_refs: row.get(7)?,
                        created_at_ms: row.get(8)?,
                        updated_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn browser_screenshots(&self, session_id: &str) -> AcResult<Vec<BrowserScreenshotRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, session_id, task_id, commit_ref, viewport, url, artifact_uri,
                        sensitive, evidence_ref, captured_at_ms
                 FROM browser_screenshots WHERE session_id=?1 ORDER BY captured_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(BrowserScreenshotRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    task_id: row.get(2)?,
                    commit_ref: row.get(3)?,
                    viewport: row.get(4)?,
                    url: row.get(5)?,
                    artifact_uri: row.get(6)?,
                    sensitive: row.get::<_, i64>(7)? != 0,
                    evidence_ref: row.get(8)?,
                    captured_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}
