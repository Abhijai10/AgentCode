use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_CLIENTS: usize = 8;
const FRAME_TIMEOUT: Duration = Duration::from_secs(5);
pub const IPC_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone)]
pub struct UnixIpcServer {
    path: PathBuf,
    daemon_euid: u32,
    active_clients: Arc<AtomicUsize>,
    request_tx: mpsc::SyncSender<IpcDispatchRequest>,
    request_rx: Arc<std::sync::Mutex<mpsc::Receiver<IpcDispatchRequest>>>,
    peer_credentials: Arc<dyn PeerCredentialProvider>,
}

impl UnixIpcServer {
    pub fn bind(path: impl Into<PathBuf>) -> AcResult<(Self, UnixListener)> {
        Self::bind_with_peer_credentials(path, Arc::new(UnixPeerCredentialProvider))
    }

    fn bind_with_peer_credentials(
        path: impl Into<PathBuf>,
        peer_credentials: Arc<dyn PeerCredentialProvider>,
    ) -> AcResult<(Self, UnixListener)> {
        let path = path.into();
        if path.exists() {
            std::fs::remove_file(&path).map_err(|error| {
                AcError::conflict("DAEMON-IPC_SOCKET_BUSY", error.to_string())
            })?;
        }
        let listener = UnixListener::bind(&path)
            .map_err(|error| AcError::validation("DAEMON-IPC_BIND", error.to_string()))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| AcError::validation("DAEMON-IPC_PERMISSIONS", error.to_string()))?;
        let daemon_euid = std::fs::metadata(&path)
            .map_err(|error| AcError::validation("DAEMON-IPC_METADATA", error.to_string()))?
            .uid();
        listener
            .set_nonblocking(true)
            .map_err(|error| AcError::validation("DAEMON-IPC_CONFIG", error.to_string()))?;
        let (request_tx, request_rx) = mpsc::sync_channel(MAX_CLIENTS);
        Ok((
            Self {
                path,
                daemon_euid,
                active_clients: Arc::new(AtomicUsize::new(0)),
                request_tx,
                request_rx: Arc::new(std::sync::Mutex::new(request_rx)),
                peer_credentials,
            },
            listener,
        ))
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn serve_once(&self, listener: &UnixListener, daemon: &mut DaemonService) -> AcResult<bool> {
        loop {
            match listener.accept() {
                Ok((stream, _)) => self.spawn_client(stream)?,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(AcError::validation("DAEMON-IPC_ACCEPT", error.to_string())),
            }
        }
        let request = match self.request_rx.lock().map_err(|_| {
            AcError::conflict("DAEMON-IPC_QUEUE_POISONED", "IPC request queue lock poisoned")
        })?.try_recv() {
            Ok(request) => request,
            Err(mpsc::TryRecvError::Empty) => return Ok(false),
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(AcError::conflict("DAEMON-IPC_QUEUE_CLOSED", "IPC request queue closed"))
            }
        };
        let (response, should_shutdown) = dispatch_request(&request.payload, daemon);
        let _ = request.response_tx.send(response);
        Ok(should_shutdown)
    }

    pub fn cleanup(&self) { let _ = std::fs::remove_file(&self.path); }

    fn spawn_client(&self, mut stream: UnixStream) -> AcResult<()> {
        let previous = self.active_clients.fetch_add(1, Ordering::SeqCst);
        if previous >= MAX_CLIENTS {
            self.active_clients.fetch_sub(1, Ordering::SeqCst);
            let _ = write_frame(
                &mut stream,
                &error_response("unknown", "DAEMON-IPC_CLIENT_LIMIT", "too many IPC clients".to_string()),
            );
            return Ok(());
        }
        let request_tx = self.request_tx.clone();
        let active_clients = Arc::clone(&self.active_clients);
        let daemon_euid = self.daemon_euid;
        let peer_credentials = Arc::clone(&self.peer_credentials);
        thread::Builder::new()
            .name("agentcode-ipc-client".to_string())
            .spawn(move || {
                let _guard = ActiveClientGuard(active_clients);
                let _ = serve_client(stream, daemon_euid, peer_credentials, request_tx);
            })
            .map_err(|error| AcError::validation("DAEMON-IPC_CLIENT_THREAD", error.to_string()))?;
        Ok(())
    }
}

struct ActiveClientGuard(Arc<AtomicUsize>);
impl Drop for ActiveClientGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

struct IpcDispatchRequest {
    payload: Value,
    response_tx: mpsc::SyncSender<Value>,
}

pub struct UnixIpcClient { path: PathBuf }
impl UnixIpcClient {
    pub fn new(path: impl Into<PathBuf>) -> Self { Self { path: path.into() } }
    pub fn request(&self, request: Value) -> AcResult<Value> {
        let mut stream = UnixStream::connect(&self.path)
            .map_err(|error| AcError::new("DAEMON-IPC_UNAVAILABLE", error.to_string(), ac_common::ErrorKind::Unavailable, ac_common::Retryability::Retryable))?;
        write_frame(&mut stream, &request)?;
        read_frame(&mut stream)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerCredential {
    pub peer_euid: u32,
    pub daemon_euid: u32,
}

trait PeerCredentialProvider: Send + Sync {
    fn peer_euid(&self, stream: &UnixStream) -> io::Result<u32>;
}

struct UnixPeerCredentialProvider;

impl PeerCredentialProvider for UnixPeerCredentialProvider {
    fn peer_euid(&self, stream: &UnixStream) -> io::Result<u32> {
        let cloned = stream.try_clone()?;
        cloned.set_nonblocking(true)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        let _guard = runtime.enter();
        let tokio_stream = tokio::net::UnixStream::from_std(cloned)?;
        Ok(tokio_stream.peer_cred()?.uid())
    }
}

pub fn authorize_peer_credential(credential: &PeerCredential) -> AcResult<()> {
    if credential.peer_euid == credential.daemon_euid {
        Ok(())
    } else {
        Err(AcError::policy_denied(
            "DAEMON-IPC_PEER_UID_DENIED",
            "IPC peer effective UID does not match daemon effective UID",
        ))
    }
}

fn serve_client(
    mut stream: UnixStream,
    daemon_euid: u32,
    peer_credentials: Arc<dyn PeerCredentialProvider>,
    request_tx: mpsc::SyncSender<IpcDispatchRequest>,
) -> AcResult<()> {
    stream
        .set_nonblocking(false)
        .map_err(|error| AcError::validation("DAEMON-IPC_CLIENT_CONFIG", error.to_string()))?;
    stream
        .set_read_timeout(Some(FRAME_TIMEOUT))
        .map_err(|error| AcError::validation("DAEMON-IPC_CLIENT_CONFIG", error.to_string()))?;
    stream
        .set_write_timeout(Some(FRAME_TIMEOUT))
        .map_err(|error| AcError::validation("DAEMON-IPC_CLIENT_CONFIG", error.to_string()))?;
    let peer_euid = match peer_credentials.peer_euid(&stream) {
        Ok(peer_euid) => peer_euid,
        Err(error) => {
            let _ = write_frame(&mut stream, &error_response("unknown", "DAEMON-IPC_PEER_CREDENTIAL", error.to_string()));
            return Ok(());
        }
    };
    if let Err(error) = authorize_peer_credential(&PeerCredential { peer_euid, daemon_euid }) {
        let _ = write_frame(&mut stream, &error_response("unknown", error.code(), error.to_string()));
        return Ok(());
    }
    let request = match read_frame(&mut stream) {
        Ok(request) => request,
        Err(error) => { let _ = write_frame(&mut stream, &error_response("unknown", error.code(), error.to_string())); return Ok(()); }
    };
    let (response_tx, response_rx) = mpsc::sync_channel(1);
    if request_tx.try_send(IpcDispatchRequest { payload: request, response_tx }).is_err() {
        let _ = write_frame(&mut stream, &error_response("unknown", "DAEMON-IPC_BACKPRESSURE", "daemon IPC request queue is full".to_string()));
        return Ok(());
    }
    if let Ok(response) = response_rx.recv_timeout(FRAME_TIMEOUT) {
        let _ = write_frame(&mut stream, &response);
    }
    Ok(())
}

fn dispatch_request(request: &Value, daemon: &mut DaemonService) -> (Value, bool) {
    let correlation_id = request.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let command = request.get("command").and_then(Value::as_str).unwrap_or("");
    let response = match command {
        "Ping" | "Health" | "GetDaemonInfo" => json!({"id": correlation_id, "ok": true, "protocol_version": IPC_PROTOCOL_VERSION, "lifecycle": format!("{:?}", daemon.health().lifecycle), "recovered_sessions": daemon.health().recovered_sessions}),
        "SubmitMission" => match request.get("goal").and_then(Value::as_str) {
            Some(goal) => match daemon.handle(DaemonCommand::CreateSession {
                goal: goal.to_string(),
                workspace_root: request
                    .get("workspace_root")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            }) {
                Ok(DaemonResponse::SessionCreated { mission_id, session_id }) => json!({"id": correlation_id, "ok": true, "mission_id": mission_id.to_string(), "session_id": session_id.to_string()}),
                Ok(_) => error_response(correlation_id, "DAEMON-IPC_PROTOCOL", "unexpected response".to_string()),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "SubmitMission requires goal".to_string()),
        },
        "ListActiveMissions" => match daemon.active_missions() {
            Ok(missions) => json!({"id": correlation_id, "ok": true, "missions": missions.into_iter().map(|(mission_id, state, task_count)| json!({"mission_id": mission_id, "state": state, "task_count": task_count})).collect::<Vec<_>>() }),
            Err(error) => error_response(correlation_id, error.code(), error.to_string()),
        },
        "GetMission" | "GetTaskState" => match request.get("mission_id").and_then(Value::as_str) {
            Some(mission_id) => match daemon.task_states(mission_id) {
                Ok(tasks) => {
                    let status = daemon.mission_status(mission_id);
                    let state = match &status {
                        Some(status) => Some(status.state.clone()),
                        None => daemon.persisted_mission_state(mission_id).ok().flatten(),
                    };
                    let goal = daemon.mission_goal(mission_id).ok().flatten();
                    let workspace_root = daemon.mission_workspace(mission_id).ok().flatten();
                    json!({"id": correlation_id, "ok": true, "mission_id": mission_id, "session_id": status.as_ref().map(|status| status.session_id.to_string()), "state": state, "goal": goal, "workspace_root": workspace_root, "tasks": tasks.into_iter().map(|(task_id, state)| json!({"task_id": task_id, "state": state})).collect::<Vec<_>>() })
                },
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "mission_id is required".to_string()),
        },
        "GetMissionDetails" | "GetTaskDetails" | "GetMissionEvents" | "GetChangeSetSummary"
        | "GetEvidenceSummary" | "GetVerificationSummary" => {
            match request.get("mission_id").and_then(Value::as_str) {
                Some(mission_id) => {
                    let result = match command {
                        "GetMissionDetails" => daemon.mission_details(mission_id),
                        "GetTaskDetails" => daemon.task_details(mission_id),
                        "GetMissionEvents" => daemon.mission_events(
                            mission_id,
                            request.get("limit").and_then(Value::as_u64).map(|n| n as usize),
                        ),
                        "GetChangeSetSummary" => daemon.changeset_summary(mission_id),
                        "GetEvidenceSummary" => daemon.evidence_summary(mission_id),
                        _ => daemon.verification_summary(mission_id),
                    };
                    match result {
                        Ok(mut payload) => {
                            payload["id"] = json!(correlation_id);
                            payload["ok"] = json!(true);
                            payload["mission_id"] = json!(mission_id);
                            payload
                        }
                        Err(error) => error_response(correlation_id, error.code(), error.to_string()),
                    }
                }
                None => error_response(
                    correlation_id,
                    "DAEMON-IPC_INVALID",
                    "mission_id is required".to_string(),
                ),
            }
        }
        "PauseMission" | "ResumeMission" | "CancelMission" => match request.get("mission_id").and_then(Value::as_str) {
            Some(mission_id) => {
                let result = match command { "PauseMission" => daemon.pause_mission(mission_id), "ResumeMission" => daemon.resume_mission(mission_id), _ => daemon.cancel_mission(mission_id) };
                match result { Ok(()) => json!({"id": correlation_id, "ok": true, "mission_id": mission_id, "state": daemon.mission_status(mission_id).map(|status| status.state)}), Err(error) => error_response(correlation_id, error.code(), error.to_string()) }
            }
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "mission_id is required".to_string()),
        },
        "GetRecentEvents" | "GetBlockedReason" => error_response(correlation_id, "DAEMON-IPC_UNSUPPORTED", "command is not available in macOS V1 runtime".to_string()),
        "ListProviders" => match daemon.provider_catalog() {
            Ok(providers) => json!({"id": correlation_id, "ok": true, "providers": providers.into_iter().map(provider_catalog_json).collect::<Vec<_>>()}),
            Err(error) => error_response(correlation_id, error.code(), error.to_string()),
        },
        "ListProviderAccounts" => match request.get("provider_id").and_then(Value::as_str) {
            Some(provider_id) => match daemon.list_provider_accounts(provider_id) {
                Ok(accounts) => json!({"id": correlation_id, "ok": true, "provider_id": provider_id, "accounts": accounts.into_iter().map(|account| provider_account_json(&account, true)).collect::<Vec<_>>()}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "provider_id is required".to_string()),
        },
        "GetProviderAccount" => match request.get("account_id").and_then(Value::as_str) {
            Some(account_id) => match daemon.provider_account(account_id) {
                Ok(Some(account)) => json!({"id": correlation_id, "ok": true, "account": provider_account_json(&account, true)}),
                Ok(None) => error_response(correlation_id, "DAEMON-PROVIDER_ACCOUNT_NOT_FOUND", "provider account not found".to_string()),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "account_id is required".to_string()),
        },
        "CreateProviderAccount" => {
            let provider_id = request.get("provider_id").and_then(Value::as_str).unwrap_or("");
            let label = request.get("label").and_then(Value::as_str).unwrap_or("");
            let credential_ref = request.get("credential_ref").and_then(Value::as_str).unwrap_or("");
            let credential_region = request.get("credential_region").and_then(Value::as_str).unwrap_or("");
            let organization = request.get("organization").and_then(Value::as_str).unwrap_or("");
            let project = request.get("project").and_then(Value::as_str).unwrap_or("");
            let workspace = request.get("workspace").and_then(Value::as_str).unwrap_or("");
            let enabled = request.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            match daemon.save_provider_account(provider_id, label, credential_ref, credential_region, organization, project, workspace, enabled) {
                Ok(account_id) => json!({"id": correlation_id, "ok": true, "account_id": account_id.to_string()}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "UpdateProviderAccount" => match request.get("account_id").and_then(Value::as_str) {
            Some(account_id) => {
                let label = request.get("label").and_then(Value::as_str).unwrap_or("");
                let credential_ref = request.get("credential_ref").and_then(Value::as_str).unwrap_or("");
                let credential_region = request.get("credential_region").and_then(Value::as_str).unwrap_or("");
                let organization = request.get("organization").and_then(Value::as_str).unwrap_or("");
                let project = request.get("project").and_then(Value::as_str).unwrap_or("");
                let workspace = request.get("workspace").and_then(Value::as_str).unwrap_or("");
                let enabled = request.get("enabled").and_then(Value::as_bool).unwrap_or(true);
                match daemon.update_provider_account(account_id, label, credential_ref, credential_region, organization, project, workspace, enabled) {
                    Ok(()) => json!({"id": correlation_id, "ok": true, "account_id": account_id}),
                    Err(error) => error_response(correlation_id, error.code(), error.to_string()),
                }
            }
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "account_id is required".to_string()),
        },
        "DeleteProviderAccount" => match request.get("account_id").and_then(Value::as_str) {
            Some(account_id) => match daemon.delete_provider_account(account_id) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "account_id": account_id}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "account_id is required".to_string()),
        },
        "SetProviderAccountEnabled" => {
            let account_id = request.get("account_id").and_then(Value::as_str).unwrap_or("");
            let enabled = request.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            match daemon.set_provider_account_enabled(account_id, enabled) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "account_id": account_id}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "RotateProviderAccount" => {
            let account_id = request.get("account_id").and_then(Value::as_str).unwrap_or("");
            let credential_ref = request.get("credential_ref").and_then(Value::as_str).unwrap_or("");
            match daemon.rotate_provider_account(account_id, credential_ref) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "account_id": account_id}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "TestProviderAccount" => match request.get("account_id").and_then(Value::as_str) {
            Some(account_id) => match daemon.test_provider_account(account_id) {
                Ok(status) => json!({"id": correlation_id, "ok": true, "account_id": status.account_id.to_string(), "provider_id": status.provider_id.to_string(), "healthy": status.ok, "health_state": status.health_state, "latency_ms": status.latency_ms, "failure": status.failure.map(|failure| format!("{failure:?}")), "masked_credential": status.masked_credential, "detail": status.detail}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "account_id is required".to_string()),
        },
        "DiscoverProviderModels" => {
            let endpoint = request.get("endpoint").and_then(Value::as_str).unwrap_or("");
            let kind = request.get("kind").and_then(Value::as_str).unwrap_or("ollama");
            match daemon.discover_provider_models(endpoint, kind) {
                Ok(models) => json!({"id": correlation_id, "ok": true, "models": models.into_iter().map(|model| json!({"model_name": model.model_name, "capabilities": model.capabilities, "context_window": model.context_window, "parameters": model.parameters})).collect::<Vec<_>>()}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "StoreCredential" => {
            let name = request.get("name").and_then(Value::as_str).unwrap_or("");
            let value = request.get("value").and_then(Value::as_str).unwrap_or("");
            match daemon.store_credential(name, value) {
                Ok(credential_ref) => json!({"id": correlation_id, "ok": true, "credential_ref": credential_ref}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DeleteCredential" => {
            let name = request.get("name").and_then(Value::as_str).unwrap_or("");
            match daemon.delete_credential(name) {
                Ok(()) => json!({"id": correlation_id, "ok": true}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "TestCredential" => {
            let provider_id = request.get("provider_id").and_then(Value::as_str).unwrap_or("");
            let credential_ref = request.get("credential_ref").and_then(Value::as_str).unwrap_or("");
            let organization = request.get("organization").and_then(Value::as_str).unwrap_or("");
            let project = request.get("project").and_then(Value::as_str).unwrap_or("");
            let workspace = request.get("workspace").and_then(Value::as_str).unwrap_or("");
            match daemon.test_credential(provider_id, credential_ref, organization, project, workspace) {
                Ok(status) => json!({"id": correlation_id, "ok": true, "provider_id": status.provider_id.to_string(), "healthy": status.ok, "health_state": status.health_state, "latency_ms": status.latency_ms, "failure": status.failure.map(|failure| format!("{failure:?}")), "masked_credential": status.masked_credential, "detail": status.detail}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "GetDesktopSettings" => match daemon.desktop_settings() {
            Ok(preferences) => json!({"id": correlation_id, "ok": true, "appearance": preferences.appearance, "notifications_enabled": preferences.notifications_enabled, "completion_sound_enabled": preferences.completion_sound_enabled, "reduced_motion": preferences.reduced_motion, "budget_limit_micros": preferences.budget_limit_micros}),
            Err(error) => error_response(correlation_id, error.code(), error.to_string()),
        },
        "SetDesktopSettings" => {
            let appearance = request.get("appearance").and_then(Value::as_str).unwrap_or("light").to_string();
            let notifications = request.get("notifications_enabled").and_then(Value::as_bool).unwrap_or(true);
            let completion_sound = request.get("completion_sound_enabled").and_then(Value::as_bool).unwrap_or(true);
            let reduced_motion = request.get("reduced_motion").and_then(Value::as_bool).unwrap_or(false);
            let budget = request.get("budget_limit_micros").and_then(Value::as_u64);
            match daemon.save_desktop_settings(appearance, notifications, completion_sound, reduced_motion, budget) {
                Ok(()) => json!({"id": correlation_id, "ok": true}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "ShutdownDaemon" => { let result = daemon.handle(DaemonCommand::Stop); match result { Ok(_) => json!({"id": correlation_id, "ok": true}), Err(error) => error_response(correlation_id, error.code(), error.to_string()) } }
        _ => error_response(correlation_id, "DAEMON-IPC_UNKNOWN_COMMAND", "unknown IPC command".to_string()),
    };
    let should_shutdown = command == "ShutdownDaemon" && response.get("ok") == Some(&Value::Bool(true));
    (response, should_shutdown)
}

fn error_response(id: &str, code: &str, message: String) -> Value { json!({"id": id, "ok": false, "error": {"code": code, "message": message}}) }

/// Backend-owned provider catalog JSON.  Contains metadata only — never a
/// credential value.
fn provider_catalog_json(entry: ac_provider::ProviderCatalogEntry) -> Value {
    json!({
        "id": entry.id.to_string(),
        "display_name": entry.display_name,
        "description": entry.description,
        "website_url": entry.website_url,
        "logo_url": entry.logo_url,
        "credential_url": entry.credential_url,
        "pricing_classification": entry.pricing_classification,
        "capabilities": entry.capabilities,
    })
}

/// Provider account JSON.  `mask` is always true in IPC responses: the raw
/// credential reference is replaced with a masked form so that no credential
/// value or secret name can leak into UI state.
fn provider_account_json(account: &ac_provider::catalog::ProviderAccount, mask: bool) -> Value {
    let credential = if mask {
        account.masked_credential()
    } else {
        account.credential_ref.clone()
    };
    json!({
        "id": account.id.to_string(),
        "provider_id": account.provider_id.to_string(),
        "label": account.label,
        "credential_ref": credential,
        "credential_region": account.credential_region,
        "organization": account.organization,
        "project": account.project,
        "workspace": account.workspace,
        "enabled": account.enabled,
        "health_state": account.health_state,
        "quota_rate_limit": account.quota_rate_limit,
        "quota_remaining": account.quota_remaining,
        "quota_reset_at_ms": account.quota_reset_at_ms,
        "last_success_at_ms": account.last_success_at_ms,
        "last_failure_at_ms": account.last_failure_at_ms,
        "failure_reason": account.failure_reason,
        "credential_masked": mask,
    })
}

fn write_frame(stream: &mut UnixStream, value: &Value) -> AcResult<()> { let body = serde_json::to_vec(value).map_err(|error| AcError::validation("DAEMON-IPC_SERIALIZE", error.to_string()))?; if body.len() > MAX_FRAME_BYTES { return Err(AcError::validation("DAEMON-IPC_FRAME_TOO_LARGE", "IPC message exceeds limit")); } stream.write_all(&(body.len() as u32).to_be_bytes()).and_then(|_| stream.write_all(&body)).map_err(|error| AcError::validation("DAEMON-IPC_WRITE", error.to_string())) }
fn read_frame(stream: &mut UnixStream) -> AcResult<Value> { let mut size = [0; 4]; stream.read_exact(&mut size).map_err(|error| AcError::validation("DAEMON-IPC_FRAME", error.to_string()))?; let size = u32::from_be_bytes(size) as usize; if size == 0 || size > MAX_FRAME_BYTES { return Err(AcError::validation("DAEMON-IPC_FRAME_TOO_LARGE", "invalid IPC frame size")); } let mut body = vec![0; size]; stream.read_exact(&mut body).map_err(|error| AcError::validation("DAEMON-IPC_FRAME", error.to_string()))?; serde_json::from_slice(&body).map_err(|error| AcError::validation("DAEMON-IPC_JSON", error.to_string())) }

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicU32;
    use std::time::{Duration, Instant};

    #[derive(Default)]
    struct FixedPeerCredentials {
        uid: AtomicU32,
    }

    impl FixedPeerCredentials {
        fn new(uid: u32) -> Self {
            Self { uid: AtomicU32::new(uid) }
        }
    }

    impl PeerCredentialProvider for FixedPeerCredentials {
        fn peer_euid(&self, _stream: &UnixStream) -> io::Result<u32> {
            Ok(self.uid.load(Ordering::SeqCst))
        }
    }

    fn temp_paths(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let dir = PathBuf::from(format!(
            "/tmp/acipc-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let (db, lock) = default_paths(&dir);
        let socket = default_socket_path(&dir);
        (dir, db, lock, socket)
    }

    fn bind_or_skip(
        socket: &Path,
        provider: Option<Arc<dyn PeerCredentialProvider>>,
    ) -> Option<(UnixIpcServer, UnixListener)> {
        let result = if let Some(provider) = provider {
            UnixIpcServer::bind_with_peer_credentials(socket, provider)
        } else {
            UnixIpcServer::bind(socket)
        };
        match result {
            Ok(bound) => Some(bound),
            Err(error) if error.code() == "DAEMON-IPC_BIND" && error.to_string().contains("Operation not permitted") => {
                eprintln!("Unix socket bind is environment-blocked; skipping IPC socket proof");
                None
            }
            Err(error) => panic!("unexpected IPC bind failure: {error:?}"),
        }
    }

    #[test]
    fn peer_authorization_accepts_same_uid_and_rejects_different_uid() {
        assert!(authorize_peer_credential(&PeerCredential { peer_euid: 501, daemon_euid: 501 }).is_ok());
        let err = authorize_peer_credential(&PeerCredential { peer_euid: 502, daemon_euid: 501 }).unwrap_err();
        assert_eq!(err.code(), "DAEMON-IPC_PEER_UID_DENIED");
    }

    #[test]
    fn wrong_uid_is_rejected_before_command_dispatch() {
        let (dir, db, lock, socket) = temp_paths("wrong-uid");
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let provider = Arc::new(FixedPeerCredentials::new(12345));
        let Some((server, listener)) = bind_or_skip(&socket, Some(provider)) else {
            daemon.stop().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let response = std::thread::spawn({
            let socket = socket.clone();
            move || UnixIpcClient::new(socket).request(json!({"id":"bad","command":"SubmitMission","goal":"must not dispatch"})).unwrap()
        });
        // Pump the server loop until the client has completed (or a generous
        // deadline elapses).  Under full-suite parallel CPU contention the
        // single-threaded pump can be starved, so a fixed short budget is
        // unreliable; the assertion below is unchanged and remains strict.
        let start = Instant::now();
        while !response.is_finished() && start.elapsed() < Duration::from_secs(15) {
            server.serve_once(&listener, &mut daemon).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        let response = response.join().unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "DAEMON-IPC_PEER_UID_DENIED");
        assert!(daemon.active_missions().unwrap().is_empty());
        server.cleanup();
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }
    #[test]
    fn provider_account_ipc_responses_never_expose_raw_secret() {
        // Full IPC round trip: StoreCredential + CreateProviderAccount + list
        // and get account responses must never contain the raw secret value or
        // the unmasked reference.  This proves the no-leak requirement at the
        // exact boundary React consumes.
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        let (dir, db, lock, socket) = temp_paths("secret-leak");
        std::env::set_var("AGENTCODE_SECRET_DIR", dir.join("secrets"));
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.stop().unwrap();
            std::env::remove_var("AGENTCODE_SECRET_DIR");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let store = std::thread::spawn({
            let socket = socket.clone();
            move || {
                UnixIpcClient::new(socket)
                    .request(json!({
                        "id": "store",
                        "command": "StoreCredential",
                        "name": "ui-saved-key",
                        "value": "sk-ui-saved-RAW-SECRET"
                    }))
                    .unwrap()
            }
        });
        pump_until(&server, &listener, &mut daemon, &store);
        let store = store.join().unwrap();
        assert_eq!(store["ok"], true, "store response: {store}");
        let credential_ref = store["credential_ref"].as_str().unwrap().to_string();
        assert_eq!(credential_ref, "secret:ui-saved-key");

        let create = std::thread::spawn({
            let socket = socket.clone();
            let credential_ref = credential_ref.clone();
            move || {
                UnixIpcClient::new(socket)
                    .request(json!({
                        "id": "create",
                        "command": "CreateProviderAccount",
                        "provider_id": "openai",
                        "label": "ui-account",
                        "credential_ref": credential_ref
                    }))
                    .unwrap()
            }
        });
        pump_until(&server, &listener, &mut daemon, &create);
        let create = create.join().unwrap();
        assert_eq!(create["ok"], true, "create response: {create}");
        let account_id = create["account_id"].as_str().unwrap().to_string();

        let list = std::thread::spawn({
            let socket = socket.clone();
            move || {
                UnixIpcClient::new(socket)
                    .request(json!({"id": "list", "command": "ListProviderAccounts", "provider_id": "openai"}))
                    .unwrap()
            }
        });
        pump_until(&server, &listener, &mut daemon, &list);
        let list = list.join().unwrap();
        let get = std::thread::spawn({
            let socket = socket.clone();
            let account_id = account_id.clone();
            move || {
                UnixIpcClient::new(socket)
                    .request(json!({"id": "get", "command": "GetProviderAccount", "account_id": account_id}))
                    .unwrap()
            }
        });
        pump_until(&server, &listener, &mut daemon, &get);
        let get = get.join().unwrap();

        // The raw secret value must appear nowhere in any IPC response.
        let serialized = format!("{store}{create}{list}{get}");
        assert!(
            !serialized.contains("sk-ui-saved-RAW-SECRET"),
            "raw secret leaked into IPC responses: {serialized}"
        );
        // Account list/get responses must carry only the masked form, never
        // the unmasked `secret:NAME` reference.  (The StoreCredential response
        // itself returns the reference because the UI needs that handle to
        // create the account; that is the documented contract.)
        assert!(
            !format!("{list}{get}").contains("secret:ui-saved-key"),
            "unmasked credential reference leaked into account responses: {serialized}"
        );
        assert!(serialized.contains("****"), "masked credential must be present");
        // Account is durable and reachable after restart with the same ref.
        drop(server);
        daemon.stop().unwrap();
        let daemon = DaemonService::open(&db, &lock).unwrap();
        let accounts = daemon.list_provider_accounts("openai").unwrap();
        assert_eq!(accounts[0].credential_ref, "secret:ui-saved-key");

        std::env::remove_var("AGENTCODE_SECRET_DIR");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stalled_client_does_not_block_healthy_client() {
        let (dir, db, lock, socket) = temp_paths("concurrency");
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.stop().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let mut stalled = UnixStream::connect(&socket).unwrap();
        stalled.write_all(&100u32.to_be_bytes()).unwrap();

        for _ in 0..10 {
            server.serve_once(&listener, &mut daemon).unwrap();
        }

        let healthy = std::thread::spawn({
            let socket = socket.clone();
            move || UnixIpcClient::new(socket).request(json!({"id":"ok","command":"Health"})).unwrap()
        });
        let start = Instant::now();
        while !healthy.is_finished() && start.elapsed() < Duration::from_millis(5000) {
            server.serve_once(&listener, &mut daemon).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(healthy.is_finished(), "healthy client waited behind stalled client");
        let response = healthy.join().unwrap();
        assert_eq!(response["ok"], true);
        drop(stalled);
        server.cleanup();
        daemon.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    fn pump_until(
        server: &UnixIpcServer,
        listener: &UnixListener,
        daemon: &mut DaemonService,
        thread: &std::thread::JoinHandle<Value>,
    ) {
        let start = Instant::now();
        while !thread.is_finished() && start.elapsed() < Duration::from_secs(15) {
            server.serve_once(listener, daemon).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            thread.is_finished(),
            "IPC request did not complete within the pump budget"
        );
    }

    /// Seed a deterministic mission with real persisted rows so the
    /// observability IPC commands can be exercised against authoritative
    /// SQLite data through the real daemon + Unix socket boundary.
    struct SeededMission {
        mission_id: String,
        evidence_ids: Vec<String>,
    }

    fn seed_observability_mission(db: &ControlPlaneDb, completed: bool) -> SeededMission {
        use ac_changeset::{
            EditEngine, EditPrecondition, EditRequest, EditStrategy, MemoryFileRepository,
        };
        use ac_evidence::{EvidenceKind, EvidenceStore, Provenance};
        use ac_kernel::{AllowAllPolicy, Kernel, MissionState};
        use ac_verification::{
            FinalAuditInput, GateStatus, ProjectCapabilities, VerificationEngine,
            VerificationRisk,
        };

        let mut kernel = Kernel::new(AllowAllPolicy);
        kernel.start().unwrap();
        let mission_id = kernel
            .create_mission("observability test mission")
            .unwrap();
        kernel
            .transition_mission(&mission_id, MissionState::Active, Vec::new())
            .unwrap();
        if completed {
            kernel
                .transition_mission(&mission_id, MissionState::Completed, Vec::new())
                .unwrap();
        }
        db.put_mission(kernel.mission(&mission_id).unwrap()).unwrap();
        for event in kernel.events() {
            db.append_kernel_event(event).unwrap();
        }

        let session_id = StableId::new("session");
        db.save_session(
            &session_id,
            &mission_id,
            if completed { "completed" } else { "running" },
            Some("/tmp/observability-workspace"),
        )
        .unwrap();

        let worker = ac_db::WorkerRecord {
            id: StableId::new("worker").to_string(),
            mission_id: mission_id.to_string(),
            session_id: session_id.to_string(),
            state: "running".to_string(),
            workspace_ref: Some("/tmp/observability-workspace".to_string()),
            updated_at_ms: 1_700_000_000_000,
        };
        let task_a = ac_db::TaskRecord {
            id: StableId::new("task-a").to_string(),
            mission_id: mission_id.to_string(),
            title: "Implement the feature".to_string(),
            state: if completed { "completed" } else { "running" }.to_string(),
            dependencies_json: String::new(),
            assigned_worker_id: Some(worker.id.clone()),
            retry_count: 1,
            max_retries: 3,
            updated_at_ms: 1_700_000_001_000,
            acceptance_criteria_json: r#"[{"id":"ac-1","description":"works","required":true}]"#
                .to_string(),
        };
        let task_b = ac_db::TaskRecord {
            id: StableId::new("task-b").to_string(),
            mission_id: mission_id.to_string(),
            title: "Verify the feature".to_string(),
            state: if completed { "completed" } else { "ready" }.to_string(),
            dependencies_json: task_a.id.clone(),
            assigned_worker_id: Some(worker.id.clone()),
            retry_count: 0,
            max_retries: 3,
            updated_at_ms: 1_700_000_002_000,
            acceptance_criteria_json: "[]".to_string(),
        };

        let mut evidence_store = EvidenceStore::new();
        let ev_a = evidence_store
            .append(
                EvidenceKind::CommandOutput,
                Provenance {
                    source: "agent.observation".to_string(),
                    commit: Some("abc".to_string()),
                    worktree: Some("/tmp/observability-workspace".to_string()),
                    tool: Some("fs.read".to_string()),
                },
                "mem://observability/a",
                "fnv1a64:abc123",
            )
            .unwrap();
        let ev_b = evidence_store
            .append(
                EvidenceKind::TestReport,
                Provenance {
                    source: "dev.test".to_string(),
                    commit: Some("abc".to_string()),
                    worktree: Some("/tmp/observability-workspace".to_string()),
                    tool: Some("cargo".to_string()),
                },
                "mem://observability/b",
                "fnv1a64:def456",
            )
            .unwrap();
        // A sensitive record with a redacted summary — its raw content must
        // never appear in IPC responses.
        let ev_sensitive = evidence_store
            .append_tool_output(
                Provenance {
                    source: "tool.execution".to_string(),
                    commit: Some("abc".to_string()),
                    worktree: Some("/tmp/observability-workspace".to_string()),
                    tool: Some("run".to_string()),
                },
                "mem://observability/sensitive",
                "token=sk-observability-secret\noutput line",
                &["sk-observability-secret".to_string()],
            )
            .unwrap();
        let evidence_ids = vec![
            ev_a.to_string(),
            ev_b.to_string(),
            ev_sensitive.to_string(),
        ];

        let attempts = vec![ac_db::TaskAttemptRecord {
            id: StableId::new("attempt-1").to_string(),
            task_id: task_a.id.clone(),
            worker_id: worker.id.clone(),
            outcome: "succeeded".to_string(),
            evidence_refs: format!("{ev_a},{ev_b},{ev_sensitive}"),
            failure_class: None,
            created_at_ms: 1_700_000_003_000,
        }];
        db.persist_graph_atomic(&worker, &[task_a.clone(), task_b.clone()], &attempts)
            .unwrap();
        for record in evidence_store.records() {
            db.append_evidence(record).unwrap();
        }

        // ChangeSet through the authoritative edit-engine path.
        let mut repo = MemoryFileRepository::new("rev-obs");
        repo.put("src/lib.rs", "pub fn answer() -> u32 { 41 }\n");
        let request = EditRequest {
            path: "src/lib.rs".to_string(),
            precondition: EditPrecondition {
                path: "src/lib.rs".to_string(),
                expected_hash: Some(repo.hash("src/lib.rs").unwrap()),
                base_revision: "rev-obs".to_string(),
                symbol_fingerprint: None,
            },
            strategy: EditStrategy::SearchReplace {
                search: "41".to_string(),
                replace: "42".to_string(),
                expected_matches: 1,
            },
        };
        let mut transaction = EditEngine.prepare(&repo, vec![request]).unwrap();
        EditEngine.apply(&mut repo, &mut transaction).unwrap();
        // Persist both the changeset row (changesets table) and its edit
        // transaction (edit_transactions/edit_operations), mirroring the real
        // durability path so the observability join can find it.
        db.save_changeset(&transaction.changeset).unwrap();
        db.save_changeset_transaction(
            &transaction,
            Some(task_a.id.as_str()),
            None,
            "rev-obs",
            "rust",
        )
        .unwrap();

        // Verification run + final audit.
        let engine = VerificationEngine::new(ac_security::CapabilityPolicy::new());
        let profile = engine.derive_profile(
            StableId::new("task"),
            VerificationRisk::High,
            &ProjectCapabilities {
                cargo: true,
                makefile: false,
                package_json: false,
                python: false,
                go: false,
                browser: false,
                security: false,
            },
        );
        let mut manifest_evidence = EvidenceStore::new();
        let requirement_id = StableId::new("req");
        let manifest = engine
            .record_evidence_manifest(
                "commit-obs",
                "/tmp/observability-workspace",
                "cargo test --workspace",
                GateStatus::Passed,
                vec![requirement_id.clone()],
                vec!["src/lib.rs".to_string()],
                &mut manifest_evidence,
            )
            .unwrap();
        db.save_verification_manifest(
            &manifest,
            Some(profile.id.as_str()),
            Some(task_b.id.as_str()),
        )
        .unwrap();
        if completed {
            let audit = engine
                .final_audit(
                    FinalAuditInput {
                        original_goal: "observability test mission".to_string(),
                        requirements: vec!["feature works".to_string()],
                        required_requirement_ids: vec![requirement_id.clone()],
                        verified_requirement_ids: vec![requirement_id.clone()],
                        evidence_refs: vec![manifest.evidence_ref.clone()],
                        worker_completion_text: "done".to_string(),
                        unresolved_limitations: Vec::new(),
                    },
                    &mut manifest_evidence,
                )
                .unwrap();
            db.save_final_audit(
                mission_id.as_str(),
                "observability test mission",
                &["feature works".to_string()],
                &audit,
                true,
            )
            .unwrap();
        }

        SeededMission {
            mission_id: mission_id.to_string(),
            evidence_ids,
        }
    }

    fn request_via_ipc(
        server: &UnixIpcServer,
        listener: &UnixListener,
        daemon: &mut DaemonService,
        payload: Value,
    ) -> Value {
        let client = std::thread::spawn({
            let socket = server.path().to_path_buf();
            move || UnixIpcClient::new(socket).request(payload).unwrap()
        });
        pump_until(server, listener, daemon, &client);
        client.join().unwrap()
    }

    #[test]
    fn observability_mission_details_surfaces_full_snapshot() {
        let (dir, db_path, lock, socket) = temp_paths("obs-details");
        let seeded_mission_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, false);
            seeded_mission_id = seeded.mission_id;
        }
        // Daemon open without start() to avoid coordinator re-execution
        // of the non-terminal mission.
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let response = request_via_ipc(
            &server,
            &listener,
            &mut daemon,
            json!({"id":"obs","command":"GetMissionDetails","mission_id": seeded_mission_id}),
        );
        assert_eq!(response["ok"], true, "details response: {response}");
        assert_eq!(response["mission_id"], seeded_mission_id);
        assert_eq!(response["state"], "running");
        assert_eq!(response["goal"], "observability test mission");
        assert_eq!(response["workspace_root"], "/tmp/observability-workspace");
        assert_eq!(response["task_count"], 2);
        assert_eq!(response["completed_task_count"], 0);
        let current_task = response["current_task"].as_str().unwrap();
        assert!(
            current_task.starts_with("task-a-"),
            "current_task should be the running task, got {current_task}"
        );
        assert!(response["created_at_ms"].is_u64() || response["created_at_ms"].is_i64());
        assert!(response["updated_at_ms"].is_u64() || response["updated_at_ms"].is_i64());
        assert_eq!(response["terminal"], false);
        assert!(response["progress"].is_number());
        assert!(response["state_counts"].is_object());
        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn observability_task_details_include_attempts_and_dependencies() {
        let (dir, db_path, lock, socket) = temp_paths("obs-tasks");
        let seeded_mission_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, false);
            seeded_mission_id = seeded.mission_id;
        }
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let response = request_via_ipc(
            &server,
            &listener,
            &mut daemon,
            json!({"id":"obs","command":"GetTaskDetails","mission_id": seeded_mission_id}),
        );
        assert_eq!(response["ok"], true, "tasks response: {response}");
        let tasks = response["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        let task_a = tasks
            .iter()
            .find(|task| task["title"] == "Implement the feature")
            .unwrap();
        assert_eq!(task_a["state"], "running");
        assert_eq!(task_a["retry_count"], 1);
        assert_eq!(task_a["max_retries"], 3);
        assert_eq!(task_a["attempts"].as_array().unwrap().len(), 1);
        assert_eq!(task_a["attempts"][0]["outcome"], "succeeded");
        let task_b = tasks
            .iter()
            .find(|task| task["title"] == "Verify the feature")
            .unwrap();
        assert_eq!(task_b["dependencies"].as_array().unwrap().len(), 1);
        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn observability_activity_events_are_bounded_and_real() {
        let (dir, db_path, lock, socket) = temp_paths("obs-events");
        let seeded_mission_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, true);
            seeded_mission_id = seeded.mission_id;
        }
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let response = request_via_ipc(
            &server,
            &listener,
            &mut daemon,
            json!({"id":"obs","command":"GetMissionEvents","mission_id": seeded_mission_id, "limit": 3}),
        );
        assert_eq!(response["ok"], true, "events response: {response}");
        let events = response["events"].as_array().unwrap();
        assert!(events.len() <= 3, "events must be bounded, got {events:?}");
        assert!(!events.is_empty(), "seeded mission must produce events");
        for event in events {
            assert!(event["kind"].as_str().is_some(), "event: {event}");
            assert!(event["created_at_ms"].is_u64() || event["created_at_ms"].is_i64());
        }
        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn observability_changeset_summary_lists_files_read_only() {
        let (dir, db_path, lock, socket) = temp_paths("obs-changeset");
        let seeded_mission_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, true);
            seeded_mission_id = seeded.mission_id;
        }
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let response = request_via_ipc(
            &server,
            &listener,
            &mut daemon,
            json!({"id":"obs","command":"GetChangeSetSummary","mission_id": seeded_mission_id}),
        );
        assert_eq!(response["ok"], true, "changeset response: {response}");
        let changesets = response["changesets"].as_array().unwrap();
        assert!(!changesets.is_empty());
        let first = &changesets[0];
        assert!(first["changeset_id"].as_str().is_some());
        assert!(first["state"].as_str().is_some());
        let files = first["files"].as_array().unwrap();
        assert!(!files.is_empty(), "changeset files must be exposed");
        assert_eq!(files[0]["path"], "src/lib.rs");
        assert_eq!(files[0]["strategy"], "SearchReplace");
        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn observability_evidence_summary_never_leaks_raw_or_secret_content() {
        let (dir, db_path, lock, socket) = temp_paths("obs-evidence");
        let seeded_mission_id;
        let seeded_evidence;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, false);
            seeded_mission_id = seeded.mission_id;
            seeded_evidence = seeded.evidence_ids;
        }
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let response = request_via_ipc(
            &server,
            &listener,
            &mut daemon,
            json!({"id":"obs","command":"GetEvidenceSummary","mission_id": seeded_mission_id}),
        );
        assert_eq!(response["ok"], true, "evidence response: {response}");
        let evidence = response["evidence"].as_array().unwrap();
        assert!(!evidence.is_empty());
        let serialized = format!("{response}");
        // The raw secret value must never appear anywhere in the response.
        assert!(
            !serialized.contains("sk-observability-secret"),
            "raw secret leaked into evidence summary: {serialized}"
        );
        // A redacted summary is expected and safe: it must contain the
        // redaction marker, proving the evidence layer's redaction ran.
        assert!(
            serialized.contains("[REDACTED]"),
            "sensitive evidence summary must be redacted: {serialized}"
        );
        for item in evidence {
            assert!(item["evidence_id"].as_str().is_some());
            assert!(item["kind"].as_str().is_some());
            assert!(item["content_hash"].as_str().is_some());
            assert!(item["sensitive"].is_boolean());
            assert!(item.get("raw_content").is_none(), "raw_content exposed: {item}");
        }
        for id in &seeded_evidence {
            assert!(
                evidence
                    .iter()
                    .any(|item| item["evidence_id"] == json!(id)),
                "evidence {id} missing from summary: {response}"
            );
        }
        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn observability_verification_summary_returns_results_without_raw_artifact() {
        let (dir, db_path, lock, socket) = temp_paths("obs-verification");
        let seeded_mission_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, true);
            seeded_mission_id = seeded.mission_id;
        }
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let response = request_via_ipc(
            &server,
            &listener,
            &mut daemon,
            json!({"id":"obs","command":"GetVerificationSummary","mission_id": seeded_mission_id}),
        );
        assert_eq!(response["ok"], true, "verification response: {response}");
        let verifications = response["verifications"].as_array().unwrap();
        assert!(!verifications.is_empty());
        for run in verifications {
            assert!(run["verification_id"].as_str().is_some());
            assert!(run["status"].as_str().is_some());
            assert!(run["passed"].is_boolean());
            assert!(run.get("raw_artifact").is_none(), "raw_artifact exposed: {run}");
            assert!(run["command"].as_str().is_some());
        }
        let audits = response["final_audits"].as_array().unwrap();
        assert!(!audits.is_empty());
        assert_eq!(audits[0]["completion_allowed"], true);
        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn observability_invalid_mission_id_returns_typed_error() {
        let (dir, db_path, lock, socket) = temp_paths("obs-invalid");
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        for command in [
            "GetMissionDetails",
            "GetTaskDetails",
            "GetMissionEvents",
            "GetChangeSetSummary",
            "GetEvidenceSummary",
            "GetVerificationSummary",
        ] {
            let response = request_via_ipc(
                &server,
                &listener,
                &mut daemon,
                json!({"id":"obs","command": command, "mission_id": "mission-does-not-exist"}),
            );
            assert_eq!(response["ok"], false, "{command} must reject unknown mission: {response}");
            assert_eq!(
                response["error"]["code"], "DAEMON-MISSION_NOT_FOUND",
                "{command} error: {response}"
            );
        }
        let response = request_via_ipc(
            &server,
            &listener,
            &mut daemon,
            json!({"id":"obs","command":"GetMissionDetails"}),
        );
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "DAEMON-IPC_INVALID");
        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn observability_completed_mission_remains_completed_across_restart() {
        let (dir, db_path, lock, socket) = temp_paths("obs-restart");
        let seeded_mission_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, true);
            seeded_mission_id = seeded.mission_id;
        }
        // First cycle: open + start (safe because session is terminal).
        {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            let Some((server, listener)) = bind_or_skip(&socket, None) else {
                daemon.stop().unwrap();
                let _ = fs::remove_dir_all(dir);
                return;
            };
            let response = request_via_ipc(
                &server,
                &listener,
                &mut daemon,
                json!({"id":"obs","command":"GetMissionDetails","mission_id": seeded_mission_id}),
            );
            assert_eq!(response["state"], "completed");
            assert_eq!(response["terminal"], true);
            assert_eq!(response["completion"]["completion_allowed"], true);
            server.cleanup();
            daemon.stop().unwrap();
        }
        // Second cycle: restart with same SQLite, state must still be
        // completed and the final audit must still be reachable.
        {
            let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
            daemon.start().unwrap();
            let Some((server, listener)) = bind_or_skip(&socket, None) else {
                daemon.stop().unwrap();
                let _ = fs::remove_dir_all(dir);
                return;
            };
            let response = request_via_ipc(
                &server,
                &listener,
                &mut daemon,
                json!({"id":"obs","command":"GetMissionDetails","mission_id": seeded_mission_id}),
            );
            assert_eq!(response["state"], "completed");
            assert_eq!(response["terminal"], true);
            assert_eq!(response["completion"]["completion_allowed"], true);
            server.cleanup();
            daemon.stop().unwrap();
        }
        let _ = fs::remove_dir_all(dir);
    }
}
