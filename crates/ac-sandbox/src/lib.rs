use std::collections::BTreeMap;
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
        if !self.path_within_workspace(&request.cwd) {
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
            cwd: request.cwd,
            allowed_env: request.env,
            capabilities,
            timeout_ms: request.timeout_ms,
        })
    }

    fn path_within_workspace(&self, path: &Path) -> bool {
        self.policy
            .workspace_roots
            .iter()
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
        let policy = SandboxPolicy {
            capability_policy: CapabilityPolicy::new()
                .allow(Capability::ProcessExec("curl".to_string())),
            ..SandboxPolicy::new(vec![PathBuf::from("/repo")])
        };
        let manager = SandboxManager::new(policy);
        let err = manager
            .prepare_execution(ExecRequest {
                argv: vec!["curl".to_string(), "https://example.invalid".to_string()],
                cwd: PathBuf::from("/repo"),
                env: BTreeMap::new(),
                network: true,
                timeout_ms: 1000,
            })
            .unwrap_err();
        assert_eq!(err.code(), "SANDBOX-NETWORK_DENIED");
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
