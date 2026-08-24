impl ControlPlaneDb {
    pub fn save_skill(&self, skill: &SkillManifest) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO skills VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(id) DO UPDATE SET
                    description=excluded.description,
                    trigger_hints=excluded.trigger_hints,
                    loaded_at_ms=excluded.loaded_at_ms",
                params![
                    skill.id.to_string(),
                    skill.name,
                    skill.description,
                    skill.version,
                    skill.source,
                    format!("{:?}", skill.scope),
                    format!("{:?}", skill.trust_tier),
                    skill.trigger_hints.join(","),
                    format!("{:?}", skill.required_capabilities),
                    skill.context_cost,
                    skill.project_id.as_ref().map(StableId::to_string),
                    skill.task_id.as_ref().map(StableId::to_string),
                    skill.full_instructions,
                    skill.loaded_at.map(millis)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn skill(&self, id: &str) -> AcResult<Option<SkillRow>> {
        self.connection
            .query_row(
                "SELECT id, name, scope, trust_tier, full_instructions FROM skills WHERE id=?1",
                params![id],
                |row| {
                    Ok(SkillRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        scope: row.get(2)?,
                        trust_tier: row.get(3)?,
                        full_instructions: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_hook_manifest(&self, hook: &HookManifest) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO hook_manifests VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET timeout_ms=excluded.timeout_ms",
                params![
                    hook.id.to_string(),
                    hook.extension_id.to_string(),
                    format!("{:?}", hook.event),
                    hook.priority,
                    hook.timeout_ms,
                    hook.idempotency_key,
                    format!("{:?}", hook.failure_policy),
                    format!("{:?}", hook.required_capabilities)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_hook_invocation(&self, invocation: &HookInvocation) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO hook_invocations VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    invocation.id.to_string(),
                    invocation.hook_id.to_string(),
                    format!("{:?}", invocation.event),
                    format!("{:?}", invocation.outcome),
                    invocation.evidence_ref.as_ref().map(StableId::to_string),
                    millis(invocation.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn hook_invocations(&self, hook_id: &str) -> AcResult<Vec<HookInvocationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, hook_id, outcome, evidence_ref
                 FROM hook_invocations WHERE hook_id=?1 ORDER BY created_at_ms",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![hook_id], |row| {
                Ok(HookInvocationRow {
                    id: row.get(0)?,
                    hook_id: row.get(1)?,
                    outcome: row.get(2)?,
                    evidence_ref: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_mcp_server(&self, server: &McpServerRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO mcp_servers VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET health=excluded.health, restart_count=excluded.restart_count, trust_tier=excluded.trust_tier",
                params![
                    server.id.to_string(),
                    server.name,
                    server.version,
                    format!("{:?}", server.transport),
                    format!("{:?}", server.trust_tier),
                    format!("{:?}", server.health),
                    server.restart_count
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_mcp_tool(&self, tool: &McpToolRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO mcp_tools VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET schema_json=excluded.schema_json",
                params![
                    tool.id.to_string(),
                    tool.server_id.to_string(),
                    tool.name,
                    tool.description,
                    tool.schema,
                    format!("{:?}", tool.risk),
                    format!("{:?}", tool.required_capabilities)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_mcp_invocation(&self, invocation: &McpInvocationRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO mcp_invocations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    invocation.id.to_string(),
                    invocation.server_id.to_string(),
                    invocation.tool_id.to_string(),
                    format!("{:?}", invocation.status),
                    invocation.output,
                    invocation.evidence_ref.to_string(),
                    millis(invocation.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn mcp_server(&self, id: &str) -> AcResult<Option<McpServerRow>> {
        self.connection
            .query_row(
                "SELECT id, name, health, restart_count FROM mcp_servers WHERE id=?1",
                params![id],
                |row| {
                    Ok(McpServerRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        health: row.get(2)?,
                        restart_count: row.get::<_, i64>(3)? as u32,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_security_scan(
        &self,
        input: &SecurityScanInput,
        report: &SecurityScanReport,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_threat_models VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET evidence_refs=excluded.evidence_refs",
                params![
                    report.threat_model.id.to_string(),
                    input.repository_id.to_string(),
                    input.commit,
                    report.threat_model.entry_points.join(","),
                    report.threat_model.auth_boundaries.join(","),
                    report.threat_model.data_stores.join(","),
                    report.threat_model.admin_operations.join(","),
                    report.threat_model.cloud_configuration.join(","),
                    report.threat_model.sensitive_assets.join(","),
                    stable_ids_csv(&report.threat_model.evidence_refs)
                ],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "INSERT INTO security_scan_reports VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET missing_adapters=excluded.missing_adapters",
                params![
                    report.id.to_string(),
                    input.repository_id.to_string(),
                    input.commit,
                    format!("{:?}", report.adapters_run),
                    report.missing_adapters.join(","),
                    report.threat_model.id.to_string()
                ],
            )
            .map_err(db_error)?;
        for item in &report.instances {
            self.connection
                .execute(
                    "INSERT INTO security_finding_instances VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                     ON CONFLICT(id) DO NOTHING",
                    params![
                        item.id.to_string(),
                        report.id.to_string(),
                        format!("{:?}", item.adapter),
                        item.rule_id,
                        format!("{:?}", item.severity),
                        item.confidence,
                        format!("{:?}", item.proof_level),
                        item.file_path,
                        item.line,
                        item.fingerprint,
                        item.redacted_evidence,
                        item.raw_evidence_ref.to_string()
                    ],
                )
                .map_err(db_error)?;
        }
        for finding in &report.findings {
            self.connection
                .execute(
                    "INSERT INTO security_findings VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT(id) DO UPDATE SET status=excluded.status",
                    params![
                        finding.id.to_string(),
                        report.id.to_string(),
                        finding.root_cause,
                        format!("{:?}", finding.severity),
                        finding.confidence,
                        finding.exploitability,
                        format!("{:?}", finding.status),
                        finding.affected_code.join(","),
                        stable_ids_csv(&finding.evidence_refs),
                        finding.remediation,
                        stable_ids_csv(&finding.instance_ids)
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn save_security_reports(
        &self,
        scan_id: &StableId,
        bundle: &SecurityReportBundle,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_reports VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scan_id) DO UPDATE SET markdown=excluded.markdown, json_report=excluded.json_report, sarif=excluded.sarif",
                params![scan_id.to_string(), bundle.markdown, bundle.json, bundle.sarif],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_scan(&self, id: &str) -> AcResult<Option<SecurityScanRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, commit_ref, threat_model_id FROM security_scan_reports WHERE id=?1",
                params![id],
                |row| {
                    Ok(SecurityScanRow {
                        id: row.get(0)?,
                        repository_id: row.get(1)?,
                        commit_ref: row.get(2)?,
                        threat_model_id: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn security_findings(&self, scan_id: &str) -> AcResult<Vec<SecurityFindingRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, root_cause, severity, status FROM security_findings
                 WHERE scan_id=?1 ORDER BY root_cause",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![scan_id], |row| {
                Ok(SecurityFindingRow {
                    id: row.get(0)?,
                    root_cause: row.get(1)?,
                    severity: row.get(2)?,
                    status: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    fn configure(&self) -> AcResult<()> {
        self.connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_error)?;
        self.connection
            .busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(db_error)?;
        Ok(())
    }
}
