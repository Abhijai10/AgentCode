use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::{EvidenceKind, EvidenceStore, Provenance};
use ac_sandbox::{ExecRequest, SandboxManager, SandboxPolicy};
use ac_security::{Capability, CapabilityPolicy, SecurityDecision};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDefinition {
    pub id: String,
    pub version: String,
    pub required_capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolRequest {
    pub id: StableId,
    pub tool_id: String,
    pub tool_version: String,
    pub payload: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolStatus {
    Succeeded,
    Failed,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResult {
    pub request_id: StableId,
    pub status: ToolStatus,
    pub observation: String,
    pub evidence_ref: StableId,
    pub finished_at: TimestampMillis,
}

pub trait ToolExecutor {
    fn execute(&self, request: &ToolRequest) -> AcResult<String>;
}

#[derive(Clone, Debug)]
pub struct WorkspaceTools {
    root: PathBuf,
}

impl WorkspaceTools {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn register_all(&self, broker: &mut ToolBroker) -> AcResult<()> {
        broker.register_tool(
            ToolDefinition {
                id: "fs.list".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::FilesystemRead(
                    self.root.display().to_string(),
                )],
            },
            Box::new(ListDirectoryTool {
                root: self.root.clone(),
            }),
        )?;
        broker.register_tool(
            ToolDefinition {
                id: "fs.read".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::FilesystemRead(
                    self.root.display().to_string(),
                )],
            },
            Box::new(ReadFileTool {
                root: self.root.clone(),
            }),
        )?;
        broker.register_tool(
            ToolDefinition {
                id: "fs.write".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::FilesystemWrite(
                    self.root.display().to_string(),
                )],
            },
            Box::new(WriteFileTool {
                root: self.root.clone(),
            }),
        )?;
        broker.register_tool(
            ToolDefinition {
                id: "fs.search".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::FilesystemRead(
                    self.root.display().to_string(),
                )],
            },
            Box::new(SearchTool {
                root: self.root.clone(),
            }),
        )?;
        broker.register_tool(
            ToolDefinition {
                id: "cmd.exec".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::ProcessExec("*".to_string())],
            },
            Box::new(CommandExecTool {
                sandbox: SandboxManager::new(SandboxPolicy {
                    workspace_roots: vec![self.root.clone()],
                    capability_policy: CapabilityPolicy::new()
                        .allow(Capability::ProcessExec("*".to_string())),
                    network_default_allow: false,
                    max_timeout_ms: 30_000,
                }),
                cwd: self.root.clone(),
            }),
        )?;
        Ok(())
    }
}

struct ListDirectoryTool {
    root: PathBuf,
}

impl ToolExecutor for ListDirectoryTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let path = safe_join(&self.root, &request.payload)?;
        let mut entries = fs::read_dir(path)
            .map_err(|err| AcError::validation("TOOL-FS_LIST_FAILED", err.to_string()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name().to_string_lossy().to_string())
                    .map_err(|err| AcError::validation("TOOL-FS_LIST_FAILED", err.to_string()))
            })
            .collect::<AcResult<Vec<_>>>()?;
        entries.sort();
        Ok(entries.join("\n"))
    }
}

struct ReadFileTool {
    root: PathBuf,
}

impl ToolExecutor for ReadFileTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let path = safe_join(&self.root, &request.payload)?;
        fs::read_to_string(path)
            .map_err(|err| AcError::validation("TOOL-FS_READ_FAILED", err.to_string()))
    }
}

struct WriteFileTool {
    root: PathBuf,
}

impl ToolExecutor for WriteFileTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let (relative_path, content) = request.payload.split_once('\n').ok_or_else(|| {
            AcError::validation(
                "TOOL-FS_WRITE_PAYLOAD",
                "write payload must be '<relative-path>\\n<content>'",
            )
        })?;
        let path = safe_join(&self.root, relative_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| AcError::validation("TOOL-FS_WRITE_FAILED", err.to_string()))?;
        }
        fs::write(&path, content)
            .map_err(|err| AcError::validation("TOOL-FS_WRITE_FAILED", err.to_string()))?;
        Ok(format!("wrote:{}:{}", relative_path, content.len()))
    }
}

struct SearchTool {
    root: PathBuf,
}

impl ToolExecutor for SearchTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let mut matches = Vec::new();
        search_dir(&self.root, &self.root, &request.payload, &mut matches)?;
        Ok(matches.join("\n"))
    }
}

struct CommandExecTool {
    sandbox: SandboxManager,
    cwd: PathBuf,
}

impl ToolExecutor for CommandExecTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        if request.payload.trim().is_empty() {
            return Err(AcError::validation(
                "TOOL-COMMAND_EMPTY",
                "command payload cannot be empty",
            ));
        }
        let argv = request
            .payload
            .split_whitespace()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let plan = self.sandbox.prepare_execution(ExecRequest {
            argv,
            cwd: self.cwd.clone(),
            env: BTreeMap::new(),
            network: false,
            timeout_ms: 5_000,
        })?;
        let mut child = Command::new(&plan.argv[0])
            .args(&plan.argv[1..])
            .current_dir(&plan.cwd)
            .env_clear()
            .envs(&plan.allowed_env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| AcError::validation("TOOL-COMMAND_SPAWN_FAILED", err.to_string()))?;
        let deadline = Instant::now() + Duration::from_millis(plan.timeout_ms);
        loop {
            if child
                .try_wait()
                .map_err(|err| AcError::validation("TOOL-COMMAND_WAIT_FAILED", err.to_string()))?
                .is_some()
            {
                let output = child.wait_with_output().map_err(|err| {
                    AcError::validation("TOOL-COMMAND_OUTPUT_FAILED", err.to_string())
                })?;
                return Ok(format!(
                    "status:{}\nstdout:{}\nstderr:{}",
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                return Err(AcError::new(
                    "TOOL-COMMAND_TIMEOUT",
                    "command timed out and was killed",
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::Retryable,
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn safe_join(root: &Path, relative: &str) -> AcResult<PathBuf> {
    if relative.contains("..") || relative.starts_with('/') {
        return Err(AcError::policy_denied(
            "TOOL-PATH_ESCAPE",
            "tool path must stay inside workspace",
        ));
    }
    Ok(root.join(relative))
}

fn search_dir(
    root: &Path,
    current: &Path,
    needle: &str,
    matches: &mut Vec<String>,
) -> AcResult<()> {
    for entry in fs::read_dir(current)
        .map_err(|err| AcError::validation("TOOL-SEARCH_FAILED", err.to_string()))?
    {
        let entry =
            entry.map_err(|err| AcError::validation("TOOL-SEARCH_FAILED", err.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            search_dir(root, &path, needle, matches)?;
        } else if path.is_file() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            for (idx, line) in content.lines().enumerate() {
                if line.contains(needle) {
                    let relative = path.strip_prefix(root).unwrap_or(&path);
                    matches.push(format!("{}:{}:{}", relative.display(), idx + 1, line));
                }
            }
        }
    }
    Ok(())
}

pub struct ToolBroker {
    definitions: BTreeMap<String, ToolDefinition>,
    executors: BTreeMap<String, Box<dyn ToolExecutor>>,
    policy: CapabilityPolicy,
}

impl ToolBroker {
    pub fn new(policy: CapabilityPolicy) -> Self {
        Self {
            definitions: BTreeMap::new(),
            executors: BTreeMap::new(),
            policy,
        }
    }

    pub fn register_tool(
        &mut self,
        definition: ToolDefinition,
        executor: Box<dyn ToolExecutor>,
    ) -> AcResult<()> {
        if definition.id.trim().is_empty() || definition.version.trim().is_empty() {
            return Err(AcError::validation(
                "TOOL-INVALID_DEFINITION",
                "tool id and version are required",
            ));
        }
        self.executors.insert(definition.id.clone(), executor);
        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn invoke(
        &self,
        request: ToolRequest,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ToolResult> {
        let definition = self
            .definitions
            .get(&request.tool_id)
            .ok_or_else(|| AcError::validation("TOOL-UNKNOWN_TOOL", "tool is not registered"))?;
        if definition.version != request.tool_version {
            return Err(AcError::validation(
                "TOOL-VERSION_MISMATCH",
                "tool request version does not match registry",
            ));
        }
        let mut requested = definition.required_capabilities.clone();
        requested.extend(request.capabilities.clone());
        if self.policy.evaluate(&requested) != SecurityDecision::Allow {
            let evidence_ref = evidence_store.append(
                EvidenceKind::CommandOutput,
                Provenance {
                    source: "tool-broker".to_string(),
                    commit: None,
                    worktree: None,
                    tool: Some(request.tool_id.clone()),
                },
                format!("mem://tool/{}/denied", request.id),
                "denied",
            )?;
            return Ok(ToolResult {
                request_id: request.id,
                status: ToolStatus::Denied,
                observation: "policy denied tool invocation".to_string(),
                evidence_ref,
                finished_at: TimestampMillis::now(),
            });
        }
        let executor = self.executors.get(&request.tool_id).ok_or_else(|| {
            AcError::validation("TOOL-MISSING_EXECUTOR", "tool executor not found")
        })?;
        let observation = match executor.execute(&request) {
            Ok(observation) => observation,
            Err(error) => {
                let evidence_ref = evidence_store.append(
                    EvidenceKind::CommandOutput,
                    Provenance {
                        source: "tool-broker".to_string(),
                        commit: None,
                        worktree: None,
                        tool: Some(request.tool_id.clone()),
                    },
                    format!("mem://tool/{}/failed", request.id),
                    error.code(),
                )?;
                return Ok(ToolResult {
                    request_id: request.id,
                    status: ToolStatus::Failed,
                    observation: error.to_string(),
                    evidence_ref,
                    finished_at: TimestampMillis::now(),
                });
            }
        };
        let evidence_ref = evidence_store.append(
            EvidenceKind::CommandOutput,
            Provenance {
                source: "tool-broker".to_string(),
                commit: None,
                worktree: None,
                tool: Some(request.tool_id.clone()),
            },
            format!("mem://tool/{}/output", request.id),
            format!("len:{}", observation.len()),
        )?;
        Ok(ToolResult {
            request_id: request.id,
            status: ToolStatus::Succeeded,
            observation,
            evidence_ref,
            finished_at: TimestampMillis::now(),
        })
    }
}

pub struct CommandPlanner<'a> {
    sandbox: &'a SandboxManager,
}

impl<'a> CommandPlanner<'a> {
    pub fn new(sandbox: &'a SandboxManager) -> Self {
        Self { sandbox }
    }

    pub fn plan(&self, request: ExecRequest) -> AcResult<StableId> {
        Ok(self.sandbox.prepare_execution(request)?.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoExecutor;

    impl ToolExecutor for EchoExecutor {
        fn execute(&self, request: &ToolRequest) -> AcResult<String> {
            Ok(request.payload.clone())
        }
    }

    #[test]
    fn broker_denies_before_executor_runs_without_policy() {
        let mut broker = ToolBroker::new(CapabilityPolicy::new());
        broker
            .register_tool(
                ToolDefinition {
                    id: "echo".to_string(),
                    version: "1".to_string(),
                    required_capabilities: vec![Capability::ProcessExec("echo".to_string())],
                },
                Box::new(EchoExecutor),
            )
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "echo".to_string(),
                    tool_version: "1".to_string(),
                    payload: "hello".to_string(),
                    capabilities: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(result.status, ToolStatus::Denied);
        assert_eq!(evidence.len(), 1);
    }

    #[test]
    fn workspace_command_executes_through_sandbox_plan() {
        let root = std::env::temp_dir().join(format!("agentcode-tool-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::new(root).register_all(&mut broker).unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: "echo ok".to_string(),
                    capabilities: vec![Capability::ProcessExec("echo".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(result.status, ToolStatus::Succeeded);
        assert!(result.observation.contains("ok"));
    }
}
