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

    pub fn save_release_candidate(&self, row: &ReleaseCandidateRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO release_candidates VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    row.id,
                    row.version,
                    row.candidate_id,
                    row.build_id,
                    row.commit_hash,
                    row.platform_target,
                    row.validation_status,
                    row.evidence_refs,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn release_candidates(&self) -> AcResult<Vec<ReleaseCandidateRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, version, candidate_id, build_id, commit_hash, platform_target,
                 validation_status, evidence_refs, created_at_ms
                 FROM release_candidates ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ReleaseCandidateRow {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    candidate_id: row.get(2)?,
                    build_id: row.get(3)?,
                    commit_hash: row.get(4)?,
                    platform_target: row.get(5)?,
                    validation_status: row.get(6)?,
                    evidence_refs: row.get(7)?,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_release_validation_run(&self, row: &ReleaseValidationRunRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO release_validation_runs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    row.id,
                    row.candidate_id,
                    row.security_status,
                    row.tests_status,
                    row.migration_status,
                    row.artifact_status,
                    row.performance_status,
                    row.release_approval_status,
                    row.evidence_refs,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn release_validation_runs(&self) -> AcResult<Vec<ReleaseValidationRunRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, candidate_id, security_status, tests_status, migration_status,
                 artifact_status, performance_status, release_approval_status, evidence_refs,
                 created_at_ms FROM release_validation_runs ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ReleaseValidationRunRow {
                    id: row.get(0)?,
                    candidate_id: row.get(1)?,
                    security_status: row.get(2)?,
                    tests_status: row.get(3)?,
                    migration_status: row.get(4)?,
                    artifact_status: row.get(5)?,
                    performance_status: row.get(6)?,
                    release_approval_status: row.get(7)?,
                    evidence_refs: row.get(8)?,
                    created_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_release_approval_decision(
        &self,
        row: &ReleaseApprovalDecisionRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO release_approval_decisions VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    row.id,
                    row.approved_version,
                    row.validation_evidence_refs,
                    row.security_status,
                    row.approval_timestamp_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn release_approval_decisions(&self) -> AcResult<Vec<ReleaseApprovalDecisionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, approved_version, validation_evidence_refs, security_status,
                 approval_timestamp_ms FROM release_approval_decisions ORDER BY approval_timestamp_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ReleaseApprovalDecisionRow {
                    id: row.get(0)?,
                    approved_version: row.get(1)?,
                    validation_evidence_refs: row.get(2)?,
                    security_status: row.get(3)?,
                    approval_timestamp_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_final_release_manifest(&self, row: &FinalReleaseManifestRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO final_release_manifests VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    row.id,
                    row.version,
                    row.features,
                    row.migrations,
                    row.artifacts,
                    row.checksums,
                    row.known_limitations,
                    row.manifest_hash,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn final_release_manifests(&self) -> AcResult<Vec<FinalReleaseManifestRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, version, features, migrations, artifacts, checksums,
                 known_limitations, manifest_hash, created_at_ms
                 FROM final_release_manifests ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(FinalReleaseManifestRow {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    features: row.get(2)?,
                    migrations: row.get(3)?,
                    artifacts: row.get(4)?,
                    checksums: row.get(5)?,
                    known_limitations: row.get(6)?,
                    manifest_hash: row.get(7)?,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_release_evidence_bundle(&self, row: &ReleaseEvidenceBundleRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO release_evidence_bundles VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id,
                    row.version,
                    row.audit_report_ref,
                    row.security_report_ref,
                    row.validation_report_ref,
                    row.artifact_report_ref,
                    row.migration_report_ref,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn release_evidence_bundles(&self) -> AcResult<Vec<ReleaseEvidenceBundleRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, version, audit_report_ref, security_report_ref, validation_report_ref,
                 artifact_report_ref, migration_report_ref, created_at_ms
                 FROM release_evidence_bundles ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ReleaseEvidenceBundleRow {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    audit_report_ref: row.get(2)?,
                    security_report_ref: row.get(3)?,
                    validation_report_ref: row.get(4)?,
                    artifact_report_ref: row.get(5)?,
                    migration_report_ref: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}
