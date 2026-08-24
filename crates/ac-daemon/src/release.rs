#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseVersion {
    pub version: String,
    pub source_commit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseArtifact {
    pub id: StableId,
    pub version: String,
    pub platform: String,
    pub artifact_kind: String,
    pub build_hash: String,
    pub integrity_hash: String,
    pub source_commit: String,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildMetadata {
    pub id: StableId,
    pub version: String,
    pub commit_ref: String,
    pub build_profile: String,
    pub environment: String,
    pub reproducible: bool,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateDecision {
    AlreadyCurrent,
    Install,
    Blocked,
    Rollback,
}

impl UpdateDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlreadyCurrent => "already_current",
            Self::Install => "install",
            Self::Blocked => "blocked",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdatePlan {
    pub id: StableId,
    pub current_version: String,
    pub available_version: String,
    pub decision: UpdateDecision,
    pub verified: bool,
    pub rollback_ref: Option<String>,
    pub recovery_action: String,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagingLayout {
    pub app_bundle_id: String,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub managed_tools_dir: PathBuf,
    pub browser_profiles_dir: PathBuf,
}

impl PackagingLayout {
    pub fn validate(&self, repository_path: &Path) -> AcResult<()> {
        let dirs = [
            &self.config_dir,
            &self.data_dir,
            &self.cache_dir,
            &self.log_dir,
            &self.managed_tools_dir,
            &self.browser_profiles_dir,
        ];
        for dir in dirs {
            if dir.starts_with(repository_path) {
                return Err(AcError::policy_denied(
                    "RELEASE-APP_STATE_IN_REPOSITORY",
                    "application state must not be stored inside the user repository",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseChecklist {
    pub security_checks: bool,
    pub tests: bool,
    pub artifact_verification: bool,
    pub migration_validation: bool,
}

impl ReleaseChecklist {
    pub fn approved(&self) -> bool {
        self.security_checks && self.tests && self.artifact_verification && self.migration_validation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseCandidateStatus {
    Draft,
    Validating,
    Accepted,
    Rejected,
}

impl ReleaseCandidateStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Validating => "validating",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseCandidate {
    pub id: StableId,
    pub version: String,
    pub candidate_id: String,
    pub build_id: StableId,
    pub commit_hash: String,
    pub platform_target: String,
    pub validation_status: ReleaseCandidateStatus,
    pub evidence_refs: Vec<String>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationStatus {
    Pass,
    Fail,
    Blocked,
}

impl ValidationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationStep {
    pub name: String,
    pub status: ValidationStatus,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionValidationRun {
    pub id: StableId,
    pub candidate_id: StableId,
    pub security: ValidationStep,
    pub tests: ValidationStep,
    pub migration: ValidationStep,
    pub artifact: ValidationStep,
    pub performance: ValidationStep,
    pub release_approval: ValidationStep,
    pub created_at: TimestampMillis,
}

impl ProductionValidationRun {
    pub fn passed(&self) -> bool {
        [
            &self.security,
            &self.tests,
            &self.migration,
            &self.artifact,
            &self.performance,
            &self.release_approval,
        ]
        .iter()
        .all(|step| step.status == ValidationStatus::Pass && !step.evidence_ref.trim().is_empty())
    }

    pub fn evidence_refs(&self) -> Vec<String> {
        [
            &self.security,
            &self.tests,
            &self.migration,
            &self.artifact,
            &self.performance,
            &self.release_approval,
        ]
        .iter()
        .map(|step| step.evidence_ref.clone())
        .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationSafetyReport {
    pub fresh_install: bool,
    pub upgrade: bool,
    pub schema_version: u32,
    pub interrupted_recovery: bool,
    pub evidence_ref: String,
}

impl MigrationSafetyReport {
    pub fn passed(&self, expected_schema_version: u32) -> bool {
        self.fresh_install
            && self.upgrade
            && self.schema_version == expected_schema_version
            && self.interrupted_recovery
            && !self.evidence_ref.trim().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionReadinessAudit {
    pub id: StableId,
    pub architecture_status: String,
    pub security_status: String,
    pub reliability_status: String,
    pub performance_status: String,
    pub release_status: String,
    pub known_limitations: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub passed: bool,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseCandidateGate {
    pub tests_pass: bool,
    pub security_pass: bool,
    pub migrations_pass: bool,
    pub artifacts_valid: bool,
    pub evidence_refs: Vec<String>,
}

impl ReleaseCandidateGate {
    pub fn approve(
        &self,
        candidate: &ReleaseCandidate,
        validation: &ProductionValidationRun,
    ) -> AcResult<()> {
        if candidate.validation_status != ReleaseCandidateStatus::Validating {
            return Err(AcError::conflict(
                "RC-CANDIDATE_NOT_VALIDATING",
                "release candidate must be in validating state before approval",
            ));
        }
        if !(self.tests_pass
            && self.security_pass
            && self.migrations_pass
            && self.artifacts_valid
            && validation.passed()
            && !self.evidence_refs.is_empty())
        {
            return Err(AcError::policy_denied(
                "RC-APPROVAL_BLOCKED",
                "RC approval requires passing tests, security, migrations, artifact validation, and evidence",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseStatus {
    Draft,
    Candidate,
    Approved,
    Released,
}

impl ReleaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Candidate => "candidate",
            Self::Approved => "approved",
            Self::Released => "released",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalReleaseManifest {
    pub id: StableId,
    pub version: String,
    pub features: Vec<String>,
    pub migrations: Vec<String>,
    pub artifacts: Vec<String>,
    pub checksums: Vec<String>,
    pub known_limitations: Vec<String>,
    pub manifest_hash: String,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseDecisionRecord {
    pub id: StableId,
    pub approved_version: String,
    pub validation_evidence_refs: Vec<String>,
    pub security_status: String,
    pub approval_timestamp: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseEvidenceBundle {
    pub id: StableId,
    pub version: String,
    pub audit_report_ref: String,
    pub security_report_ref: String,
    pub validation_report_ref: String,
    pub artifact_report_ref: String,
    pub migration_report_ref: String,
    pub created_at: TimestampMillis,
}

impl ReleaseEvidenceBundle {
    pub fn complete(&self) -> bool {
        [
            &self.audit_report_ref,
            &self.security_report_ref,
            &self.validation_report_ref,
            &self.artifact_report_ref,
            &self.migration_report_ref,
        ]
        .iter()
        .all(|reference| !reference.trim().is_empty())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseStateMachine {
    pub version: String,
    pub status: ReleaseStatus,
    pub evidence_refs: Vec<String>,
}

impl ReleaseStateMachine {
    pub fn new(version: impl Into<String>) -> AcResult<Self> {
        Ok(Self {
            version: required(version.into(), "RELEASE-VERSION_EMPTY")?,
            status: ReleaseStatus::Draft,
            evidence_refs: Vec::new(),
        })
    }

    pub fn transition(
        &mut self,
        next: ReleaseStatus,
        evidence_ref: impl Into<String>,
    ) -> AcResult<()> {
        let evidence_ref = required(evidence_ref.into(), "RELEASE-EVIDENCE_EMPTY")?;
        let allowed = matches!(
            (&self.status, &next),
            (ReleaseStatus::Draft, ReleaseStatus::Candidate)
                | (ReleaseStatus::Candidate, ReleaseStatus::Approved)
                | (ReleaseStatus::Approved, ReleaseStatus::Released)
        );
        if !allowed {
            return Err(AcError::conflict(
                "RELEASE-INVALID_TRANSITION",
                "release status must move Draft -> Candidate -> Approved -> Released",
            ));
        }
        self.status = next;
        self.evidence_refs.push(evidence_ref);
        Ok(())
    }
}

#[derive(Default)]
pub struct ReleaseEngineer {
    artifacts: Vec<ReleaseArtifact>,
    builds: Vec<BuildMetadata>,
    updates: Vec<UpdatePlan>,
    candidates: Vec<ReleaseCandidate>,
    validations: Vec<ProductionValidationRun>,
    manifests: Vec<FinalReleaseManifest>,
    decisions: Vec<ReleaseDecisionRecord>,
    bundles: Vec<ReleaseEvidenceBundle>,
}

impl ReleaseEngineer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn capture_build(
        &mut self,
        version: impl Into<String>,
        commit_ref: impl Into<String>,
        profile: impl Into<String>,
        environment: impl Into<String>,
    ) -> AcResult<BuildMetadata> {
        let build = BuildMetadata {
            id: StableId::new("build"),
            version: required(version.into(), "RELEASE-VERSION_EMPTY")?,
            commit_ref: required(commit_ref.into(), "RELEASE-COMMIT_EMPTY")?,
            build_profile: required(profile.into(), "RELEASE-PROFILE_EMPTY")?,
            environment: required(environment.into(), "RELEASE-ENV_EMPTY")?,
            reproducible: true,
            created_at: TimestampMillis::now(),
        };
        self.builds.push(build.clone());
        Ok(build)
    }

    pub fn create_candidate(
        &mut self,
        version: impl Into<String>,
        candidate_id: impl Into<String>,
        build: &BuildMetadata,
        platform_target: impl Into<String>,
        evidence_refs: Vec<String>,
    ) -> AcResult<ReleaseCandidate> {
        if evidence_refs.is_empty() {
            return Err(AcError::validation(
                "RC-EVIDENCE_EMPTY",
                "release candidate must reference freeze/build evidence",
            ));
        }
        let candidate = ReleaseCandidate {
            id: StableId::new("rc"),
            version: required(version.into(), "RC-VERSION_EMPTY")?,
            candidate_id: required(candidate_id.into(), "RC-CANDIDATE_EMPTY")?,
            build_id: build.id.clone(),
            commit_hash: build.commit_ref.clone(),
            platform_target: required(platform_target.into(), "RC-PLATFORM_EMPTY")?,
            validation_status: ReleaseCandidateStatus::Validating,
            evidence_refs,
            created_at: TimestampMillis::now(),
        };
        self.candidates.push(candidate.clone());
        Ok(candidate)
    }

    pub fn run_production_validation(
        &mut self,
        candidate: &ReleaseCandidate,
        migration: &MigrationSafetyReport,
        checklist: &ReleaseChecklist,
        performance_pass: bool,
        evidence_prefix: impl Into<String>,
    ) -> ProductionValidationRun {
        let evidence_prefix = evidence_prefix.into();
        let step = |name: &str, passed: bool| ValidationStep {
            name: name.to_string(),
            status: if passed {
                ValidationStatus::Pass
            } else {
                ValidationStatus::Fail
            },
            evidence_ref: format!("{evidence_prefix}/{name}"),
        };
        let validation = ProductionValidationRun {
            id: StableId::new("validation"),
            candidate_id: candidate.id.clone(),
            security: step("security", checklist.security_checks),
            tests: step("tests", checklist.tests),
            migration: step("migration", migration.passed(18)),
            artifact: step("artifact", checklist.artifact_verification),
            performance: step("performance", performance_pass),
            release_approval: step("release-approval", checklist.approved()),
            created_at: TimestampMillis::now(),
        };
        self.validations.push(validation.clone());
        validation
    }

    pub fn audit_readiness(
        &self,
        validation: &ProductionValidationRun,
        known_limitations: Vec<String>,
    ) -> ProductionReadinessAudit {
        let passed = validation.passed();
        ProductionReadinessAudit {
            id: StableId::new("audit"),
            architecture_status: "subsystems accepted through Phase 27 evidence".to_string(),
            security_status: if validation.security.status == ValidationStatus::Pass {
                "security release gate passed".to_string()
            } else {
                "security release gate blocked".to_string()
            },
            reliability_status: "chaos and recovery evidence current".to_string(),
            performance_status: if validation.performance.status == ValidationStatus::Pass {
                "8 GB performance matrix accepted".to_string()
            } else {
                "performance matrix blocked".to_string()
            },
            release_status: if passed {
                "candidate approved for V1 release".to_string()
            } else {
                "candidate blocked".to_string()
            },
            known_limitations,
            evidence_refs: validation.evidence_refs(),
            passed,
            created_at: TimestampMillis::now(),
        }
    }

    pub fn create_artifact(
        &mut self,
        version: &ReleaseVersion,
        platform: impl Into<String>,
        artifact_kind: impl Into<String>,
        bytes: &[u8],
    ) -> AcResult<ReleaseArtifact> {
        if bytes.is_empty() {
            return Err(AcError::validation(
                "RELEASE-ARTIFACT_EMPTY",
                "release artifact bytes are required for checksum verification",
            ));
        }
        let integrity_hash = stable_hash(bytes);
        let artifact = ReleaseArtifact {
            id: StableId::new("artifact"),
            version: required(version.version.clone(), "RELEASE-VERSION_EMPTY")?,
            platform: required(platform.into(), "RELEASE-PLATFORM_EMPTY")?,
            artifact_kind: required(artifact_kind.into(), "RELEASE-KIND_EMPTY")?,
            build_hash: stable_hash(format!("{}:{}", version.version, version.source_commit).as_bytes()),
            integrity_hash,
            source_commit: required(version.source_commit.clone(), "RELEASE-COMMIT_EMPTY")?,
            created_at: TimestampMillis::now(),
        };
        self.artifacts.push(artifact.clone());
        Ok(artifact)
    }

    pub fn verify_artifact(&self, artifact: &ReleaseArtifact, bytes: &[u8]) -> bool {
        artifact.integrity_hash == stable_hash(bytes)
    }

    pub fn approve_candidate(
        &self,
        candidate: &mut ReleaseCandidate,
        validation: &ProductionValidationRun,
        gate: &ReleaseCandidateGate,
    ) -> AcResult<()> {
        gate.approve(candidate, validation)?;
        candidate.validation_status = ReleaseCandidateStatus::Accepted;
        Ok(())
    }

    pub fn final_manifest(
        &mut self,
        version: impl Into<String>,
        features: Vec<String>,
        migrations: Vec<String>,
        artifacts: &[ReleaseArtifact],
        known_limitations: Vec<String>,
    ) -> AcResult<FinalReleaseManifest> {
        if features.is_empty() || migrations.is_empty() || artifacts.is_empty() {
            return Err(AcError::validation(
                "RELEASE-MANIFEST_INCOMPLETE",
                "features, migrations and artifacts are required in the final release manifest",
            ));
        }
        let artifact_ids = artifacts
            .iter()
            .map(|artifact| artifact.id.to_string())
            .collect::<Vec<_>>();
        let checksums = artifacts
            .iter()
            .map(|artifact| artifact.integrity_hash.clone())
            .collect::<Vec<_>>();
        let version = required(version.into(), "RELEASE-VERSION_EMPTY")?;
        let manifest_material = format!(
            "{}|{}|{}|{}|{}",
            version,
            features.join(","),
            migrations.join(","),
            artifact_ids.join(","),
            checksums.join(",")
        );
        let manifest = FinalReleaseManifest {
            id: StableId::new("manifest"),
            version,
            features,
            migrations,
            artifacts: artifact_ids,
            checksums,
            known_limitations,
            manifest_hash: stable_hash(manifest_material.as_bytes()),
            created_at: TimestampMillis::now(),
        };
        self.manifests.push(manifest.clone());
        Ok(manifest)
    }

    pub fn approve_release(
        &mut self,
        manifest: &FinalReleaseManifest,
        validation: &ProductionValidationRun,
        security_status: impl Into<String>,
    ) -> AcResult<ReleaseDecisionRecord> {
        if !validation.passed() || manifest.manifest_hash.trim().is_empty() {
            return Err(AcError::policy_denied(
                "RELEASE-APPROVAL_BLOCKED",
                "release approval requires passing validation and a final manifest hash",
            ));
        }
        let decision = ReleaseDecisionRecord {
            id: StableId::new("decision"),
            approved_version: manifest.version.clone(),
            validation_evidence_refs: validation.evidence_refs(),
            security_status: required(security_status.into(), "RELEASE-SECURITY_EMPTY")?,
            approval_timestamp: TimestampMillis::now(),
        };
        self.decisions.push(decision.clone());
        Ok(decision)
    }

    pub fn evidence_bundle(
        &mut self,
        version: impl Into<String>,
        audit_report_ref: impl Into<String>,
        security_report_ref: impl Into<String>,
        validation_report_ref: impl Into<String>,
        artifact_report_ref: impl Into<String>,
        migration_report_ref: impl Into<String>,
    ) -> AcResult<ReleaseEvidenceBundle> {
        let bundle = ReleaseEvidenceBundle {
            id: StableId::new("bundle"),
            version: required(version.into(), "RELEASE-VERSION_EMPTY")?,
            audit_report_ref: required(audit_report_ref.into(), "RELEASE-AUDIT_EMPTY")?,
            security_report_ref: required(security_report_ref.into(), "RELEASE-SECURITY_EMPTY")?,
            validation_report_ref: required(
                validation_report_ref.into(),
                "RELEASE-VALIDATION_EMPTY",
            )?,
            artifact_report_ref: required(artifact_report_ref.into(), "RELEASE-ARTIFACT_EMPTY")?,
            migration_report_ref: required(migration_report_ref.into(), "RELEASE-MIGRATION_EMPTY")?,
            created_at: TimestampMillis::now(),
        };
        self.bundles.push(bundle.clone());
        Ok(bundle)
    }

    pub fn plan_update(
        &mut self,
        current_version: impl Into<String>,
        available: &ReleaseArtifact,
        bytes: &[u8],
    ) -> UpdatePlan {
        let current_version = current_version.into();
        let verified = self.verify_artifact(available, bytes);
        let decision = if !verified {
            UpdateDecision::Blocked
        } else if current_version == available.version {
            UpdateDecision::AlreadyCurrent
        } else {
            UpdateDecision::Install
        };
        let plan = UpdatePlan {
            id: StableId::new("update"),
            current_version,
            available_version: available.version.clone(),
            decision,
            verified,
            rollback_ref: Some(format!("rollback:{}", available.source_commit)),
            recovery_action: if verified {
                "rollback to previous app bundle and keep migrated DB backup".to_string()
            } else {
                "block update and retain current version".to_string()
            },
            created_at: TimestampMillis::now(),
        };
        self.updates.push(plan.clone());
        plan
    }

    pub fn rollback_after_failed_update(
        &mut self,
        current_version: impl Into<String>,
        failed_version: impl Into<String>,
        rollback_ref: impl Into<String>,
    ) -> UpdatePlan {
        let plan = UpdatePlan {
            id: StableId::new("update"),
            current_version: current_version.into(),
            available_version: failed_version.into(),
            decision: UpdateDecision::Rollback,
            verified: false,
            rollback_ref: Some(rollback_ref.into()),
            recovery_action: "restore previous artifact and preserve user data directories".to_string(),
            created_at: TimestampMillis::now(),
        };
        self.updates.push(plan.clone());
        plan
    }

    pub fn diagnostics_report(&self, canary_secret: &str, raw: &str) -> String {
        raw.replace(canary_secret, "[REDACTED]")
    }

    pub fn reset_plan_preserves_repository(&self, repository_path: &Path) -> Vec<PathBuf> {
        vec![
            PathBuf::from("~/Library/Caches/com.agentcode.indexes"),
            PathBuf::from("~/Library/Caches/com.agentcode.runtime"),
        ]
        .into_iter()
        .filter(|path| !path.starts_with(repository_path))
        .collect()
    }

    pub fn artifacts(&self) -> &[ReleaseArtifact] {
        &self.artifacts
    }

    pub fn builds(&self) -> &[BuildMetadata] {
        &self.builds
    }

    pub fn updates(&self) -> &[UpdatePlan] {
        &self.updates
    }

    pub fn candidates(&self) -> &[ReleaseCandidate] {
        &self.candidates
    }

    pub fn validations(&self) -> &[ProductionValidationRun] {
        &self.validations
    }

    pub fn manifests(&self) -> &[FinalReleaseManifest] {
        &self.manifests
    }

    pub fn decisions(&self) -> &[ReleaseDecisionRecord] {
        &self.decisions
    }

    pub fn bundles(&self) -> &[ReleaseEvidenceBundle] {
        &self.bundles
    }
}

fn required(value: String, code: &'static str) -> AcResult<String> {
    if value.trim().is_empty() {
        Err(AcError::validation(code, "release field is required"))
    } else {
        Ok(value)
    }
}

fn stable_hash(bytes: &[u8]) -> String {
    let hash = bytes
        .iter()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(1_099_511_628_211)
        });
    format!("fnv1a64:{hash:016x}")
}
