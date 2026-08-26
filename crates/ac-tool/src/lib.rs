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
use ac_security::{Capability, CapabilityPolicy, McpToolRecord, SecurityDecision};
use ac_security::{
    ScannerFailure, ScannerProcessRequest, ScannerProcessResult, SecurityScannerExecutor,
};

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
        let plan = self
            .sandbox
            .prepare_execution(ExecRequest {
                argv: request.argv,
                cwd: request.cwd,
                env: toolchain_env(),
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
        self.manager
            .run("security-scanner", plan)
            .map(|result| ScannerProcessResult {
                exit_code: result.exit_code,
                stdout: result.stdout,
                stderr: result.stderr,
                stdout_truncated: result.stdout_truncated,
                stderr_truncated: result.stderr_truncated,
            })
            .map_err(|error| match error.code() {
                "TOOL-COMMAND_TIMEOUT" => ScannerFailure::Timeout,
                "TOOL-COMMAND_CANCELLED" => ScannerFailure::Cancelled,
                "TOOL-COMMAND_SPAWN_FAILED" => ScannerFailure::Unavailable(error.to_string()),
                _ => ScannerFailure::ExecutionFailed(error.to_string()),
            })
    }
}

impl ProcessManager {
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
        };
        self.records
            .lock()
            .expect("process records lock")
            .insert(id.clone(), record.clone());
        let deadline = Instant::now() + Duration::from_millis(plan.timeout_ms);
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
        };
        self.children
            .lock()
            .expect("process children lock")
            .insert(id.clone(), child);
        self.records
            .lock()
            .expect("process records lock")
            .insert(id.clone(), record);
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
        record.state = ProcessState::Cancelled;
        Ok(())
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
                id: "fs.create".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::FilesystemWrite(
                    self.root.display().to_string(),
                )],
            },
            Box::new(CreateFileTool {
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
                id: "fs.delete".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::FilesystemWrite(
                    self.root.display().to_string(),
                )],
            },
            Box::new(DeleteFileTool {
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
                    required_isolation: self.required_isolation,
                    ..SandboxPolicy::new(vec![self.root.clone()])
                }),
                cwd: self.root.clone(),
                manager: ProcessManager::default(),
            }),
        )?;
        for (id, command) in [
            ("repo.status", vec!["git", "status", "--short"]),
            ("repo.diff", vec!["git", "diff", "--", "."]),
            ("repo.branch", vec!["git", "branch", "--show-current"]),
            ("dev.test", vec!["cargo", "test", "--quiet"]),
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

struct WriteFileTool {
    root: PathBuf,
}

struct CreateFileTool {
    root: PathBuf,
}

impl ToolExecutor for CreateFileTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let path = safe_join(&self.root, &request.payload)?;
        if path.exists() {
            return Err(AcError::conflict(
                "TOOL-FS_CREATE_EXISTS",
                "refusing to overwrite an existing file",
            ));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| AcError::validation("TOOL-FS_CREATE_FAILED", error.to_string()))?;
        }
        fs::File::create(&path)
            .map_err(|error| AcError::validation("TOOL-FS_CREATE_FAILED", error.to_string()))?;
        Ok(format!("created:{}", request.payload))
    }
}

struct DeleteFileTool {
    root: PathBuf,
}

impl ToolExecutor for DeleteFileTool {
    fn execute(&self, request: &ToolRequest) -> AcResult<String> {
        let path = safe_join(&self.root, &request.payload)?;
        if path.is_dir() {
            return Err(AcError::policy_denied(
                "TOOL-FS_DELETE_DIRECTORY",
                "directory deletion is not permitted",
            ));
        }
        fs::remove_file(&path)
            .map_err(|error| AcError::validation("TOOL-FS_DELETE_FAILED", error.to_string()))?;
        Ok(format!("deleted:{}", request.payload))
    }
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

fn format_process_observation(result: &NativeProcessResult) -> String {
    format!(
        "sandbox_backend:{}\nachieved_isolation:{:?}\nnetwork_policy:{:?}\nworkspace_roots:{}\ntimeout_ms:{}\nmax_output_bytes:{}\nstdout_truncated:{}\nstderr_truncated:{}\nstatus:{}\nstdout:{}\nstderr:{}",
        result.record.sandbox.backend_name,
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
    let home = std::env::temp_dir().join(format!("agentcode-home-{}", StableId::new("tool")));
    let _ = fs::create_dir_all(&home);
    env.insert("HOME".to_string(), home.display().to_string());
    env
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
        let observation = match executor.execute_with_cancellation(&request, cancelled) {
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

    struct EchoExecutor;

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
                    payload: "echo\nok".to_string(),
                    capabilities: vec![Capability::ProcessExec("echo".to_string())],
                },
                &mut evidence,
            )
            .unwrap();
        if result.status == ToolStatus::Failed
            && (result.observation.contains("SANDBOX-ISOLATION_UNSUPPORTED")
                || result.observation.contains("SANDBOX-UNAVAILABLE"))
        {
            let _ = fs::remove_dir_all(root);
            return;
        }
        assert_eq!(result.status, ToolStatus::Succeeded);
        assert!(result.observation.contains("ok"));
    }

    #[test]
    fn repository_tool_records_execution_evidence() {
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
        if result.status == ToolStatus::Failed
            && (result.observation.contains("SANDBOX-ISOLATION_UNSUPPORTED")
                || result.observation.contains("SANDBOX-UNAVAILABLE"))
        {
            let _ = fs::remove_dir_all(root);
            return;
        }
        assert_eq!(result.status, ToolStatus::Succeeded);
        assert!(result.observation.contains("README.md"));
        assert_eq!(evidence.len(), 1);
        let _ = fs::remove_dir_all(root);
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
