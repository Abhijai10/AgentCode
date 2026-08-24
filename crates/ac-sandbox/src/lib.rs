use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ac_common::{AcError, AcResult, StableId};
use ac_security::{Capability, CapabilityPolicy, SecurityDecision};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecRequest {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub network: bool,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilesystemRequest {
    pub path: PathBuf,
    pub write: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxedExecutionPlan {
    pub id: StableId,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub allowed_env: BTreeMap<String, String>,
    pub capabilities: Vec<Capability>,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxPolicy {
    pub workspace_roots: Vec<PathBuf>,
    pub capability_policy: CapabilityPolicy,
    pub network_default_allow: bool,
    pub max_timeout_ms: u64,
}

impl SandboxPolicy {
    pub fn new(workspace_roots: Vec<PathBuf>) -> Self {
        Self {
            workspace_roots,
            capability_policy: CapabilityPolicy::new(),
            network_default_allow: false,
            max_timeout_ms: 60_000,
        }
    }
}

pub struct SandboxManager {
    policy: SandboxPolicy,
}

#[derive(Clone, Debug, Default)]
pub struct SecretBroker {
    values: BTreeMap<String, String>,
}

impl SecretBroker {
    pub fn insert(
        &mut self,
        reference: impl Into<String>,
        value: impl Into<String>,
    ) -> AcResult<()> {
        let reference = reference.into();
        let value = value.into();
        if reference.trim().is_empty() || value.is_empty() {
            return Err(AcError::validation(
                "SANDBOX-INVALID_SECRET",
                "secret reference and value are required",
            ));
        }
        self.values.insert(reference, value);
        Ok(())
    }

    pub fn inject(
        &self,
        references: &BTreeMap<String, String>,
    ) -> AcResult<(BTreeMap<String, String>, Vec<String>)> {
        let mut env = BTreeMap::new();
        let mut values = Vec::new();
        for (env_key, reference) in references {
            let value = self.values.get(reference).ok_or_else(|| {
                AcError::validation(
                    "SANDBOX-SECRET_UNAVAILABLE",
                    "approved secret reference is unavailable",
                )
            })?;
            env.insert(env_key.clone(), value.clone());
            values.push(value.clone());
        }
        Ok((env, values))
    }
}

impl SandboxManager {
    pub fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }

    pub fn evaluate_filesystem(&self, request: &FilesystemRequest) -> SecurityDecision {
        if self.path_within_workspace(&request.path) {
            SecurityDecision::Allow
        } else {
            SecurityDecision::Deny
        }
    }

    pub fn resolve_workspace_path(&self, path: &Path, write: bool) -> AcResult<PathBuf> {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            return Err(AcError::policy_denied(
                "SANDBOX-RELATIVE_PATH",
                "filesystem paths must be resolved by the workspace tool",
            ));
        };
        let resolved = if write && !candidate.exists() {
            let parent = candidate.parent().ok_or_else(|| {
                AcError::policy_denied("SANDBOX-PATH_ESCAPE", "path has no parent")
            })?;
            let parent = fs::canonicalize(parent)
                .map_err(|err| AcError::validation("SANDBOX-PATH_RESOLUTION", err.to_string()))?;
            parent.join(candidate.file_name().ok_or_else(|| {
                AcError::validation("SANDBOX-PATH_RESOLUTION", "path has no file name")
            })?)
        } else {
            fs::canonicalize(&candidate)
                .map_err(|err| AcError::validation("SANDBOX-PATH_RESOLUTION", err.to_string()))?
        };
        if !self.path_within_workspace(&resolved) {
            return Err(AcError::policy_denied(
                "SANDBOX-PATH_ESCAPE",
                "resolved path escapes the workspace",
            ));
        }
        Ok(resolved)
    }

    pub fn prepare_execution(&self, request: ExecRequest) -> AcResult<SandboxedExecutionPlan> {
        if request.argv.is_empty() || request.argv.iter().any(|part| part.trim().is_empty()) {
            return Err(AcError::validation(
                "SANDBOX-INVALID_COMMAND",
                "argv must contain non-empty command parts",
            ));
        }
        if request.timeout_ms == 0 || request.timeout_ms > self.policy.max_timeout_ms {
            return Err(AcError::validation(
                "SANDBOX-INVALID_TIMEOUT",
                "timeout must be within policy limits",
            ));
        }
        let cwd = self.resolve_workspace_path(&request.cwd, false)?;
        if !cwd.is_dir() {
            return Err(AcError::policy_denied(
                "SANDBOX-CWD_OUTSIDE_WORKSPACE",
                "execution cwd must be inside an allowed workspace",
            ));
        }
        if !self.policy.network_default_allow && request.network {
            return Err(AcError::policy_denied(
                "SANDBOX-NETWORK_DENIED",
                "network is denied by default",
            ));
        }
        if Self::contains_dangerous_shell(&request.argv) {
            return Err(AcError::policy_denied(
                "SANDBOX-DANGEROUS_SHELL",
                "nested shell command requires explicit policy support",
            ));
        }
        let mut capabilities = vec![Capability::ProcessExec(request.argv[0].clone())];
        if request.network {
            capabilities.push(Capability::Network("*".to_string()));
        }
        match self.policy.capability_policy.evaluate(&capabilities) {
            SecurityDecision::Deny => {
                return Err(AcError::policy_denied(
                    "SANDBOX-CAPABILITY_DENIED",
                    "capability policy denied execution",
                ));
            }
            SecurityDecision::RequireApproval => {
                return Err(AcError::policy_denied(
                    "SANDBOX-APPROVAL_REQUIRED",
                    "execution requires approval before spawn",
                ));
            }
            SecurityDecision::Allow => {}
        }
        Ok(SandboxedExecutionPlan {
            id: StableId::new("exec"),
            argv: request.argv,
            cwd,
            allowed_env: request.env,
            capabilities,
            timeout_ms: request.timeout_ms,
        })
    }

    pub fn prepare_execution_with_secrets(
        &self,
        request: ExecRequest,
        secret_references: &BTreeMap<String, String>,
        secrets: &SecretBroker,
    ) -> AcResult<(SandboxedExecutionPlan, Vec<String>)> {
        let mut plan = self.prepare_execution(request)?;
        let requested = secret_references
            .values()
            .cloned()
            .map(Capability::SecretRead)
            .collect::<Vec<_>>();
        if self.policy.capability_policy.evaluate(&requested) != SecurityDecision::Allow {
            return Err(AcError::policy_denied(
                "SANDBOX-SECRET_DENIED",
                "secret injection requires explicit capability approval",
            ));
        }
        let (secret_env, values) = secrets.inject(secret_references)?;
        plan.allowed_env.extend(secret_env);
        plan.capabilities.extend(requested);
        Ok((plan, values))
    }

    fn path_within_workspace(&self, path: &Path) -> bool {
        self.policy
            .workspace_roots
            .iter()
            .filter_map(|root| fs::canonicalize(root).ok())
            .any(|root| path.starts_with(root))
    }

    fn contains_dangerous_shell(argv: &[String]) -> bool {
        matches!(
            argv.first().map(String::as_str),
            Some("sh" | "bash" | "zsh")
        ) && argv
            .iter()
            .any(|part| part.contains(';') || part.contains("&&") || part.contains("||"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_is_denied_by_default() {
        let root = std::env::temp_dir().join(format!("agentcode-sandbox-{}", StableId::new("t")));
        std::fs::create_dir_all(&root).unwrap();
        let policy = SandboxPolicy {
            capability_policy: CapabilityPolicy::new()
                .allow(Capability::ProcessExec("curl".to_string())),
            ..SandboxPolicy::new(vec![root.clone()])
        };
        let manager = SandboxManager::new(policy);
        let err = manager
            .prepare_execution(ExecRequest {
                argv: vec!["curl".to_string(), "https://example.invalid".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: true,
                timeout_ms: 1000,
            })
            .unwrap_err();
        assert_eq!(err.code(), "SANDBOX-NETWORK_DENIED");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_escape_is_denied() {
        let manager = SandboxManager::new(SandboxPolicy::new(vec![PathBuf::from("/repo")]));
        assert_eq!(
            manager.evaluate_filesystem(&FilesystemRequest {
                path: PathBuf::from("/Users/me/.ssh/id_rsa"),
                write: false
            }),
            SecurityDecision::Deny
        );
    }
}
