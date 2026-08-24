use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum IsolationLevel {
    None,
    ProcessRestricted,
    FilesystemIsolated,
    NetworkIsolated,
    Strong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkPolicy {
    DenyAll,
    AllowAll,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxBackendCapabilities {
    pub backend_name: String,
    pub max_isolation: IsolationLevel,
    pub filesystem_isolation: bool,
    pub network_isolation: bool,
    pub process_tree_control: bool,
    pub env_isolation: bool,
    pub output_limit: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxEvidence {
    pub backend_name: String,
    pub requested_isolation: IsolationLevel,
    pub achieved_isolation: IsolationLevel,
    pub workspace_roots: Vec<PathBuf>,
    pub network_policy: NetworkPolicy,
    pub allowed_env_keys: Vec<String>,
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
    pub process_tree_control: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedSandboxExecution {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub allowed_env: BTreeMap<String, String>,
    pub evidence: SandboxEvidence,
}

pub trait SandboxBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> SandboxBackendCapabilities;
    fn prepare(&self, request: &SandboxBackendRequest) -> AcResult<PreparedSandboxExecution>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxBackendRequest {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub allowed_env: BTreeMap<String, String>,
    pub workspace_roots: Vec<PathBuf>,
    pub timeout_ms: u64,
    pub network_policy: NetworkPolicy,
    pub required_isolation: IsolationLevel,
    pub max_output_bytes: usize,
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
    pub backend_argv: Vec<String>,
    pub cwd: PathBuf,
    pub allowed_env: BTreeMap<String, String>,
    pub capabilities: Vec<Capability>,
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
    pub sandbox_evidence: SandboxEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxPolicy {
    pub workspace_roots: Vec<PathBuf>,
    pub capability_policy: CapabilityPolicy,
    pub network_default_allow: bool,
    pub max_timeout_ms: u64,
    pub required_isolation: IsolationLevel,
    pub max_output_bytes: usize,
}

impl SandboxPolicy {
    pub fn new(workspace_roots: Vec<PathBuf>) -> Self {
        Self {
            workspace_roots,
            capability_policy: CapabilityPolicy::new(),
            network_default_allow: false,
            max_timeout_ms: 60_000,
            required_isolation: IsolationLevel::FilesystemIsolated,
            max_output_bytes: 1024 * 1024,
        }
    }
}

pub struct SandboxManager {
    policy: SandboxPolicy,
    backend: Box<dyn SandboxBackend>,
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
        Self {
            policy,
            backend: default_backend(),
        }
    }

    pub fn with_backend(policy: SandboxPolicy, backend: Box<dyn SandboxBackend>) -> Self {
        Self { policy, backend }
    }

    pub fn diagnostics(&self) -> SandboxBackendCapabilities {
        self.backend.capabilities()
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
        let network_policy = if request.network {
            NetworkPolicy::AllowAll
        } else {
            NetworkPolicy::DenyAll
        };
        let prepared = self.backend.prepare(&SandboxBackendRequest {
            argv: request.argv.clone(),
            cwd: cwd.clone(),
            allowed_env: request.env.clone(),
            workspace_roots: self.policy.workspace_roots.clone(),
            timeout_ms: request.timeout_ms,
            network_policy,
            required_isolation: self.policy.required_isolation,
            max_output_bytes: self.policy.max_output_bytes,
        })?;
        Ok(SandboxedExecutionPlan {
            id: StableId::new("exec"),
            argv: request.argv,
            backend_argv: prepared.argv,
            cwd,
            allowed_env: prepared.allowed_env,
            capabilities,
            timeout_ms: request.timeout_ms,
            max_output_bytes: self.policy.max_output_bytes,
            sandbox_evidence: prepared.evidence,
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

fn default_backend() -> Box<dyn SandboxBackend> {
    #[cfg(target_os = "macos")]
    {
        if macos_sandbox_exec_usable() {
            Box::new(MacosSandboxExecBackend)
        } else {
            Box::new(ProcessRestrictedBackend)
        }
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxBubblewrapBackend)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Box::new(UnsupportedSandboxBackend)
    }
}

#[derive(Clone, Debug)]
pub struct UnsupportedSandboxBackend;

impl SandboxBackend for UnsupportedSandboxBackend {
    fn name(&self) -> &'static str {
        "unsupported"
    }

    fn capabilities(&self) -> SandboxBackendCapabilities {
        SandboxBackendCapabilities {
            backend_name: self.name().to_string(),
            max_isolation: IsolationLevel::None,
            filesystem_isolation: false,
            network_isolation: false,
            process_tree_control: false,
            env_isolation: true,
            output_limit: true,
        }
    }

    fn prepare(&self, _request: &SandboxBackendRequest) -> AcResult<PreparedSandboxExecution> {
        Err(AcError::new(
            "SANDBOX-UNAVAILABLE",
            "no supported OS sandbox backend is available; execution blocked",
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::NotRetryable,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct ProcessRestrictedBackend;

impl SandboxBackend for ProcessRestrictedBackend {
    fn name(&self) -> &'static str {
        "process-restricted"
    }

    fn capabilities(&self) -> SandboxBackendCapabilities {
        SandboxBackendCapabilities {
            backend_name: self.name().to_string(),
            max_isolation: IsolationLevel::ProcessRestricted,
            filesystem_isolation: false,
            network_isolation: false,
            process_tree_control: true,
            env_isolation: true,
            output_limit: true,
        }
    }

    fn prepare(&self, request: &SandboxBackendRequest) -> AcResult<PreparedSandboxExecution> {
        if request.required_isolation > IsolationLevel::ProcessRestricted {
            return Err(AcError::policy_denied(
                "SANDBOX-ISOLATION_UNSUPPORTED",
                "filesystem/network sandbox isolation is unavailable; execution blocked",
            ));
        }
        Ok(PreparedSandboxExecution {
            argv: request.argv.clone(),
            cwd: request.cwd.clone(),
            allowed_env: request.allowed_env.clone(),
            evidence: evidence_for(self.name(), request, IsolationLevel::ProcessRestricted),
        })
    }
}

#[derive(Clone, Debug)]
pub struct MacosSandboxExecBackend;

impl SandboxBackend for MacosSandboxExecBackend {
    fn name(&self) -> &'static str {
        "macos-sandbox-exec"
    }

    fn capabilities(&self) -> SandboxBackendCapabilities {
        let available = command_available("sandbox-exec");
        SandboxBackendCapabilities {
            backend_name: self.name().to_string(),
            max_isolation: if available {
                IsolationLevel::FilesystemIsolated
            } else {
                IsolationLevel::None
            },
            filesystem_isolation: available,
            network_isolation: available,
            process_tree_control: true,
            env_isolation: true,
            output_limit: true,
        }
    }

    fn prepare(&self, request: &SandboxBackendRequest) -> AcResult<PreparedSandboxExecution> {
        if request.required_isolation <= IsolationLevel::ProcessRestricted {
            return ProcessRestrictedBackend.prepare(request);
        }
        if !command_available("sandbox-exec") {
            return Err(AcError::new(
                "SANDBOX-UNAVAILABLE",
                "sandbox-exec is unavailable; execution blocked",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            ));
        }
        let caps = self.capabilities();
        if caps.max_isolation < request.required_isolation {
            return Err(AcError::policy_denied(
                "SANDBOX-ISOLATION_UNSUPPORTED",
                "requested isolation exceeds macOS backend capability",
            ));
        }
        let profile = write_macos_profile(request)?;
        let mut argv = vec![
            "/usr/bin/sandbox-exec".to_string(),
            "-f".to_string(),
            profile.display().to_string(),
        ];
        argv.extend(resolve_backend_argv(request)?);
        Ok(PreparedSandboxExecution {
            argv,
            cwd: request.cwd.clone(),
            allowed_env: request.allowed_env.clone(),
            evidence: evidence_for(self.name(), request, IsolationLevel::FilesystemIsolated),
        })
    }
}

#[derive(Clone, Debug)]
pub struct LinuxBubblewrapBackend;

impl SandboxBackend for LinuxBubblewrapBackend {
    fn name(&self) -> &'static str {
        "linux-bubblewrap"
    }

    fn capabilities(&self) -> SandboxBackendCapabilities {
        let available = command_available("bwrap");
        SandboxBackendCapabilities {
            backend_name: self.name().to_string(),
            max_isolation: if available {
                IsolationLevel::Strong
            } else {
                IsolationLevel::None
            },
            filesystem_isolation: available,
            network_isolation: available,
            process_tree_control: available,
            env_isolation: true,
            output_limit: true,
        }
    }

    fn prepare(&self, request: &SandboxBackendRequest) -> AcResult<PreparedSandboxExecution> {
        if request.required_isolation <= IsolationLevel::ProcessRestricted {
            return ProcessRestrictedBackend.prepare(request);
        }
        if !command_available("bwrap") {
            return Err(AcError::new(
                "SANDBOX-UNAVAILABLE",
                "bubblewrap is unavailable; execution blocked",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            ));
        }
        let caps = self.capabilities();
        if caps.max_isolation < request.required_isolation {
            return Err(AcError::policy_denied(
                "SANDBOX-ISOLATION_UNSUPPORTED",
                "requested isolation exceeds Linux backend capability",
            ));
        }
        let mut argv = vec![
            "bwrap".to_string(),
            "--unshare-all".to_string(),
            "--die-with-parent".to_string(),
            "--new-session".to_string(),
            "--proc".to_string(),
            "/proc".to_string(),
            "--tmpfs".to_string(),
            "/tmp".to_string(),
            "--ro-bind".to_string(),
            "/usr".to_string(),
            "/usr".to_string(),
            "--ro-bind".to_string(),
            "/bin".to_string(),
            "/bin".to_string(),
            "--dev".to_string(),
            "/dev".to_string(),
        ];
        for root in &request.workspace_roots {
            argv.extend([
                "--bind".to_string(),
                root.display().to_string(),
                root.display().to_string(),
            ]);
        }
        argv.extend(["--chdir".to_string(), request.cwd.display().to_string()]);
        argv.push("--".to_string());
        argv.extend(request.argv.clone());
        Ok(PreparedSandboxExecution {
            argv,
            cwd: request.cwd.clone(),
            allowed_env: request.allowed_env.clone(),
            evidence: evidence_for(self.name(), request, IsolationLevel::Strong),
        })
    }
}

fn command_available(command: &str) -> bool {
    Command::new("/usr/bin/which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn macos_sandbox_exec_usable() -> bool {
    let path = std::env::temp_dir().join(format!(
        "agentcode-sandbox-probe-{}.sb",
        StableId::new("probe")
    ));
    if fs::write(&path, "(version 1)\n(allow default)\n").is_err() {
        return false;
    }
    let usable = Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(&path)
        .arg("/usr/bin/true")
        .env_clear()
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    let _ = fs::remove_file(path);
    usable
}

fn write_macos_profile(request: &SandboxBackendRequest) -> AcResult<PathBuf> {
    let profile_path =
        std::env::temp_dir().join(format!("agentcode-sandbox-{}.sb", StableId::new("profile")));
    let mut profile =
        String::from("(version 1)\n(deny default)\n(allow process*)\n(allow sysctl-read)\n");
    profile.push_str("(allow file-read* (literal \"/dev/null\") (literal \"/dev/urandom\") (subpath \"/usr\") (subpath \"/bin\") (subpath \"/System\") (subpath \"/Library\"))\n");
    if let Ok(home) = std::env::var("HOME") {
        let home = escape_profile_string(&home);
        profile.push_str(&format!(
            "(deny file-read* (subpath \"{home}/.ssh\") (subpath \"{home}/.aws\") (subpath \"{home}/.config/gcloud\"))\n"
        ));
    }
    for key in ["CARGO_HOME", "RUSTUP_HOME"] {
        if let Some(path) = request.allowed_env.get(key) {
            allow_profile_subpath(&mut profile, path, true);
        }
    }
    if let Some(path) = request.allowed_env.get("PATH") {
        for dir in std::env::split_paths(path) {
            allow_profile_subpath(&mut profile, &dir.display().to_string(), false);
        }
    }
    for root in &request.workspace_roots {
        let root = fs::canonicalize(root)
            .map_err(|error| AcError::validation("SANDBOX-PATH_RESOLUTION", error.to_string()))?;
        profile.push_str(&format!(
            "(allow file-read* file-write* (subpath \"{}\"))\n",
            escape_profile_string(&root.display().to_string())
        ));
    }
    let temp = std::env::temp_dir();
    profile.push_str(&format!(
        "(allow file-read* file-write* (subpath \"{}\"))\n",
        escape_profile_string(&temp.display().to_string())
    ));
    if request.network_policy == NetworkPolicy::AllowAll {
        profile.push_str("(allow network*)\n");
    }
    fs::write(&profile_path, profile)
        .map_err(|error| AcError::validation("SANDBOX-PROFILE_WRITE_FAILED", error.to_string()))?;
    Ok(profile_path)
}

fn resolve_backend_argv(request: &SandboxBackendRequest) -> AcResult<Vec<String>> {
    let Some(program) = request.argv.first() else {
        return Ok(Vec::new());
    };
    let mut argv = request.argv.clone();
    if program.contains('/') {
        return Ok(argv);
    }
    let path = request
        .allowed_env
        .get("PATH")
        .map(String::as_str)
        .unwrap_or("");
    for dir in std::env::split_paths(path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            argv[0] = candidate.display().to_string();
            return Ok(argv);
        }
    }
    Err(AcError::new(
        "SANDBOX-COMMAND_NOT_FOUND",
        format!("command `{program}` was not found in the sandbox PATH"),
        ac_common::ErrorKind::Validation,
        ac_common::Retryability::NotRetryable,
    ))
}

fn allow_profile_subpath(profile: &mut String, path: &str, writable: bool) {
    let path = PathBuf::from(path);
    let resolved = fs::canonicalize(&path).unwrap_or(path);
    let access = if writable {
        "file-read* file-write*"
    } else {
        "file-read*"
    };
    profile.push_str(&format!(
        "(allow {access} (subpath \"{}\"))\n",
        escape_profile_string(&resolved.display().to_string())
    ));
}

fn escape_profile_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn evidence_for(
    backend_name: &str,
    request: &SandboxBackendRequest,
    achieved_isolation: IsolationLevel,
) -> SandboxEvidence {
    SandboxEvidence {
        backend_name: backend_name.to_string(),
        requested_isolation: request.required_isolation,
        achieved_isolation,
        workspace_roots: request.workspace_roots.clone(),
        network_policy: request.network_policy,
        allowed_env_keys: request.allowed_env.keys().cloned().collect(),
        timeout_ms: request.timeout_ms,
        max_output_bytes: request.max_output_bytes,
        process_tree_control: true,
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

    #[test]
    fn unavailable_filesystem_isolation_fails_closed() {
        let root = std::env::temp_dir().join(format!("agentcode-sandbox-{}", StableId::new("t")));
        std::fs::create_dir_all(&root).unwrap();
        let manager = SandboxManager::with_backend(
            SandboxPolicy {
                capability_policy: CapabilityPolicy::new()
                    .allow(Capability::ProcessExec("*".to_string())),
                ..SandboxPolicy::new(vec![root.clone()])
            },
            Box::new(ProcessRestrictedBackend),
        );
        let err = manager
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/echo".to_string(), "blocked".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 1_000,
            })
            .unwrap_err();
        assert_eq!(err.code(), "SANDBOX-ISOLATION_UNSUPPORTED");
        let _ = std::fs::remove_dir_all(root);
    }
}
