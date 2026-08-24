impl ControlPlaneDb {
    pub fn save_dependency_audit(&self, row: &DependencyAuditRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO dependency_audit_records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    row.id,
                    row.name,
                    row.version,
                    row.license,
                    row.source,
                    row.checksum,
                    row.security_status,
                    row.vulnerability_refs,
                    row.release_blocking,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn dependency_audits(&self) -> AcResult<Vec<DependencyAuditRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, name, version, license, source, checksum, security_status,
                 vulnerability_refs, release_blocking, created_at_ms
                 FROM dependency_audit_records ORDER BY name ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DependencyAuditRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    license: row.get(3)?,
                    source: row.get(4)?,
                    checksum: row.get(5)?,
                    security_status: row.get(6)?,
                    vulnerability_refs: row.get(7)?,
                    release_blocking: row.get(8)?,
                    created_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_hardening_report(&self, row: &HardeningReportRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO hardening_reports VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.report_type,
                    row.findings,
                    row.mitigations,
                    row.unresolved_risks,
                    row.accepted_limitations,
                    row.release_blocked,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn hardening_reports(&self) -> AcResult<Vec<HardeningReportRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, report_type, findings, mitigations, unresolved_risks,
                 accepted_limitations, release_blocked, created_at_ms
                 FROM hardening_reports ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(HardeningReportRow {
                    id: row.get(0)?,
                    report_type: row.get(1)?,
                    findings: row.get(2)?,
                    mitigations: row.get(3)?,
                    unresolved_risks: row.get(4)?,
                    accepted_limitations: row.get(5)?,
                    release_blocked: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_release_artifact(&self, row: &ReleaseArtifactRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO release_artifacts VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.version,
                    row.platform,
                    row.artifact_kind,
                    row.build_hash,
                    row.integrity_hash,
                    row.source_commit,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn release_artifacts(&self) -> AcResult<Vec<ReleaseArtifactRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, version, platform, artifact_kind, build_hash, integrity_hash,
                 source_commit, created_at_ms FROM release_artifacts ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ReleaseArtifactRow {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    platform: row.get(2)?,
                    artifact_kind: row.get(3)?,
                    build_hash: row.get(4)?,
                    integrity_hash: row.get(5)?,
                    source_commit: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_release_build(&self, row: &ReleaseBuildRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO release_builds VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    row.id,
                    row.version,
                    row.commit_ref,
                    row.build_profile,
                    row.environment,
                    row.reproducible,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn release_builds(&self) -> AcResult<Vec<ReleaseBuildRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, version, commit_ref, build_profile, environment, reproducible,
                 created_at_ms FROM release_builds ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ReleaseBuildRow {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    commit_ref: row.get(2)?,
                    build_profile: row.get(3)?,
                    environment: row.get(4)?,
                    reproducible: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_update_record(&self, row: &UpdateRecordRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO update_records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.current_version,
                    row.available_version,
                    row.decision,
                    row.verified,
                    row.rollback_ref,
                    row.recovery_action,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn update_records(&self) -> AcResult<Vec<UpdateRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, current_version, available_version, decision, verified, rollback_ref,
                 recovery_action, created_at_ms FROM update_records ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UpdateRecordRow {
                    id: row.get(0)?,
                    current_version: row.get(1)?,
                    available_version: row.get(2)?,
                    decision: row.get(3)?,
                    verified: row.get(4)?,
                    rollback_ref: row.get(5)?,
                    recovery_action: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}
