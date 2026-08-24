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

#[derive(Default)]
pub struct ReleaseEngineer {
    artifacts: Vec<ReleaseArtifact>,
    builds: Vec<BuildMetadata>,
    updates: Vec<UpdatePlan>,
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
