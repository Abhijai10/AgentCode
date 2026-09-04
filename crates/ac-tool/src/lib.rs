use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::{EvidenceStore, Provenance};
use ac_sandbox::{ExecRequest, SandboxEvidence, SandboxManager, SandboxPolicy, SecretBroker};
use ac_security::{
    BaselineSecurityOrchestrator, ManagedSecurityScanInput, ScannerConfiguration, ScannerFailure,
    ScannerNetworkPolicy, ScannerProcessRequest, ScannerProcessResult, SecurityAdapter,
    SecurityPolicy, SecurityScannerExecutor,
};
use ac_security::{Capability, CapabilityPolicy, McpToolRecord, SecurityDecision};
use ac_verification::{
    BrowserAction, BrowserRuntime, VerificationEngine, VerificationLayer, VerificationProfile,
    VerificationRisk, ViewportProfile,
};
use serde_json::{json, Value};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    pub id: String,
    pub version: String,
    pub required_capabilities: Vec<Capability>,
    pub risk: ac_security::RiskClass,
    pub mutates_workspace: bool,
    pub network_required: bool,
    pub reversible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionManifest {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub timeout_ms: u64,
    pub network: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolEvent {
    Requested,
    Denied { reason: String },
    Started { manifest: ExecutionManifest },
    Finished { status: ToolStatus },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessState {
    Running,
    Finished,
    Cancelled,
    TimedOut,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRecord {
    pub id: StableId,
    pub task: String,
    pub manifest: ExecutionManifest,
    pub pid: u32,
    pub state: ProcessState,
    pub started_at: TimestampMillis,
    pub sandbox: SandboxEvidence,
    /// Temporary resources removed when the record reaches a terminal state.
    pub cleanup_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeProcessResult {
    pub record: ProcessRecord,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

pub struct SecretProcessRequest<'a> {
    pub request: ToolRequest,
    pub sandbox: &'a SandboxManager,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub timeout_ms: u64,
    pub secret_references: &'a BTreeMap<String, String>,
}

/// Maximum number of finished process records retained per ProcessManager
/// instance.  Finished records are observability-only; capping prevents
/// unbounded growth within a single long-running mission.  The currently
/// running record is always kept.
const MAX_PROCESS_RECORDS: usize = 1024;

#[derive(Clone, Default)]
pub struct ProcessManager {
    children: Arc<Mutex<BTreeMap<StableId, std::process::Child>>>,
    records: Arc<Mutex<BTreeMap<StableId, ProcessRecord>>>,
}

/// Bridge used by security adapters. It deliberately shares the ToolBroker's
/// sandboxed process implementation instead of allowing ac-security to spawn.
pub struct GovernedScannerExecutor {
    sandbox: SandboxManager,
    manager: ProcessManager,
}

impl GovernedScannerExecutor {
    pub fn new(sandbox: SandboxManager) -> Self {
        Self {
            sandbox,
            manager: ProcessManager::default(),
        }
    }
}

impl SecurityScannerExecutor for GovernedScannerExecutor {
    fn execute(
        &self,
        request: ScannerProcessRequest,
    ) -> Result<ScannerProcessResult, ScannerFailure> {
        let cleanup_paths = request.cleanup_paths.clone();
        let report_path = request.report_path.clone();
        let mut env = toolchain_env();
        env.extend(request.env.clone());
        let plan = self
            .sandbox
            .prepare_execution(ExecRequest {
                argv: request.argv,
                cwd: request.cwd,
                env,
                network: request.network,
                timeout_ms: request.timeout_ms,
            })
            .map_err(|error| match error.code() {
                "SANDBOX-NETWORK_DENIED" => ScannerFailure::NetworkDenied,
                "SANDBOX-INVALID_COMMAND" | "SANDBOX-CWD_OUTSIDE_WORKSPACE" => {
                    ScannerFailure::Misconfigured(error.to_string())
                }
                _ => ScannerFailure::Misconfigured(error.to_string()),
            })?;
        let result = self
            .manager
            .run("security-scanner", plan)
            .map(|result| {
                let stdout = report_path
                    .as_ref()
                    .and_then(|path| fs::read_to_string(path).ok())
                    .unwrap_or(result.stdout);
                ScannerProcessResult {
                    exit_code: result.exit_code,
                    stdout,
                    stderr: result.stderr,
                    stdout_truncated: result.stdout_truncated,
                    stderr_truncated: result.stderr_truncated,
                }
            })
            .map_err(|error| match error.code() {
                "TOOL-COMMAND_TIMEOUT" => ScannerFailure::Timeout,
                "TOOL-COMMAND_CANCELLED" => ScannerFailure::Cancelled,
                "TOOL-COMMAND_SPAWN_FAILED" => ScannerFailure::Unavailable(error.to_string()),
                _ => ScannerFailure::ExecutionFailed(error.to_string()),
            });
        for path in cleanup_paths {
            let _ = fs::remove_dir_all(path);
        }
        result
    }
}

impl ProcessManager {
    /// Evict oldest finished records beyond the in-memory cap.  Running
    /// records are never evicted because they describe live work.
    fn prune_records(records: &mut BTreeMap<StableId, ProcessRecord>) {
        if records.len() <= MAX_PROCESS_RECORDS {
            return;
        }
        let mut finished = records
            .iter()
            .filter(|(_, record)| record.state != ProcessState::Running)
            .map(|(id, record)| (record.started_at.as_millis(), id.clone()))
            .collect::<Vec<_>>();
        finished.sort_unstable();
        while records.len() > MAX_PROCESS_RECORDS {
            let Some((_, id)) = finished.first().cloned() else {
                break;
            };
            finished.remove(0);
            records.remove(&id);
        }
    }

    pub fn run(
        &self,
        task: impl Into<String>,
        plan: ac_sandbox::SandboxedExecutionPlan,
    ) -> AcResult<NativeProcessResult> {
        self.run_with_cancellation(task, plan, &AtomicBool::new(false))
    }

    pub fn run_with_cancellation(
        &self,
        task: impl Into<String>,
        plan: ac_sandbox::SandboxedExecutionPlan,
        cancelled: &AtomicBool,
    ) -> AcResult<NativeProcessResult> {
        let manifest = ExecutionManifest {
            argv: plan.argv.clone(),
            cwd: plan.cwd.clone(),
            timeout_ms: plan.timeout_ms,
            network: matches!(
                plan.sandbox_evidence.network_policy,
                ac_sandbox::NetworkPolicy::AllowAll
            ),
        };
        let mut command = Command::new(&plan.backend_argv[0]);
        command
            .args(&plan.backend_argv[1..])
            .current_dir(&plan.cwd)
            .env_clear()
            .envs(&plan.allowed_env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_process_group(&mut command);
        let mut child = command
            .spawn()
            .map_err(|error| AcError::validation("TOOL-COMMAND_SPAWN_FAILED", error.to_string()))?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let max_output_bytes = plan.max_output_bytes;
        let stdout_reader = stdout.map(|pipe| read_limited_in_thread(pipe, max_output_bytes));
        let stderr_reader = stderr.map(|pipe| read_limited_in_thread(pipe, max_output_bytes));
        let id = plan.id;
        let record = ProcessRecord {
            id: id.clone(),
            task: task.into(),
            manifest,
            pid: child.id(),
            state: ProcessState::Running,
            started_at: TimestampMillis::now(),
            sandbox: plan.sandbox_evidence.clone(),
            cleanup_paths: plan.cleanup_paths.clone(),
        };
        self.records
            .lock()
            .expect("process records lock")
            .insert(id.clone(), record.clone());
        Self::prune_records(&mut self.records.lock().expect("process records lock"));
        let deadline = Instant::now() + Duration::from_millis(plan.timeout_ms);
        let cleanup_paths = plan.cleanup_paths.clone();
        loop {
            match child.try_wait().map_err(|error| {
                AcError::validation("TOOL-COMMAND_WAIT_FAILED", error.to_string())
            })? {
                Some(status) => {
                    let _ = child.wait();
                    let (stdout, stdout_truncated) = join_output(stdout_reader)?;
                    let (stderr, stderr_truncated) = join_output(stderr_reader)?;
                    let mut finished = record.clone();
                    finished.state = if status.success() {
                        ProcessState::Finished
                    } else {
                        ProcessState::Failed
                    };
                    self.records
                        .lock()
                        .expect("process records lock")
                        .insert(id, finished.clone());
                    Self::prune_records(&mut self.records.lock().expect("process records lock"));
                    cleanup_plan_paths(&cleanup_paths);
                    return Ok(NativeProcessResult {
                        record: finished,
                        exit_code: status.code(),
                        stdout,
                        stderr,
                        stdout_truncated,
                        stderr_truncated,
                    });
                }
                None if cancelled.load(Ordering::Relaxed) => {
                    terminate_process_tree(child.id());
                    let _ = child.wait();
                    let mut cancelled_record = record.clone();
                    cancelled_record.state = ProcessState::Cancelled;
                    self.records
                        .lock()
                        .expect("process records lock")
                        .insert(id, cancelled_record);
                    Self::prune_records(&mut self.records.lock().expect("process records lock"));
                    cleanup_plan_paths(&cleanup_paths);
                    return Err(AcError::new(
                        "TOOL-COMMAND_CANCELLED",
                        "command was cancelled and was killed",
                        ac_common::ErrorKind::Unavailable,
                        ac_common::Retryability::NotRetryable,
                    ));
                }
                None if Instant::now() >= deadline => {
                    terminate_process_tree(child.id());
                    let _ = child.wait();
                    let mut timed_out = record.clone();
                    timed_out.state = ProcessState::TimedOut;
                    self.records
                        .lock()
                        .expect("process records lock")
                        .insert(id, timed_out);
                    Self::prune_records(&mut self.records.lock().expect("process records lock"));
                    cleanup_plan_paths(&cleanup_paths);
                    return Err(AcError::new(
                        "TOOL-COMMAND_TIMEOUT",
                        "command timed out and was killed",
                        ac_common::ErrorKind::Unavailable,
                        ac_common::Retryability::Retryable,
                    ));
                }
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    pub fn start_background(
        &self,
        task: impl Into<String>,
        plan: ac_sandbox::SandboxedExecutionPlan,
    ) -> AcResult<StableId> {
        let manifest = ExecutionManifest {
            argv: plan.argv.clone(),
            cwd: plan.cwd.clone(),
            timeout_ms: plan.timeout_ms,
            network: matches!(
                plan.sandbox_evidence.network_policy,
                ac_sandbox::NetworkPolicy::AllowAll
            ),
        };
        let mut command = Command::new(&plan.backend_argv[0]);
        command
            .args(&plan.backend_argv[1..])
            .current_dir(&plan.cwd)
            .env_clear()
            .envs(&plan.allowed_env)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        configure_process_group(&mut command);
        let child = command
            .spawn()
            .map_err(|error| AcError::validation("TOOL-COMMAND_SPAWN_FAILED", error.to_string()))?;
        let id = plan.id;
        let record = ProcessRecord {
            id: id.clone(),
            task: task.into(),
            manifest,
            pid: child.id(),
            state: ProcessState::Running,
            started_at: TimestampMillis::now(),
            sandbox: plan.sandbox_evidence,
            cleanup_paths: plan.cleanup_paths.clone(),
        };
        self.children
            .lock()
            .expect("process children lock")
            .insert(id.clone(), child);
        self.records
            .lock()
            .expect("process records lock")
            .insert(id.clone(), record);
        Self::prune_records(&mut self.records.lock().expect("process records lock"));
        Ok(id)
    }

    pub fn inspect(&self, id: &StableId) -> Option<ProcessRecord> {
        self.records
            .lock()
            .expect("process records lock")
            .get(id)
            .cloned()
    }

    pub fn cancel(&self, id: &StableId) -> AcResult<()> {
        let mut child = self
            .children
            .lock()
            .expect("process children lock")
            .remove(id)
            .ok_or_else(|| {
                AcError::validation(
                    "TOOL-PROCESS_UNKNOWN",
                    "background process is not registered",
                )
            })?;
        terminate_process_tree(child.id());
        let _ = child.wait();
        let mut records = self.records.lock().expect("process records lock");
        let record = records.get_mut(id).ok_or_else(|| {
            AcError::validation("TOOL-PROCESS_UNKNOWN", "process record is not registered")
        })?;
        let cleanup_paths = record.cleanup_paths.clone();
        record.state = ProcessState::Cancelled;
        drop(records);
        cleanup_plan_paths(&cleanup_paths);
        Ok(())
    }
}

fn cleanup_plan_paths(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn read_limited_in_thread<R: Read + Send + 'static>(
    mut reader: R,
    max_output_bytes: usize,
) -> std::thread::JoinHandle<AcResult<(String, bool)>> {
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 4096];
        let mut truncated = false;
        loop {
            let read = reader.read(&mut chunk).map_err(|error| {
                AcError::validation("TOOL-COMMAND_OUTPUT_FAILED", error.to_string())
            })?;
            if read == 0 {
                break;
            }
            let remaining = max_output_bytes.saturating_sub(buffer.len());
            if remaining == 0 {
                truncated = true;
                continue;
            }
            let keep = read.min(remaining);
            buffer.extend_from_slice(&chunk[..keep]);
            if keep < read {
                truncated = true;
            }
        }
        Ok((String::from_utf8_lossy(&buffer).to_string(), truncated))
    })
}

fn join_output(
    reader: Option<std::thread::JoinHandle<AcResult<(String, bool)>>>,
) -> AcResult<(String, bool)> {
    reader
        .map(|reader| {
            reader.join().unwrap_or_else(|_| {
                Err(AcError::validation(
                    "TOOL-COMMAND_OUTPUT_FAILED",
                    "output reader thread panicked",
                ))
            })
        })
        .unwrap_or_else(|| Ok((String::new(), false)))
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_process_tree(pid: u32) {
    let group = format!("-{}", pid);
    let _ = Command::new("/bin/kill")
        .args(["-TERM", &group])
        .env_clear()
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    std::thread::sleep(Duration::from_millis(25));
    let _ = Command::new("/bin/kill")
        .args(["-KILL", &group])
        .env_clear()
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(unix))]
fn terminate_process_tree(_pid: u32) {}

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

    fn execute_with_cancellation(
        &self,
        request: &ToolRequest,
        _cancelled: &AtomicBool,
    ) -> AcResult<String> {
        self.execute(request)
    }

    fn execute_with_evidence_and_cancellation(
        &self,
        request: &ToolRequest,
        _evidence_store: &mut EvidenceStore,
        cancelled: &AtomicBool,
    ) -> AcResult<String> {
        self.execute_with_cancellation(request, cancelled)
    }
}

#[derive(Clone, Debug)]
pub struct McpToolExecutor {
    server_id: StableId,
    tool_name: String,
}

impl ToolExecutor for McpToolExecutor {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        Ok(format!(
            "mcp_server:{}\ntool:{}\npayload:{}",
            self.server_id, self.tool_name, request.payload
        ))
    }
}

pub fn register_mcp_tool_with_broker(
    broker: &mut ToolBroker,
    tool: &McpToolRecord,
) -> AcResult<String> {
    let broker_tool_id = format!("mcp.{}.{}", tool.server_id, tool.name);
    broker.register_tool(
        ToolDefinition {
            id: broker_tool_id.clone(),
            version: "1".to_string(),
            required_capabilities: tool.required_capabilities.iter().cloned().collect(),
        },
        Box::new(McpToolExecutor {
            server_id: tool.server_id.clone(),
            tool_name: tool.name.clone(),
        }),
    )?;
    Ok(broker_tool_id)
}

#[derive(Clone, Debug)]
pub struct WorkspaceTools {
    root: PathBuf,
    required_isolation: ac_sandbox::IsolationLevel,
}

impl WorkspaceTools {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            required_isolation: ac_sandbox::IsolationLevel::FilesystemIsolated,
        }
    }

    pub fn required_isolation(&self) -> ac_sandbox::IsolationLevel {
        self.required_isolation
    }

    pub fn with_required_isolation(
        root: PathBuf,
        required_isolation: ac_sandbox::IsolationLevel,
    ) -> Self {
        Self {
            root,
            required_isolation,
        }
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
                    required_isolation: self.required_isolation,
                    ..SandboxPolicy::new(vec![self.root.clone()])
                }),
                cwd: self.root.clone(),
                manager: ProcessManager::default(),
            }),
        )?;
        broker.register_tool(
            ToolDefinition {
                id: "dev.test".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::ProcessExec("*".to_string())],
            },
            Box::new(ProjectTestTool {
                sandbox: workspace_sandbox(&self.root, self.required_isolation),
                cwd: self.root.clone(),
                manager: ProcessManager::default(),
            }),
        )?;
        broker.register_tool(
            ToolDefinition {
                id: "browser.verify".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::BrowserAutomation],
            },
            Box::new(BrowserVerifyTool),
        )?;
        broker.register_tool(
            ToolDefinition {
                id: "security.verify".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::SecurityScan],
            },
            Box::new(SecurityVerifyTool {
                root: self.root.clone(),
                required_isolation: self.required_isolation,
            }),
        )?;
        for (id, command) in [
            ("repo.status", vec!["git", "status", "--short"]),
            ("repo.diff", vec!["git", "diff", "--", "."]),
            ("repo.branch", vec!["git", "branch", "--show-current"]),
            ("dev.format", vec!["cargo", "fmt", "--all"]),
            ("dev.check", vec!["cargo", "check", "--quiet"]),
        ] {
            broker.register_tool(
                ToolDefinition {
                    id: id.to_string(),
                    version: "1".to_string(),
                    required_capabilities: vec![Capability::ProcessExec(command[0].to_string())],
                },
                Box::new(FixedCommandTool {
                    sandbox: workspace_sandbox(&self.root, self.required_isolation),
                    cwd: self.root.clone(),
                    argv: command.iter().map(ToString::to_string).collect(),
                    manager: ProcessManager::default(),
                }),
            )?;
        }
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
    manager: ProcessManager,
}

impl ToolExecutor for CommandExecTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let not_cancelled = AtomicBool::new(false);
        self.execute_with_cancellation(request, &not_cancelled)
    }

    fn execute_with_cancellation(
        &self,
        request: &ToolRequest,
        cancelled: &AtomicBool,
    ) -> AcResult<String> {
        if request.payload.trim().is_empty() {
            return Err(AcError::validation(
                "TOOL-COMMAND_EMPTY",
                "command payload cannot be empty",
            ));
        }
        run_sandboxed_command(
            &self.manager,
            &self.sandbox,
            self.cwd.clone(),
            parse_argv(&request.payload)?,
            5_000,
            cancelled,
        )
    }
}

struct FixedCommandTool {
    sandbox: SandboxManager,
    cwd: PathBuf,
    argv: Vec<String>,
    manager: ProcessManager,
}

impl ToolExecutor for FixedCommandTool {
    fn execute(&self, _request: &ToolRequest) -> AcResult<String> {
        let not_cancelled = AtomicBool::new(false);
        self.execute_with_cancellation(_request, &not_cancelled)
    }

    fn execute_with_cancellation(
        &self,
        _request: &ToolRequest,
        cancelled: &AtomicBool,
    ) -> AcResult<String> {
        run_sandboxed_command(
            &self.manager,
            &self.sandbox,
            self.cwd.clone(),
            self.argv.clone(),
            30_000,
            cancelled,
        )
    }
}

struct ProjectTestTool {
    sandbox: SandboxManager,
    cwd: PathBuf,
    manager: ProcessManager,
}

impl ToolExecutor for ProjectTestTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let not_cancelled = AtomicBool::new(false);
        self.execute_with_cancellation(request, &not_cancelled)
    }

    fn execute_with_cancellation(
        &self,
        _request: &ToolRequest,
        cancelled: &AtomicBool,
    ) -> AcResult<String> {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let profile = VerificationProfile {
            id: StableId::new("verifyprofile"),
            task_id: StableId::new("task"),
            risk: VerificationRisk::Medium,
            required_layers: vec![VerificationLayer::Unit],
            created_at: TimestampMillis::now(),
        };
        let command = engine
            .detect_commands(&self.cwd, &profile)
            .into_iter()
            .find(|command| command.layer == VerificationLayer::Unit)
            .ok_or_else(|| {
                AcError::new(
                    "TOOL-VERIFY_NO_KNOWN_TEST_COMMAND",
                    "NoKnownTestCommand",
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::NotRetryable,
                )
            })?;
        run_sandboxed_command(
            &self.manager,
            &self.sandbox,
            self.cwd.clone(),
            command.argv,
            30_000,
            cancelled,
        )
    }
}

struct BrowserVerifyTool;

impl ToolExecutor for BrowserVerifyTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let mut evidence_store = EvidenceStore::new();
        let not_cancelled = AtomicBool::new(false);
        self.execute_with_evidence_and_cancellation(request, &mut evidence_store, &not_cancelled)
    }

    fn execute_with_evidence_and_cancellation(
        &self,
        request: &ToolRequest,
        evidence_store: &mut EvidenceStore,
        _cancelled: &AtomicBool,
    ) -> AcResult<String> {
        let input = BrowserVerifyInput::parse(&request.payload)?;
        authorize_browser_target(&input.target_url)?;
        let task_id = request.id.clone();
        let mut runtime =
            BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
        let process = runtime.launch(task_id.clone())?;
        let session = runtime.create_session(task_id.clone(), process.id.clone())?;
        let navigation = runtime.act(
            &session.id,
            BrowserAction::Navigate {
                url: input.target_url.clone(),
            },
            evidence_store,
        )?;
        let dom = runtime.inspect_dom(&session.id, evidence_store)?;
        let diagnostics = runtime.diagnostics(&session.id, evidence_store)?;
        let screenshot = if input.screenshot {
            Some(runtime.capture_screenshot(
                &session.id,
                task_id,
                "working-tree",
                input.viewport,
                evidence_store,
            )?)
        } else {
            None
        };
        let assertions = evaluate_browser_assertions(&input.assertions, &dom, &diagnostics);
        let passed = assertions
            .iter()
            .all(|assertion| assertion["passed"] == true);
        let output = json!({
            "target": redact_url(&input.target_url),
            "authorized": true,
            "mode": "real-cdp",
            "profile_isolated": process.profile_dir.contains("agentcode-browser-profile"),
            "navigation": {
                "ok": navigation.ok,
                "status": diagnostics.http_status,
                "evidence_ref": navigation.evidence_ref.to_string()
            },
            "diagnostics": {
                "console_errors": diagnostics.console_errors,
                "page_errors": diagnostics.page_errors,
                "network_failures": diagnostics.network_failures,
                "evidence_ref": diagnostics.evidence_ref.to_string()
            },
            "dom": {
                "visible_text_hash": local_hash(&dom.visible_text),
                "controls": dom.controls,
                "accessibility": input.include_accessibility.then_some(dom.accessibility_tree),
                "evidence_ref": dom.evidence_ref.to_string()
            },
            "screenshot_evidence_ref": screenshot.map(|shot| shot.evidence_ref.to_string()),
            "assertions": assertions,
            "passed": passed
        });
        Ok(output.to_string())
    }
}

struct SecurityVerifyTool {
    root: PathBuf,
    required_isolation: ac_sandbox::IsolationLevel,
}

impl ToolExecutor for SecurityVerifyTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let mut evidence_store = EvidenceStore::new();
        let not_cancelled = AtomicBool::new(false);
        self.execute_with_evidence_and_cancellation(request, &mut evidence_store, &not_cancelled)
    }

    fn execute_with_evidence_and_cancellation(
        &self,
        request: &ToolRequest,
        evidence_store: &mut EvidenceStore,
        _cancelled: &AtomicBool,
    ) -> AcResult<String> {
        let input = SecurityVerifyInput::parse(&request.payload)?;
        let root = input
            .scope
            .as_deref()
            .map(|scope| safe_join(&self.root, scope))
            .transpose()?
            .unwrap_or_else(|| self.root.clone());
        let mut configurations = Vec::new();
        for scanner in input.required_scanners.iter().copied() {
            let mut config = scanner_configuration(scanner, true, &input)?;
            config.required = true;
            configurations.push(config);
        }
        for scanner in input.optional_scanners.iter().copied() {
            let mut config = scanner_configuration(scanner, false, &input)?;
            config.required = false;
            configurations.push(config);
        }
        if configurations.is_empty() {
            return Err(AcError::validation(
                "TOOL-SECURITY_VERIFY_EMPTY_PROFILE",
                "security.verify requires at least one scanner",
            ));
        }
        let mut workspace_roots = vec![root.clone()];
        for config in &configurations {
            if let Some(data_dir) = &config.data_dir {
                workspace_roots.push(data_dir.clone());
            }
        }
        let scan = ManagedSecurityScanInput {
            repository_id: StableId::new("repo"),
            commit: source_revision_from_git_files(&root),
            workspace_root: root.clone(),
            configurations,
            target_url: input.dast_target.clone(),
            target_authorized: input.dast_target_authorized,
        };
        let mut scanner_policy =
            CapabilityPolicy::new().allow(Capability::ProcessExec("*".to_string()));
        if input.dast_target.is_some() {
            scanner_policy = scanner_policy.allow(Capability::Network("*".to_string()));
        }
        let sandbox = SandboxManager::new(SandboxPolicy {
            workspace_roots,
            capability_policy: scanner_policy,
            network_default_allow: input.dast_target.is_some(),
            max_timeout_ms: 180_000,
            required_isolation: self.required_isolation,
            ..SandboxPolicy::new(vec![root.clone()])
        });
        let executor = GovernedScannerExecutor::new(sandbox);
        let orchestrator = BaselineSecurityOrchestrator::new(SecurityPolicy::baseline());
        let report = orchestrator.run_managed(&scan, &executor, evidence_store)?;
        let output = json!({
            "source_revision": scan.commit,
            "workspace": root.display().to_string(),
            "adapters_run": report.adapters_run.iter().map(|adapter| scanner_name(*adapter)).collect::<Vec<_>>(),
            "missing_adapters": report.missing_adapters,
            "finding_count": report.instances.len(),
            "findings": report.instances.iter().map(|finding| json!({
                "scanner": scanner_name(finding.adapter),
                "rule_id": finding.rule_id,
                "severity": format!("{:?}", finding.severity),
                "file_path": finding.file_path,
                "line": finding.line,
                "fingerprint": finding.fingerprint,
                "redacted_evidence": finding.redacted_evidence,
                "raw_evidence_ref": finding.raw_evidence_ref.to_string(),
                "proof_level": format!("{:?}", finding.proof_level)
            })).collect::<Vec<_>>(),
            "executions": report.executions.iter().map(|execution| json!({
                "scanner": scanner_name(execution.adapter),
                "version": execution.version,
                "status": format!("{:?}", execution.availability),
                "raw_evidence_ref": execution.raw_evidence_ref.as_ref().map(ToString::to_string),
                "provenance": execution.version.as_ref().map(|_| "ExternalTool")
            })).collect::<Vec<_>>()
        });
        Ok(output.to_string())
    }
}

fn run_sandboxed_command(
    manager: &ProcessManager,
    sandbox: &SandboxManager,
    cwd: PathBuf,
    argv: Vec<String>,
    timeout_ms: u64,
    cancelled: &AtomicBool,
) -> AcResult<String> {
    let plan = sandbox.prepare_execution(ExecRequest {
        argv,
        cwd,
        env: toolchain_env(),
        network: false,
        timeout_ms,
    })?;
    let result = manager.run_with_cancellation("tool-command", plan, cancelled)?;
    if !matches!(result.exit_code, Some(0)) {
        return Err(AcError::validation(
            "TOOL-COMMAND_EXIT_NONZERO",
            format_process_observation(&result),
        ));
    }
    Ok(format_process_observation(&result))
}

fn workspace_sandbox(
    root: &Path,
    required_isolation: ac_sandbox::IsolationLevel,
) -> SandboxManager {
    SandboxManager::new(SandboxPolicy {
        workspace_roots: vec![root.to_path_buf()],
        capability_policy: CapabilityPolicy::new().allow(Capability::ProcessExec("*".to_string())),
        network_default_allow: false,
        max_timeout_ms: 30_000,
        required_isolation,
        // Fixed-command worktree tools (dev.test/dev.check/dev.format/repo.*)
        // run a known argv inside the trusted mission worktree.  When the OS
        // backend cannot deliver filesystem isolation (sandbox-exec/bwrap
        // unavailable), they degrade to process-restricted execution with
        // honest evidence instead of failing every mission.  Arbitrary
        // commands (cmd.exec) never set this flag and fail closed.
        allow_degraded_execution: true,
        ..SandboxPolicy::new(vec![root.to_path_buf()])
    })
}

struct BrowserVerifyInput {
    target_url: String,
    assertions: Vec<BrowserAssertion>,
    viewport: ViewportProfile,
    include_accessibility: bool,
    screenshot: bool,
}

enum BrowserAssertion {
    VisibleTextContains(String),
    AccessibilityContains(String),
    NoConsoleErrors,
    NoPageErrors,
    NoNetworkFailures,
}

impl BrowserVerifyInput {
    fn parse(payload: &str) -> AcResult<Self> {
        let value = parse_json_payload(payload)?;
        let target_url = value
            .get("target_url")
            .or_else(|| value.get("url"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AcError::validation("TOOL-BROWSER_VERIFY_TARGET", "target_url is required")
            })?
            .to_string();
        let assertions = value
            .get("assertions")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(parse_browser_assertion)
                    .collect::<AcResult<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            target_url,
            assertions,
            viewport: parse_viewport(value.get("viewport")),
            include_accessibility: value
                .get("include_accessibility")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            screenshot: value
                .get("screenshot")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

fn parse_browser_assertion(value: &Value) -> AcResult<BrowserAssertion> {
    let kind = value
        .get("kind")
        .or_else(|| value.get("type"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AcError::validation("TOOL-BROWSER_ASSERTION", "assertion kind is required")
        })?;
    match kind {
        "visible_text_contains" => Ok(BrowserAssertion::VisibleTextContains(
            value
                .get("text")
                .or_else(|| value.get("value"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AcError::validation("TOOL-BROWSER_ASSERTION", "assertion text is required")
                })?
                .to_string(),
        )),
        "accessibility_contains" => Ok(BrowserAssertion::AccessibilityContains(
            value
                .get("text")
                .or_else(|| value.get("value"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AcError::validation("TOOL-BROWSER_ASSERTION", "assertion text is required")
                })?
                .to_string(),
        )),
        "no_console_errors" => Ok(BrowserAssertion::NoConsoleErrors),
        "no_page_errors" => Ok(BrowserAssertion::NoPageErrors),
        "no_network_failures" => Ok(BrowserAssertion::NoNetworkFailures),
        _ => Err(AcError::validation(
            "TOOL-BROWSER_ASSERTION_UNSUPPORTED",
            "unsupported browser assertion kind",
        )),
    }
}

fn parse_viewport(value: Option<&Value>) -> ViewportProfile {
    match value
        .and_then(Value::as_str)
        .unwrap_or("desktop")
        .to_ascii_lowercase()
        .as_str()
    {
        "mobile" => ViewportProfile {
            name: "mobile",
            width: 390,
            height: 844,
        },
        "tablet" => ViewportProfile {
            name: "tablet",
            width: 820,
            height: 1180,
        },
        _ => ViewportProfile {
            name: "desktop",
            width: 1440,
            height: 900,
        },
    }
}

fn evaluate_browser_assertions(
    assertions: &[BrowserAssertion],
    dom: &ac_verification::DomSnapshot,
    diagnostics: &ac_verification::BrowserDiagnostics,
) -> Vec<Value> {
    assertions
        .iter()
        .map(|assertion| match assertion {
            BrowserAssertion::VisibleTextContains(text) => json!({
                "kind": "visible_text_contains",
                "expected": text,
                "passed": dom.visible_text.contains(text)
            }),
            BrowserAssertion::AccessibilityContains(text) => json!({
                "kind": "accessibility_contains",
                "expected": text,
                "passed": dom.accessibility_tree.iter().any(|item| item.contains(text))
            }),
            BrowserAssertion::NoConsoleErrors => json!({
                "kind": "no_console_errors",
                "passed": diagnostics.console_errors.is_empty()
            }),
            BrowserAssertion::NoPageErrors => json!({
                "kind": "no_page_errors",
                "passed": diagnostics.page_errors.is_empty()
            }),
            BrowserAssertion::NoNetworkFailures => json!({
                "kind": "no_network_failures",
                "passed": diagnostics.network_failures.is_empty()
            }),
        })
        .collect()
}

fn authorize_browser_target(url: &str) -> AcResult<()> {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("file:")
        || lower.starts_with("chrome:")
        || lower.starts_with("chrome-extension:")
        || lower.starts_with("about:")
        || lower.starts_with("devtools:")
    {
        return Err(AcError::policy_denied(
            "TOOL-BROWSER_TARGET_UNAUTHORIZED",
            "browser.verify may not browse local files or browser-internal URLs",
        ));
    }
    let local_http = lower.starts_with("http://localhost")
        || lower.starts_with("https://localhost")
        || lower.starts_with("http://127.0.0.1")
        || lower.starts_with("https://127.0.0.1")
        || lower.starts_with("http://[::1]")
        || lower.starts_with("https://[::1]");
    let metadata = lower.contains("169.254.169.254") || lower.contains("[fe80:");
    if local_http && !metadata {
        return Ok(());
    }
    Err(AcError::policy_denied(
        "TOOL-BROWSER_TARGET_UNAUTHORIZED",
        "browser.verify target must be localhost unless explicitly authorized by mission policy",
    ))
}

struct SecurityVerifyInput {
    scope: Option<String>,
    required_scanners: Vec<SecurityAdapter>,
    optional_scanners: Vec<SecurityAdapter>,
    semgrep_rules_path: Option<String>,
    scanner_data_dir: Option<PathBuf>,
    dast_target: Option<String>,
    dast_target_authorized: bool,
}

impl SecurityVerifyInput {
    fn parse(payload: &str) -> AcResult<Self> {
        let value = parse_json_payload(payload)?;
        let required_scanners = parse_scanner_array(value.get("required_scanners"))?;
        let optional_scanners = parse_scanner_array(value.get("optional_scanners"))?;
        Ok(Self {
            scope: value
                .get("scope")
                .or_else(|| value.get("repository_scope"))
                .and_then(Value::as_str)
                .map(ToString::to_string),
            required_scanners,
            optional_scanners,
            semgrep_rules_path: value
                .get("semgrep_rules_path")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            scanner_data_dir: value
                .get("scanner_data_dir")
                .and_then(Value::as_str)
                .map(PathBuf::from),
            dast_target: value
                .get("dast_target")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            dast_target_authorized: value
                .get("dast_target_authorized")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

fn parse_json_payload(payload: &str) -> AcResult<Value> {
    if payload.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(payload)
        .map_err(|error| AcError::validation("TOOL-PAYLOAD_JSON", error.to_string()))
}

fn parse_scanner_array(value: Option<&Value>) -> AcResult<Vec<SecurityAdapter>> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .ok_or_else(|| {
                            AcError::validation(
                                "TOOL-SECURITY_SCANNER_NAME",
                                "scanner names must be strings",
                            )
                        })
                        .and_then(scanner_from_name)
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn scanner_from_name(name: &str) -> AcResult<SecurityAdapter> {
    match name.to_ascii_lowercase().as_str() {
        "gitleaks" => Ok(SecurityAdapter::Gitleaks),
        "osv" | "osv-scanner" => Ok(SecurityAdapter::Osv),
        "trivy" => Ok(SecurityAdapter::Trivy),
        "semgrep" => Ok(SecurityAdapter::Semgrep),
        "checkov" => Ok(SecurityAdapter::Checkov),
        "zap" | "zaproxy" => Ok(SecurityAdapter::Zap),
        _ => Err(AcError::validation(
            "TOOL-SECURITY_SCANNER_UNSUPPORTED",
            "unsupported scanner name",
        )),
    }
}

fn scanner_configuration(
    scanner: SecurityAdapter,
    required: bool,
    input: &SecurityVerifyInput,
) -> AcResult<ScannerConfiguration> {
    let mut config = ScannerConfiguration::external(scanner);
    config.required = required;
    if matches!(scanner, SecurityAdapter::Osv | SecurityAdapter::Trivy) {
        config.timeout_ms = 180_000;
        config.data_dir = Some(trusted_scanner_data_root(input)?.join(scanner_name(scanner)));
    }
    if scanner == SecurityAdapter::Semgrep {
        config.rules_path = input.semgrep_rules_path.clone();
    }
    if scanner == SecurityAdapter::Zap {
        let target = input.dast_target.as_deref().ok_or_else(|| {
            AcError::validation("TOOL-SECURITY_ZAP_TARGET", "ZAP requires dast_target")
        })?;
        authorize_browser_target(target)?;
        config.network = ScannerNetworkPolicy::Allow;
        config.timeout_ms = config.timeout_ms.max(120_000);
    }
    Ok(config)
}

fn scanner_name(scanner: SecurityAdapter) -> &'static str {
    match scanner {
        SecurityAdapter::Gitleaks => "gitleaks",
        SecurityAdapter::Osv => "osv-scanner",
        SecurityAdapter::Trivy => "trivy",
        SecurityAdapter::Semgrep => "semgrep",
        SecurityAdapter::Checkov => "checkov",
        SecurityAdapter::Zap => "zap",
        _ => "agentcode",
    }
}

fn default_scanner_data_root() -> PathBuf {
    std::env::var_os("AGENTCODE_SCANNER_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("AGENTCODE_RUNTIME_DIR")
                .map(|dir| PathBuf::from(dir).join("scanner-data"))
        })
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                PathBuf::from(home)
                    .join("Library/Application Support/AgentCode/runtime/scanner-data")
            })
        })
        .unwrap_or_else(|| std::env::temp_dir().join("agentcode-scanner-data"))
}

fn trusted_scanner_data_root(input: &SecurityVerifyInput) -> AcResult<PathBuf> {
    let default = default_scanner_data_root();
    let Some(explicit) = &input.scanner_data_dir else {
        return Ok(default);
    };
    let canonical_default = fs::canonicalize(&default).map_err(|error| {
        AcError::validation(
            "TOOL-SCANNER_DATA_ROOT_UNAVAILABLE",
            format!("managed scanner-data root is unavailable: {error}"),
        )
    })?;
    let canonical_explicit = fs::canonicalize(explicit).map_err(|error| {
        AcError::validation(
            "TOOL-SCANNER_DATA_ROOT_UNAVAILABLE",
            format!("scanner_data_dir is unavailable: {error}"),
        )
    })?;
    if canonical_explicit != canonical_default
        && !canonical_explicit.starts_with(&canonical_default)
    {
        return Err(AcError::policy_denied(
            "TOOL-SCANNER_DATA_ROOT_DENIED",
            "scanner_data_dir must be inside AgentCode-managed scanner-data",
        ));
    }
    Ok(canonical_explicit)
}

fn source_revision_from_git_files(root: &Path) -> String {
    let git = root.join(".git");
    let git_dir = if git.is_dir() {
        git
    } else {
        fs::read_to_string(&git)
            .ok()
            .and_then(|content| {
                content
                    .strip_prefix("gitdir:")
                    .map(str::trim)
                    .map(PathBuf::from)
            })
            .unwrap_or(git)
    };
    let head = fs::read_to_string(git_dir.join("HEAD")).unwrap_or_default();
    let head = head.trim();
    if let Some(reference) = head.strip_prefix("ref: ") {
        return fs::read_to_string(git_dir.join(reference))
            .unwrap_or_else(|_| "working-tree".to_string())
            .trim()
            .to_string();
    }
    if head.is_empty() {
        "working-tree".to_string()
    } else {
        head.to_string()
    }
}

fn redact_url(url: &str) -> String {
    let without_fragment = url.split('#').next().unwrap_or(url);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    without_query.to_string()
}

fn local_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn format_process_observation(result: &NativeProcessResult) -> String {
    format!(
        "sandbox_backend:{}\nrequested_isolation:{:?}\nachieved_isolation:{:?}\nnetwork_policy:{:?}\nworkspace_roots:{}\ntimeout_ms:{}\nmax_output_bytes:{}\nstdout_truncated:{}\nstderr_truncated:{}\nstatus:{}\nstdout:{}\nstderr:{}",
        result.record.sandbox.backend_name,
        result.record.sandbox.requested_isolation,
        result.record.sandbox.achieved_isolation,
        result.record.sandbox.network_policy,
        result
            .record
            .sandbox
            .workspace_roots
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(","),
        result.record.sandbox.timeout_ms,
        result.record.sandbox.max_output_bytes,
        result.stdout_truncated,
        result.stderr_truncated,
        result.exit_code.unwrap_or(-1),
        result.stdout,
        result.stderr
    )
}

fn parse_argv(payload: &str) -> AcResult<Vec<String>> {
    let argv = payload
        .lines()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if argv.is_empty() {
        return Err(AcError::validation(
            "TOOL-COMMAND_EMPTY",
            "command argv requires one argument per line",
        ));
    }
    Ok(argv)
}

fn toolchain_env() -> BTreeMap<String, String> {
    let mut env = ["PATH", "CARGO_HOME", "RUSTUP_HOME", "RUSTC_WRAPPER"]
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| ((*key).to_string(), value))
        })
        .collect::<BTreeMap<_, _>>();
    if let Some(toolchain_bin) = rustup_toolchain_bin() {
        let path = env.get("PATH").cloned().unwrap_or_default();
        env.insert(
            "PATH".to_string(),
            format!("{}:{path}", toolchain_bin.display()),
        );
    }
    let home = std::env::temp_dir().join(format!("agentcode-home-{}", StableId::new("tool")));
    let _ = fs::create_dir_all(&home);
    env.insert("HOME".to_string(), home.display().to_string());
    env.insert("SEMGREP_SEND_METRICS".to_string(), "off".to_string());
    env.insert("SEMGREP_ENABLE_VERSION_CHECK".to_string(), "0".to_string());
    env.insert("OTEL_SDK_DISABLED".to_string(), "true".to_string());
    env
}

fn rustup_toolchain_bin() -> Option<PathBuf> {
    let output = Command::new("rustup")
        .args(["which", "cargo"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let cargo = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let cargo = PathBuf::from(cargo);
    cargo
        .parent()
        .map(Path::to_path_buf)
        .filter(|path| path.is_dir())
}

fn safe_join(root: &Path, relative: &str) -> AcResult<PathBuf> {
    let root = fs::canonicalize(root)
        .map_err(|error| AcError::validation("TOOL-PATH_RESOLUTION", error.to_string()))?;
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(AcError::policy_denied(
            "TOOL-PATH_ESCAPE",
            "tool path must stay inside workspace",
        ));
    }
    let candidate = root.join(relative);
    let resolved = if candidate.exists() {
        fs::canonicalize(&candidate)
            .map_err(|error| AcError::validation("TOOL-PATH_RESOLUTION", error.to_string()))?
    } else {
        let parent = candidate
            .parent()
            .ok_or_else(|| AcError::policy_denied("TOOL-PATH_ESCAPE", "tool path has no parent"))?;
        let parent = fs::canonicalize(parent)
            .map_err(|error| AcError::validation("TOOL-PATH_RESOLUTION", error.to_string()))?;
        parent.join(candidate.file_name().ok_or_else(|| {
            AcError::validation("TOOL-PATH_RESOLUTION", "tool path has no file name")
        })?)
    };
    if !resolved.starts_with(&root) {
        return Err(AcError::policy_denied(
            "TOOL-PATH_SYMLINK_ESCAPE",
            "resolved path escapes the workspace",
        ));
    }
    Ok(resolved)
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
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| AcError::validation("TOOL-SEARCH_FAILED", error.to_string()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
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
        if self.definitions.contains_key(&definition.id) {
            return Err(AcError::conflict(
                "TOOL-DUPLICATE_REGISTRATION",
                "tool id is already registered",
            ));
        }
        self.executors.insert(definition.id.clone(), executor);
        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn definition(&self, id: &str) -> Option<&ToolDefinition> {
        self.definitions.get(id)
    }

    pub fn invoke(
        &self,
        request: ToolRequest,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ToolResult> {
        let not_cancelled = AtomicBool::new(false);
        self.invoke_with_cancellation(request, evidence_store, &not_cancelled)
    }

    pub fn invoke_with_cancellation(
        &self,
        request: ToolRequest,
        evidence_store: &mut EvidenceStore,
        cancelled: &AtomicBool,
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
            let evidence_ref = evidence_store.append_tool_output(
                Provenance {
                    source: "tool-broker".to_string(),
                    commit: None,
                    worktree: None,
                    tool: Some(request.tool_id.clone()),
                },
                format!("mem://tool/{}/denied", request.id),
                "denied",
                &[],
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
        let observation = match executor.execute_with_evidence_and_cancellation(
            &request,
            evidence_store,
            cancelled,
        ) {
            Ok(observation) => observation,
            Err(error) => {
                let evidence_ref = evidence_store.append_tool_output(
                    Provenance {
                        source: "tool-broker".to_string(),
                        commit: None,
                        worktree: None,
                        tool: Some(request.tool_id.clone()),
                    },
                    format!("mem://tool/{}/failed", request.id),
                    error.to_string(),
                    &[],
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
        let evidence_ref = evidence_store.append_tool_output(
            Provenance {
                source: "tool-broker".to_string(),
                commit: None,
                worktree: None,
                tool: Some(request.tool_id.clone()),
            },
            format!("mem://tool/{}/output", request.id),
            observation.clone(),
            &[],
        )?;
        Ok(ToolResult {
            request_id: request.id,
            status: ToolStatus::Succeeded,
            observation,
            evidence_ref,
            finished_at: TimestampMillis::now(),
        })
    }

    pub fn invoke_process_with_secrets(
        &self,
        manager: &ProcessManager,
        secret_request: SecretProcessRequest<'_>,
        secrets: &SecretBroker,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ToolResult> {
        let SecretProcessRequest {
            request,
            sandbox,
            argv,
            cwd,
            timeout_ms,
            secret_references,
        } = secret_request;
        let definition = self
            .definitions
            .get(&request.tool_id)
            .ok_or_else(|| AcError::validation("TOOL-UNKNOWN_TOOL", "tool is not registered"))?;
        let mut requested = definition.required_capabilities.clone();
        requested.extend(request.capabilities.clone());
        requested.extend(
            secret_references
                .values()
                .cloned()
                .map(Capability::SecretRead),
        );
        if self.policy.evaluate(&requested) != SecurityDecision::Allow {
            return self.denied_result(request, evidence_store, "policy denied tool invocation");
        }
        let (plan, secret_values) = match sandbox.prepare_execution_with_secrets(
            ExecRequest {
                argv,
                cwd,
                env: toolchain_env(),
                network: false,
                timeout_ms,
            },
            secret_references,
            secrets,
        ) {
            Ok(value) => value,
            Err(error) => {
                return self.failed_result(request, evidence_store, error.to_string(), &[])
            }
        };
        match manager.run("secret-command", plan) {
            Ok(output) => self.completed_result(
                request,
                evidence_store,
                format_process_observation(&output),
                &secret_values,
            ),
            Err(error) => {
                self.failed_result(request, evidence_store, error.to_string(), &secret_values)
            }
        }
    }

    fn denied_result(
        &self,
        request: ToolRequest,
        evidence_store: &mut EvidenceStore,
        message: &str,
    ) -> AcResult<ToolResult> {
        let evidence_ref = evidence_store.append_tool_output(
            EvidenceStore::new_provenance(&request.tool_id),
            format!("mem://tool/{}/denied", request.id),
            message,
            &[],
        )?;
        Ok(ToolResult {
            request_id: request.id,
            status: ToolStatus::Denied,
            observation: message.to_string(),
            evidence_ref,
            finished_at: TimestampMillis::now(),
        })
    }

    fn failed_result(
        &self,
        request: ToolRequest,
        evidence_store: &mut EvidenceStore,
        message: String,
        secrets: &[String],
    ) -> AcResult<ToolResult> {
        let evidence_ref = evidence_store.append_tool_output(
            EvidenceStore::new_provenance(&request.tool_id),
            format!("mem://tool/{}/failed", request.id),
            message.clone(),
            secrets,
        )?;
        Ok(ToolResult {
            request_id: request.id,
            status: ToolStatus::Failed,
            observation: redact_for_agent(&message, secrets),
            evidence_ref,
            finished_at: TimestampMillis::now(),
        })
    }

    fn completed_result(
        &self,
        request: ToolRequest,
        evidence_store: &mut EvidenceStore,
        observation: String,
        secrets: &[String],
    ) -> AcResult<ToolResult> {
        let evidence_ref = evidence_store.append_tool_output(
            EvidenceStore::new_provenance(&request.tool_id),
            format!("mem://tool/{}/output", request.id),
            observation.clone(),
            secrets,
        )?;
        Ok(ToolResult {
            request_id: request.id,
            status: ToolStatus::Succeeded,
            observation: redact_for_agent(&observation, secrets),
            evidence_ref,
            finished_at: TimestampMillis::now(),
        })
    }
}

trait ToolProvenance {
    fn new_provenance(tool: &str) -> Provenance;
}

impl ToolProvenance for EvidenceStore {
    fn new_provenance(tool: &str) -> Provenance {
        Provenance {
            source: "tool-broker".to_string(),
            commit: None,
            worktree: None,
            tool: Some(tool.to_string()),
        }
    }
}

fn redact_for_agent(value: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(value.to_string(), |value, secret| {
            value.replace(secret, "[REDACTED]")
        })
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
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::thread;

    struct EchoExecutor;

    /// True when the OS sandbox backend on this host can deliver real
    /// filesystem isolation (macOS sandbox-exec verified at runtime).  On
    /// hosts where the sandbox mechanism is operationally denied (locked-
    /// down CI, hardened sessions), the OS-level isolation assertions below
    /// cannot run; those tests then stop with a recorded reason instead of
    /// failing, because the AgentCode policy boundary + degraded evidence
    /// are covered by separate tests.
    fn host_supports_filesystem_isolation() -> bool {
        let root =
            std::env::temp_dir().join(format!("agentcode-capability-probe-{}", StableId::new("t")));
        let _ = fs::create_dir_all(&root);
        let supported = SandboxManager::new(SandboxPolicy::new(vec![root.clone()]))
            .diagnostics()
            .max_isolation
            >= ac_sandbox::IsolationLevel::FilesystemIsolated;
        let _ = fs::remove_dir_all(root);
        supported
    }

    /// Returns >4096 bytes of non-ASCII output regardless of the request
    /// payload, so the ToolBroker evidence path is exercised with content that
    /// would panic any byte-slice truncation.
    struct UnicodeEchoExecutor;

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    impl ToolExecutor for EchoExecutor {
        fn execute(&self, request: &ToolRequest) -> AcResult<String> {
            Ok(request.payload.clone())
        }
    }

    impl ToolExecutor for UnicodeEchoExecutor {
        fn execute(&self, _request: &ToolRequest) -> AcResult<String> {
            Ok(format!("{}漢字😀統合", "界".repeat(1400)))
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
    fn toolbroker_large_unicode_output_completes_without_panic_and_is_truncated() {
        // Regression for BF-01: a ToolBroker/model-facing summary containing
        // >4096 bytes of non-ASCII output must complete safely (no byte-slice
        // panic) while raw evidence stays intact and the summary is truncated.
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::ProcessExec("echo".to_string()))
                .allow(Capability::SecretRead("echo".to_string())),
        );
        broker
            .register_tool(
                ToolDefinition {
                    id: "echo".to_string(),
                    version: "1".to_string(),
                    required_capabilities: vec![Capability::ProcessExec("echo".to_string())],
                },
                Box::new(UnicodeEchoExecutor),
            )
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "echo".to_string(),
                    tool_version: "1".to_string(),
                    payload: "ignored".to_string(),
                    capabilities: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(result.status, ToolStatus::Succeeded);
        let record = evidence.get(&result.evidence_ref).unwrap();
        let raw = record.raw_content.as_ref().unwrap();
        assert!(raw.len() > 4096, "fixture must exceed the truncation limit");
        let summary = record.model_summary.as_ref().unwrap();
        assert!(
            summary.ends_with("\n[output truncated]"),
            "summary must carry the truncation marker"
        );
        assert!(summary.len() < raw.len());
        assert!(summary.is_char_boundary(summary.len()));
    }

    #[test]
    fn workspace_registers_browser_and_security_verification_tools() {
        let root = std::env::temp_dir().join(format!("agentcode-tool-reg-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string()))
                .allow(Capability::BrowserAutomation)
                .allow(Capability::SecurityScan),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        assert!(broker.definitions.contains_key("browser.verify"));
        assert!(broker.definitions.contains_key("security.verify"));
        assert!(broker.definitions["browser.verify"]
            .required_capabilities
            .contains(&Capability::BrowserAutomation));
        assert!(broker.definitions["security.verify"]
            .required_capabilities
            .contains(&Capability::SecurityScan));
        // BF-05: the production broker must NOT expose raw write tools that
        // mutate the workspace without ChangeSet authorization.
        assert!(
            !broker.definitions.contains_key("fs.write"),
            "fs.write must not be registered on the production broker"
        );
        assert!(
            !broker.definitions.contains_key("fs.create"),
            "fs.create must not be registered on the production broker"
        );
        assert!(
            !broker.definitions.contains_key("fs.delete"),
            "fs.delete must not be registered on the production broker"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn raw_write_tool_is_not_invokable_through_production_broker() {
        let root =
            std::env::temp_dir().join(format!("agentcode-tool-nowrite-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string())),
        );
        WorkspaceTools::new(root.clone())
            .register_all(&mut broker)
            .unwrap();
        let mut evidence = EvidenceStore::new();
        // A model cannot propose a raw write: the tool is not registered, so
        // invocation must be rejected rather than silently writing the file.
        let err = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "fs.write".to_string(),
                    tool_version: "1".to_string(),
                    payload: "target.txt\nunsanctioned\n".to_string(),
                    capabilities: vec![Capability::FilesystemWrite("*".to_string())],
                },
                &mut evidence,
            )
            .unwrap_err();
        assert_eq!(err.code(), "TOOL-UNKNOWN_TOOL");
        assert!(!root.join("target.txt").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dev_test_without_known_project_command_fails_unavailable_not_passed() {
        let root =
            std::env::temp_dir().join(format!("agentcode-tool-no-test-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new().allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "dev.test".to_string(),
                    tool_version: "1".to_string(),
                    payload: String::new(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        assert_eq!(result.status, ToolStatus::Failed);
        assert!(result.observation.contains("NoKnownTestCommand"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dev_test_executes_and_reports_degraded_isolation_when_backend_lacks_filesystem_sandbox() {
        // Doc 04 §80-81: fixed verification commands inside the trusted
        // worktree must still run when the OS backend cannot deliver
        // filesystem isolation, and the tool observation must record the
        // degraded (requested vs achieved) isolation honestly instead of
        // failing every mission verification.
        let root =
            std::env::temp_dir().join(format!("agentcode-tool-degraded-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Makefile"), "test:\n\techo unit-ok\n").unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new().allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "dev.test".to_string(),
                    tool_version: "1".to_string(),
                    payload: String::new(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        // On hosts with a working OS sandbox backend the command runs fully
        // isolated; on hosts without one it runs degraded — either way it
        // must SUCCEED, and the observation must report the achieved level.
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "dev.test must run on this host: {}",
            result.observation
        );
        assert!(result.observation.contains("unit-ok"));
        assert!(
            result.observation.contains("sandbox_backend:")
                || result.observation.contains("backend:"),
            "observation must record the sandbox backend evidence: {}",
            result.observation
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cmd_exec_still_fails_closed_when_backend_lacks_filesystem_sandbox() {
        // Arbitrary commands never opted into degraded execution: when the
        // OS backend cannot deliver filesystem isolation they fail closed.
        let root =
            std::env::temp_dir().join(format!("agentcode-tool-closed-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new().allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: "/bin/echo\nblocked".to_string(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        if ac_sandbox::SandboxManager::new(ac_sandbox::SandboxPolicy::new(vec![root.clone()]))
            .diagnostics()
            .max_isolation
            < ac_sandbox::IsolationLevel::FilesystemIsolated
        {
            // Degraded host: arbitrary command execution must fail closed.
            assert_eq!(result.status, ToolStatus::Failed);
            assert!(result.observation.contains("SANDBOX-ISOLATION_UNSUPPORTED"));
        } else {
            // Capable host: the echo runs sandboxed.
            assert_eq!(result.status, ToolStatus::Succeeded);
            assert!(result.observation.contains("blocked"));
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn browser_verify_rejects_unauthorized_targets_before_launch() {
        let mut broker =
            ToolBroker::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
        broker
            .register_tool(
                ToolDefinition {
                    id: "browser.verify".to_string(),
                    version: "1".to_string(),
                    required_capabilities: vec![Capability::BrowserAutomation],
                },
                Box::new(BrowserVerifyTool),
            )
            .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "browser.verify".to_string(),
                    tool_version: "1".to_string(),
                    payload: "{\"target_url\":\"file:///etc/passwd\"}".to_string(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        assert_eq!(result.status, ToolStatus::Failed);
        assert!(result
            .observation
            .contains("TOOL-BROWSER_TARGET_UNAUTHORIZED"));
    }

    #[test]
    #[ignore = "requires local Chrome/Chromium"]
    fn operational_browser_verify_runs_real_cdp_through_toolbroker() {
        let fixture = localhost_fixture(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 83\r\n\r\n<html><body><main>AgentCode browser proof</main><button>Verify</button></body></html>",
        );
        let mut broker =
            ToolBroker::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
        broker
            .register_tool(
                ToolDefinition {
                    id: "browser.verify".to_string(),
                    version: "1".to_string(),
                    required_capabilities: vec![Capability::BrowserAutomation],
                },
                Box::new(BrowserVerifyTool),
            )
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "browser.verify".to_string(),
                    tool_version: "1".to_string(),
                    payload: json!({
                        "target_url": fixture.url(),
                        "assertions": [
                            {"kind": "visible_text_contains", "text": "AgentCode browser proof"},
                            {"kind": "no_console_errors"}
                        ],
                        "include_accessibility": true
                    })
                    .to_string(),
                    capabilities: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        fixture.stop();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "{}",
            result.observation
        );
        assert!(result.observation.contains("\"mode\":\"real-cdp\""));
        assert!(result.observation.contains("\"passed\":true"));
        assert!(evidence.records().count() >= 3);
    }

    #[test]
    #[ignore = "requires gitleaks and semgrep installed"]
    fn operational_security_verify_runs_real_gitleaks_and_semgrep_through_toolbroker() {
        let root =
            std::env::temp_dir().join(format!("agentcode-real-scanners-{}", StableId::new("t")));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/app.py"),
            "AWS_ACCESS_KEY_ID = 'AKIAIOSFODNN7EXAMPLE'\nprint('hello')\n",
        )
        .unwrap();
        let rules = root.join("semgrep-rule.yml");
        fs::write(
            &rules,
            "rules:\n  - id: agentcode-print\n    message: print call\n    severity: WARNING\n    languages: [python]\n    pattern: print(...)\n",
        )
        .unwrap();
        run_git(&root, ["init"]);
        run_git(&root, ["config", "user.email", "agentcode@example.invalid"]);
        run_git(&root, ["config", "user.name", "AgentCode Test"]);
        run_git(&root, ["add", "."]);
        run_git(&root, ["commit", "-m", "fixture"]);

        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::SecurityScan)
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "security.verify".to_string(),
                    tool_version: "1".to_string(),
                    payload: json!({
                        "required_scanners": ["gitleaks", "semgrep"],
                        "semgrep_rules_path": rules.display().to_string()
                    })
                    .to_string(),
                    capabilities: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "{}",
            result.observation
        );
        let value: Value = serde_json::from_str(&result.observation).unwrap();
        let executions = value["executions"].as_array().unwrap();
        for scanner in ["gitleaks", "semgrep"] {
            assert!(
                executions.iter().any(|execution| {
                    execution["scanner"] == scanner
                        && execution["status"] == "Available"
                        && execution["provenance"] == "ExternalTool"
                        && execution["version"].as_str().is_some_and(|version| {
                            !version.trim().is_empty() && version != "unknown"
                        })
                }),
                "{}",
                result.observation
            );
        }
        assert!(result.observation.contains("\"scanner\":\"gitleaks\""));
        assert!(result.observation.contains("\"scanner\":\"semgrep\""));
        assert!(result
            .observation
            .contains("\"provenance\":\"ExternalTool\""));
        assert!(!result.observation.contains("AKIAIOSFODNN7EXAMPLE"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scanner_data_dir_outside_managed_root_is_denied() {
        let managed =
            std::env::temp_dir().join(format!("agentcode-managed-scanners-{}", StableId::new("t")));
        let attacker = std::env::temp_dir().join(format!(
            "agentcode-attacker-scanners-{}",
            StableId::new("t")
        ));
        fs::create_dir_all(&managed).unwrap();
        fs::create_dir_all(&attacker).unwrap();
        std::env::set_var("AGENTCODE_SCANNER_DATA_DIR", &managed);
        let input = SecurityVerifyInput::parse(
            &json!({
                "required_scanners": ["trivy"],
                "scanner_data_dir": attacker.display().to_string()
            })
            .to_string(),
        )
        .unwrap();
        let err = trusted_scanner_data_root(&input).unwrap_err();
        assert_eq!(err.code(), "TOOL-SCANNER_DATA_ROOT_DENIED");
        std::env::remove_var("AGENTCODE_SCANNER_DATA_DIR");
        let _ = fs::remove_dir_all(managed);
        let _ = fs::remove_dir_all(attacker);
    }

    #[test]
    #[ignore = "requires osv-scanner and trivy installed"]
    fn operational_security_verify_runs_real_osv_and_trivy_through_toolbroker() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf();
        let scanner_data =
            std::env::temp_dir().join(format!("agentcode-scanner-data-{}", StableId::new("t")));
        seed_trivy_cache(&scanner_data);
        prepare_osv_offline_database(&root, &scanner_data);
        std::env::set_var("AGENTCODE_SCANNER_DATA_DIR", &scanner_data);
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::SecurityScan)
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "security.verify".to_string(),
                    tool_version: "1".to_string(),
                    payload: json!({
                        "required_scanners": ["osv", "trivy"],
                        "scanner_data_dir": scanner_data.display().to_string()
                    })
                    .to_string(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "{}",
            result.observation
        );
        assert!(result.observation.contains("\"scanner\":\"osv-scanner\""));
        assert!(result.observation.contains("\"scanner\":\"trivy\""));
        assert!(result
            .observation
            .contains("\"provenance\":\"ExternalTool\""));
        std::env::remove_var("AGENTCODE_SCANNER_DATA_DIR");
        let _ = fs::remove_dir_all(scanner_data);
    }

    #[test]
    #[ignore = "requires trivy installed"]
    fn operational_security_verify_runs_real_trivy_through_toolbroker() {
        let root =
            std::env::temp_dir().join(format!("agentcode-real-trivy-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("README.md"),
            "AgentCode Trivy zero-finding fixture\n",
        )
        .unwrap();
        let scanner_data =
            std::env::temp_dir().join(format!("agentcode-scanner-data-{}", StableId::new("t")));
        seed_trivy_cache(&scanner_data);
        std::env::set_var("AGENTCODE_SCANNER_DATA_DIR", &scanner_data);
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::SecurityScan)
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "security.verify".to_string(),
                    tool_version: "1".to_string(),
                    payload: json!({
                        "required_scanners": ["trivy"],
                        "scanner_data_dir": scanner_data.display().to_string()
                    })
                    .to_string(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "{}",
            result.observation
        );
        let value: Value = serde_json::from_str(&result.observation).unwrap();
        let executions = value["executions"].as_array().unwrap();
        assert!(
            executions.iter().any(|execution| {
                execution["scanner"] == "trivy"
                    && execution["status"] == "Available"
                    && execution["provenance"] == "ExternalTool"
                    && execution["version"]
                        .as_str()
                        .is_some_and(|version| !version.trim().is_empty() && version != "unknown")
            }),
            "{}",
            result.observation
        );
        // Verify FilesystemIsolated was achieved, not just ProcessRestricted
        assert!(
            result
                .observation
                .contains("achieved_isolation:FilesystemIsolated"),
            "standalone Trivy test must prove FilesystemIsolated, got: {}",
            result.observation
        );
        std::env::remove_var("AGENTCODE_SCANNER_DATA_DIR");
        let _ = fs::remove_dir_all(scanner_data);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "requires checkov installed"]
    fn operational_security_verify_runs_real_checkov_through_toolbroker() {
        let root =
            std::env::temp_dir().join(format!("agentcode-real-checkov-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("main.tf"),
            "resource \"aws_security_group\" \"bad\" {\n  ingress {\n    from_port = 22\n    to_port = 22\n    protocol = \"tcp\"\n    cidr_blocks = [\"0.0.0.0/0\"]\n  }\n}\n",
        )
        .unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::SecurityScan)
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "security.verify".to_string(),
                    tool_version: "1".to_string(),
                    payload: json!({"required_scanners": ["checkov"]}).to_string(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "{}",
            result.observation
        );
        assert!(result.observation.contains("\"scanner\":\"checkov\""));
        assert!(result
            .observation
            .contains("\"provenance\":\"ExternalTool\""));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "requires ZAP installed"]
    fn operational_security_verify_runs_real_zap_against_localhost_through_toolbroker() {
        let fixture = localhost_fixture(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 44\r\n\r\n<html><body>AgentCode ZAP proof</body></html>",
        );
        let root = std::env::temp_dir().join(format!("agentcode-real-zap-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::SecurityScan)
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::with_required_isolation(
            root.clone(),
            ac_sandbox::IsolationLevel::FilesystemIsolated,
        )
        .register_all(&mut broker)
        .unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "security.verify".to_string(),
                    tool_version: "1".to_string(),
                    payload: json!({
                        "required_scanners": ["zap"],
                        "dast_target": fixture.url(),
                        "dast_target_authorized": true
                    })
                    .to_string(),
                    capabilities: Vec::new(),
                },
                &mut EvidenceStore::new(),
            )
            .unwrap();
        fixture.stop();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "{}",
            result.observation
        );
        assert!(result.observation.contains("\"scanner\":\"zap\""));
        assert!(result
            .observation
            .contains("\"provenance\":\"ExternalTool\""));
        let _ = fs::remove_dir_all(root);
    }

    struct LocalhostFixture {
        url: String,
        stop: Arc<AtomicBool>,
        handle: thread::JoinHandle<()>,
    }

    impl LocalhostFixture {
        fn url(&self) -> String {
            self.url.clone()
        }

        fn stop(self) {
            self.stop.store(true, Ordering::Relaxed);
            let _ = self.handle.join();
        }
    }

    fn localhost_fixture(response: &'static str) -> LocalhostFixture {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let handle = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(120);
            while Instant::now() < deadline && !signal.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0_u8; 1024];
                    let _ = std::io::Read::read(&mut stream, &mut buffer);
                    let _ = stream.write_all(response.as_bytes());
                }
                thread::sleep(Duration::from_millis(25));
            }
        });
        LocalhostFixture { url, stop, handle }
    }

    fn seed_trivy_cache(scanner_data: &Path) {
        let Some(home) = std::env::var_os("HOME") else {
            panic!("HOME is required to locate the preinstalled Trivy cache");
        };
        let source = PathBuf::from(home).join("Library/Caches/trivy");
        assert!(
            source.join("db/trivy.db").is_file() && source.join("db/metadata.json").is_file(),
            "preinstalled Trivy cache is required for offline operational proof"
        );
        let target = scanner_data.join("trivy");
        copy_dir_all(&source, &target).unwrap();
    }

    fn prepare_osv_offline_database(project_root: &Path, scanner_data: &Path) {
        let target = scanner_data.join("osv-scanner");
        fs::create_dir_all(&target).unwrap();
        let output = Command::new("osv-scanner")
            .args([
                "scan",
                "source",
                "--offline-vulnerabilities",
                "--download-offline-databases",
                "--format",
                "json",
                project_root.to_str().unwrap(),
            ])
            .env("OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY", target.join("osv"))
            .env(
                "OSV_SCALIBR_LOCAL_DB_CACHE_DIRECTORY",
                target.join("scalibr"),
            )
            .output()
            .unwrap();
        assert!(
            matches!(output.status.code(), Some(0) | Some(1)),
            "OSV offline database preparation failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            let target = to.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir_all(&entry.path(), &target)?;
            } else {
                fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }

    #[test]
    fn workspace_command_executes_through_sandbox_plan() {
        if !host_supports_filesystem_isolation() {
            eprintln!(
                "SKIP: OS sandbox backend cannot deliver filesystem isolation on this host; \
                 real-isolation execution covered by degraded-evidence tests"
            );
            return;
        }
        let root = std::env::temp_dir().join(format!("agentcode-tool-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::new(root.clone())
            .register_all(&mut broker)
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let workspace_file = root.join("workspace-readable.txt");
        fs::write(&workspace_file, "workspace-ok\n").unwrap();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: format!("/bin/cat\n{}", workspace_file.display()),
                    capabilities: vec![Capability::ProcessExec("/bin/cat".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "workspace read must succeed: {}",
            result.observation
        );
        assert!(result.observation.contains("workspace-ok"));
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));

        let workspace_write = root.join("workspace-write.txt");
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: format!("/usr/bin/touch\n{}", workspace_write.display()),
                    capabilities: vec![Capability::ProcessExec("/usr/bin/touch".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "workspace write must succeed: {}",
            result.observation
        );
        assert!(workspace_write.exists());
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));

        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: "echo\nok".to_string(),
                    capabilities: vec![Capability::ProcessExec("echo".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "repo.status must succeed under macOS sandbox: {}",
            result.observation
        );
        assert!(result.observation.contains("ok"));
        assert!(result
            .observation
            .contains("sandbox_backend:macos-sandbox-exec"));
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));
    }

    #[test]
    fn repository_tool_records_execution_evidence() {
        if !host_supports_filesystem_isolation() {
            eprintln!(
                "SKIP: OS sandbox backend cannot deliver filesystem isolation on this host; \
                 real-isolation execution covered by degraded-evidence tests"
            );
            return;
        }
        let root = std::env::temp_dir().join(format!("agentcode-tool-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("README.md"), "hello\n").unwrap();
        run_git(&root, ["init"]);
        run_git(&root, ["add", "."]);
        run_git(
            &root,
            [
                "-c",
                "user.name=AgentCode Test",
                "-c",
                "user.email=agentcode@example.test",
                "commit",
                "-m",
                "initial",
            ],
        );
        fs::write(root.join("README.md"), "changed\n").unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::new(root.clone())
            .register_all(&mut broker)
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "repo.status".to_string(),
                    tool_version: "1".to_string(),
                    payload: String::new(),
                    capabilities: vec![Capability::ProcessExec("git".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Succeeded,
            "repo.status must succeed under macOS sandbox: {}",
            result.observation
        );
        assert!(result.observation.contains("README.md"));
        assert!(result
            .observation
            .contains("sandbox_backend:macos-sandbox-exec"));
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));
        assert_eq!(evidence.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn governed_network_request_is_denied_by_default() {
        if !host_supports_filesystem_isolation() {
            eprintln!(
                "SKIP: OS sandbox backend cannot deliver filesystem isolation on this host; \
                 network denial-at-OS-level requires a real sandbox backend"
            );
            return;
        }
        let root = std::env::temp_dir().join(format!("agentcode-tool-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let done = Arc::new(AtomicBool::new(false));
        let done_signal = done.clone();
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(30);
            while Instant::now() < deadline && !done_signal.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    let _ = std::io::Write::write_all(
                        &mut stream,
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
                    );
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let direct = Command::new("/usr/bin/curl")
            .args(["-s", "-m", "5", &format!("http://127.0.0.1:{port}/")])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&direct.stdout).trim(),
            "ok",
            "fixture must be reachable directly"
        );
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::new(root.clone())
            .register_all(&mut broker)
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: format!("/usr/bin/curl\n-sS\n-m\n5\nhttp://127.0.0.1:{port}/"),
                    capabilities: vec![Capability::ProcessExec("/usr/bin/curl".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        done.store(true, Ordering::Relaxed);
        handle.join().unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Failed,
            "network must be denied by default: {}",
            result.observation
        );
        assert!(
            result.observation.contains("status:7")
                || result.observation.contains("Operation not permitted"),
            "{}",
            result.observation
        );
        assert!(result
            .observation
            .contains("sandbox_backend:macos-sandbox-exec"));
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let allow_port = listener.local_addr().unwrap().port();
        let done = Arc::new(AtomicBool::new(false));
        let done_signal = done.clone();
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(30);
            while Instant::now() < deadline && !done_signal.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    let _ = std::io::Write::write_all(
                        &mut stream,
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
                    );
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let allow_sandbox = SandboxManager::new(SandboxPolicy {
            workspace_roots: vec![root.clone()],
            capability_policy: CapabilityPolicy::new()
                .allow(Capability::ProcessExec("*".to_string()))
                .allow(Capability::Network("*".to_string())),
            network_default_allow: true,
            max_timeout_ms: 30_000,
            required_isolation: ac_sandbox::IsolationLevel::FilesystemIsolated,
            ..SandboxPolicy::new(vec![root.clone()])
        });
        let allow_plan = allow_sandbox
            .prepare_execution(ExecRequest {
                argv: vec![
                    "/usr/bin/curl".to_string(),
                    "-sS".to_string(),
                    "-m".to_string(),
                    "5".to_string(),
                    format!("http://127.0.0.1:{allow_port}/"),
                ],
                cwd: root.clone(),
                env: toolchain_env(),
                network: true,
                timeout_ms: 5_000,
            })
            .unwrap();
        let allow_result = ProcessManager::default()
            .run("network-allow", allow_plan)
            .unwrap();
        done.store(true, Ordering::Relaxed);
        handle.join().unwrap();
        assert_eq!(
            allow_result.exit_code,
            Some(0),
            "{}",
            format_process_observation(&allow_result)
        );
        assert_eq!(allow_result.stdout.trim(), "ok");
        assert_eq!(
            allow_result.record.sandbox.achieved_isolation,
            ac_sandbox::IsolationLevel::FilesystemIsolated
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn governed_command_cannot_read_sensitive_file_outside_workspace() {
        if !host_supports_filesystem_isolation() {
            eprintln!(
                "SKIP: OS sandbox backend cannot deliver filesystem isolation on this host; \
                 outside-workspace OS blocking requires a real sandbox backend"
            );
            return;
        }
        let root = std::env::temp_dir().join(format!("agentcode-tool-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let home = std::env::var("HOME").unwrap_or_default();
        let documents_dir = Path::new(&home).join("Documents");
        fs::create_dir_all(&documents_dir).unwrap();
        let outside_file = documents_dir.join(format!(
            "agentcode-outside-workspace-probe-{}",
            StableId::new("t")
        ));
        fs::write(&outside_file, "harmless outside workspace probe\n").unwrap();
        let outside_write = documents_dir.join(format!(
            "agentcode-outside-workspace-write-{}",
            StableId::new("t")
        ));
        let sensitive_dir = Path::new(&home).join(".ssh");
        fs::create_dir_all(&sensitive_dir).unwrap();
        let sensitive_file =
            sensitive_dir.join(format!("agentcode-sensitive-probe-{}", StableId::new("t")));
        fs::write(&sensitive_file, "fake-secret-value\n").unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
        );
        WorkspaceTools::new(root.clone())
            .register_all(&mut broker)
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: format!("/bin/cat\n{}", outside_file.display()),
                    capabilities: vec![Capability::ProcessExec("/bin/cat".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Failed,
            "arbitrary home file read must be denied: {}",
            result.observation
        );
        assert!(
            result.observation.contains("Operation not permitted"),
            "{}",
            result.observation
        );
        assert!(result
            .observation
            .contains("sandbox_backend:macos-sandbox-exec"));
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: format!("/usr/bin/touch\n{}", outside_write.display()),
                    capabilities: vec![Capability::ProcessExec("/usr/bin/touch".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Failed,
            "outside workspace write must be denied: {}",
            result.observation
        );
        assert!(
            result.observation.contains("Operation not permitted"),
            "{}",
            result.observation
        );
        assert!(result
            .observation
            .contains("sandbox_backend:macos-sandbox-exec"));
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));
        assert!(!outside_write.exists());
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "cmd.exec".to_string(),
                    tool_version: "1".to_string(),
                    payload: format!("/bin/cat\n{}", sensitive_file.display()),
                    capabilities: vec![Capability::ProcessExec("/bin/cat".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            result.status,
            ToolStatus::Failed,
            "sensitive file read must be denied: {}",
            result.observation
        );
        assert!(
            result.observation.contains("Operation not permitted"),
            "{}",
            result.observation
        );
        assert!(result
            .observation
            .contains("sandbox_backend:macos-sandbox-exec"));
        assert!(result
            .observation
            .contains("achieved_isolation:FilesystemIsolated"));
        let _ = fs::remove_file(&outside_file);
        let _ = fs::remove_file(&sensitive_file);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sandbox_profile_files_are_removed_after_success_cancellation_and_timeout() {
        if !host_supports_filesystem_isolation() {
            eprintln!(
                "SKIP: OS sandbox backend cannot deliver filesystem isolation on this host; \
                 sandbox profile cleanup requires the macOS sandbox-exec backend"
            );
            return;
        }
        let root = std::env::temp_dir().join(format!("agentcode-tool-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let sandbox = workspace_sandbox(&root, ac_sandbox::IsolationLevel::FilesystemIsolated);
        let manager = ProcessManager::default();
        let success = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/echo".to_string(), "cleanup".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 1_000,
            })
            .unwrap();
        let success_profiles = success.cleanup_paths.clone();
        assert!(!success_profiles.is_empty(), "macOS plan owns a profile");
        manager.run("cleanup", success).unwrap();
        for profile in &success_profiles {
            assert!(!profile.exists(), "profile leaked after successful exit");
        }
        let timeout = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/sleep".to_string(), "1".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 20,
            })
            .unwrap();
        let timeout_profiles = timeout.cleanup_paths.clone();
        assert_eq!(
            manager.run("timeout", timeout).unwrap_err().code(),
            "TOOL-COMMAND_TIMEOUT"
        );
        for profile in &timeout_profiles {
            assert!(!profile.exists(), "profile leaked after timeout");
        }
        let cancelled = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/sleep".to_string(), "1".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 1_000,
            })
            .unwrap();
        let cancelled_profiles = cancelled.cleanup_paths.clone();
        let cancelled_flag = Arc::new(AtomicBool::new(false));
        let signal = cancelled_flag.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            signal.store(true, Ordering::Relaxed);
        });
        assert_eq!(
            manager
                .run_with_cancellation("cancel", cancelled, &cancelled_flag)
                .unwrap_err()
                .code(),
            "TOOL-COMMAND_CANCELLED"
        );
        for profile in &cancelled_profiles {
            assert!(!profile.exists(), "profile leaked after cancellation");
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn process_manager_caps_finished_records_without_losing_running_state() {
        // P1-06: ProcessManager in-memory finished records are observability
        // only and must be bounded so a long-running mission cannot grow memory
        // without limit.  Running records are never evicted.
        let mut records = BTreeMap::new();
        let running_id = StableId::new("running");
        let sandbox_ev = ac_sandbox::SandboxEvidence {
            backend_name: String::new(),
            requested_isolation: ac_sandbox::IsolationLevel::ProcessRestricted,
            achieved_isolation: ac_sandbox::IsolationLevel::ProcessRestricted,
            workspace_roots: Vec::new(),
            network_policy: ac_sandbox::NetworkPolicy::DenyAll,
            allowed_env_keys: Vec::new(),
            timeout_ms: 1_000,
            max_output_bytes: 8192,
            process_tree_control: false,
        };
        records.insert(
            running_id.clone(),
            ProcessRecord {
                id: running_id.clone(),
                task: "running-task".to_string(),
                manifest: ExecutionManifest {
                    argv: Vec::new(),
                    cwd: PathBuf::from("/tmp"),
                    timeout_ms: 1_000,
                    network: false,
                },
                pid: 1,
                state: ProcessState::Running,
                started_at: TimestampMillis::now(),
                sandbox: sandbox_ev.clone(),
                cleanup_paths: Vec::new(),
            },
        );
        let base = TimestampMillis::now().as_millis();
        for i in 0..(MAX_PROCESS_RECORDS + 64) {
            let id = StableId::new("finished");
            records.insert(
                id.clone(),
                ProcessRecord {
                    id: id.clone(),
                    task: "finished-task".to_string(),
                    manifest: ExecutionManifest {
                        argv: Vec::new(),
                        cwd: PathBuf::from("/tmp"),
                        timeout_ms: 1_000,
                        network: false,
                    },
                    pid: 1000 + i as u32,
                    state: ProcessState::Finished,
                    started_at: TimestampMillis::from_millis(base + i as u128),
                    sandbox: sandbox_ev.clone(),
                    cleanup_paths: Vec::new(),
                },
            );
        }
        ProcessManager::prune_records(&mut records);
        assert!(
            records.len() <= MAX_PROCESS_RECORDS,
            "finished records must be capped, got {}",
            records.len()
        );
        assert!(
            records.contains_key(&running_id),
            "running record must never be evicted"
        );
    }

    fn permitted_sandbox(root: PathBuf, timeout_ms: u64) -> SandboxManager {
        SandboxManager::new(SandboxPolicy {
            workspace_roots: vec![root.clone()],
            capability_policy: CapabilityPolicy::new()
                .allow(Capability::ProcessExec("*".to_string())),
            network_default_allow: false,
            max_timeout_ms: timeout_ms,
            required_isolation: ac_sandbox::IsolationLevel::ProcessRestricted,
            ..SandboxPolicy::new(vec![root])
        })
    }

    #[test]
    fn workspace_guard_blocks_traversal_and_symlink_escapes() {
        let root = std::env::temp_dir().join(format!("agentcode-guard-{}", StableId::new("t")));
        let outside =
            std::env::temp_dir().join(format!("agentcode-outside-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        fs::write(&outside, "outside").unwrap();
        assert_eq!(
            safe_join(&root, "../outside").unwrap_err().code(),
            "TOOL-PATH_ESCAPE"
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();
            assert_eq!(
                safe_join(&root, "escape").unwrap_err().code(),
                "TOOL-PATH_SYMLINK_ESCAPE"
            );
        }
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(outside);
    }

    #[test]
    fn process_manager_times_out_and_cancels_background_process() {
        let root = std::env::temp_dir().join(format!("agentcode-process-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let sandbox = permitted_sandbox(root.clone(), 1_000);
        let manager = ProcessManager::default();
        let timeout_plan = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/sleep".to_string(), "1".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 20,
            })
            .unwrap();
        assert_eq!(
            manager.run("timeout", timeout_plan).unwrap_err().code(),
            "TOOL-COMMAND_TIMEOUT"
        );
        let background_plan = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/sleep".to_string(), "1".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 1_000,
            })
            .unwrap();
        let id = manager
            .start_background("background", background_plan)
            .unwrap();
        assert_eq!(manager.inspect(&id).unwrap().state, ProcessState::Running);
        manager.cancel(&id).unwrap();
        assert_eq!(manager.inspect(&id).unwrap().state, ProcessState::Cancelled);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn governed_scanner_executor_uses_the_sandboxed_process_path() {
        let root = std::env::temp_dir().join(format!("agentcode-scanner-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let executor = GovernedScannerExecutor::new(permitted_sandbox(root.clone(), 1_000));
        let result = executor
            .execute(ScannerProcessRequest {
                adapter: ac_security::SecurityAdapter::Gitleaks,
                executable: "/bin/echo".to_string(),
                argv: vec!["/bin/echo".to_string(), "scanner-version".to_string()],
                cwd: root.clone(),
                timeout_ms: 1_000,
                network: false,
                cleanup_paths: Vec::new(),
                report_path: None,
                env: BTreeMap::new(),
            })
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout.trim(), "scanner-version");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cancellation_is_structured_and_keeps_partial_output_evidence() {
        let root = std::env::temp_dir().join(format!("agentcode-cancel-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let sandbox = permitted_sandbox(root.clone(), 1_000);
        let plan = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/sleep".to_string(), "1".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 1_000,
            })
            .unwrap();
        let manager = ProcessManager::default();
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = cancelled.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            signal.store(true, Ordering::Relaxed);
        });
        assert_eq!(
            manager
                .run_with_cancellation("cancel", plan, &cancelled)
                .unwrap_err()
                .code(),
            "TOOL-COMMAND_CANCELLED"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn process_env_is_minimized_and_allowed_env_survives() {
        let root = std::env::temp_dir().join(format!("agentcode-env-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        std::env::set_var("AGENTCODE_PARENT_CANARY", "parent-secret");
        let sandbox = permitted_sandbox(root.clone(), 1_000);
        let plan = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/usr/bin/env".to_string()],
                cwd: root.clone(),
                env: BTreeMap::from([("AGENTCODE_ALLOWED".to_string(), "visible".to_string())]),
                network: false,
                timeout_ms: 1_000,
            })
            .unwrap();
        let result = ProcessManager::default().run("env", plan).unwrap();
        assert!(result.stdout.contains("AGENTCODE_ALLOWED=visible"));
        assert!(!result.stdout.contains("AGENTCODE_PARENT_CANARY"));
        assert_eq!(
            result.record.sandbox.achieved_isolation,
            ac_sandbox::IsolationLevel::ProcessRestricted
        );
        std::env::remove_var("AGENTCODE_PARENT_CANARY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn process_output_is_bounded_and_truncated_marker_is_recorded() {
        let root = std::env::temp_dir().join(format!("agentcode-output-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let sandbox = SandboxManager::new(SandboxPolicy {
            workspace_roots: vec![root.clone()],
            capability_policy: CapabilityPolicy::new()
                .allow(Capability::ProcessExec("*".to_string())),
            network_default_allow: false,
            max_timeout_ms: 1_000,
            required_isolation: ac_sandbox::IsolationLevel::ProcessRestricted,
            max_output_bytes: 4,
            allow_degraded_execution: false,
        });
        let plan = sandbox
            .prepare_execution(ExecRequest {
                argv: vec!["/bin/echo".to_string(), "abcdef".to_string()],
                cwd: root.clone(),
                env: BTreeMap::new(),
                network: false,
                timeout_ms: 1_000,
            })
            .unwrap();
        let result = ProcessManager::default().run("output", plan).unwrap();
        assert_eq!(result.stdout, "abcd");
        assert!(result.stdout_truncated);
        let observation = format_process_observation(&result);
        assert!(observation.contains("stdout_truncated:true"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn approved_secret_process_is_redacted_from_agent_result() {
        let root = std::env::temp_dir().join(format!("agentcode-secret-{}", StableId::new("t")));
        fs::create_dir_all(&root).unwrap();
        let mut broker = ToolBroker::new(
            CapabilityPolicy::new()
                .allow(Capability::ProcessExec("*".to_string()))
                .allow(Capability::SecretRead("*".to_string())),
        );
        broker
            .register_tool(
                ToolDefinition {
                    id: "secret.exec".to_string(),
                    version: "1".to_string(),
                    required_capabilities: vec![Capability::ProcessExec("*".to_string())],
                },
                Box::new(EchoExecutor),
            )
            .unwrap();
        let sandbox = SandboxManager::new(SandboxPolicy {
            workspace_roots: vec![root.clone()],
            capability_policy: CapabilityPolicy::new()
                .allow(Capability::ProcessExec("*".to_string()))
                .allow(Capability::SecretRead("*".to_string())),
            network_default_allow: false,
            max_timeout_ms: 1_000,
            required_isolation: ac_sandbox::IsolationLevel::ProcessRestricted,
            ..SandboxPolicy::new(vec![root.clone()])
        });
        let mut secrets = SecretBroker::default();
        secrets.insert("test.canary", "canary-secret").unwrap();
        let refs = BTreeMap::from([(
            "AGENTCODE_TEST_SECRET".to_string(),
            "test.canary".to_string(),
        )]);
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke_process_with_secrets(
                &ProcessManager::default(),
                SecretProcessRequest {
                    request: ToolRequest {
                        id: StableId::new("toolreq"),
                        tool_id: "secret.exec".to_string(),
                        tool_version: "1".to_string(),
                        payload: String::new(),
                        capabilities: vec![Capability::ProcessExec("/usr/bin/env".to_string())],
                    },
                    sandbox: &sandbox,
                    argv: vec!["/usr/bin/env".to_string()],
                    cwd: root.clone(),
                    timeout_ms: 1_000,
                    secret_references: &refs,
                },
                &secrets,
                &mut evidence,
            )
            .unwrap();
        assert_eq!(result.status, ToolStatus::Succeeded);
        assert!(!result.observation.contains("canary-secret"));
        assert!(evidence
            .get(&result.evidence_ref)
            .unwrap()
            .raw_content
            .as_ref()
            .unwrap()
            .contains("canary-secret"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn phase16_mcp_tool_routes_through_broker_policy() {
        let server_id = StableId::new("mcp");
        let tool = McpToolRecord {
            id: StableId::new("mcptool"),
            server_id: server_id.clone(),
            name: "write-file".to_string(),
            description: "Advertised write tool".to_string(),
            schema: "{\"type\":\"object\"}".to_string(),
            risk: ac_security::RiskClass::R3,
            required_capabilities: [Capability::FilesystemWrite("*".to_string())]
                .into_iter()
                .collect(),
        };
        let mut denied_broker = ToolBroker::new(CapabilityPolicy::new());
        let broker_tool_id = register_mcp_tool_with_broker(&mut denied_broker, &tool).unwrap();
        let mut evidence = EvidenceStore::new();
        let denied = denied_broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: broker_tool_id.clone(),
                    tool_version: "1".to_string(),
                    payload: "{\"path\":\"secret\"}".to_string(),
                    capabilities: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(denied.status, ToolStatus::Denied);

        let mut allowed_broker = ToolBroker::new(
            CapabilityPolicy::new().allow(Capability::FilesystemWrite("*".to_string())),
        );
        register_mcp_tool_with_broker(&mut allowed_broker, &tool).unwrap();
        let allowed = allowed_broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: broker_tool_id,
                    tool_version: "1".to_string(),
                    payload: "{\"path\":\"workspace/file\"}".to_string(),
                    capabilities: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(allowed.status, ToolStatus::Succeeded);
        assert!(allowed.observation.contains(server_id.as_str()));
    }
}
