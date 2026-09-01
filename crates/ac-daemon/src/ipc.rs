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
        "ConversationCreate" => {
            let project_path = request.get("project_path").and_then(Value::as_str).unwrap_or("");
            let mode = request.get("mode").and_then(Value::as_str).unwrap_or("GOAL");
            let title = request.get("title").and_then(Value::as_str).unwrap_or("");
            match daemon.create_conversation(project_path, mode, title) {
                Ok(id) => json!({"id": correlation_id, "ok": true, "conversation_id": id.to_string()}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "ConversationList" => {
            let project_path = request.get("project_path").and_then(Value::as_str).unwrap_or("");
            match daemon.list_conversations(project_path) {
                Ok(conversations) => json!({"id": correlation_id, "ok": true, "conversations": conversations.into_iter().map(conversation_json).collect::<Vec<_>>()}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "ConversationGet" => match request.get("conversation_id").and_then(Value::as_str) {
            Some(id) => match daemon.get_conversation_with_messages(id) {
                Ok(Some(conversation)) => json!({"id": correlation_id, "ok": true, "conversation": conversation_with_messages_json(conversation)}),
                Ok(None) => error_response(correlation_id, "CONVERSATION-NOT_FOUND", "conversation not found".to_string()),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "conversation_id is required".to_string()),
        },
        "ConversationRename" => {
            let id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let title = request.get("title").and_then(Value::as_str).unwrap_or("");
            match daemon.rename_conversation(id, title) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "conversation_id": id}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "ConversationArchive" => {
            let id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.archive_conversation(id) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "conversation_id": id}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "ConversationDelete" => {
            let id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.delete_conversation(id) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "conversation_id": id}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "MessageAppend" => {
            let id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let role = request.get("role").and_then(Value::as_str).unwrap_or("user");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let mission_ref = request.get("mission_ref").and_then(Value::as_str);
            let metadata_json = request.get("metadata").and_then(Value::as_str).unwrap_or("{}");
            match daemon.append_message(id, role, content, mission_ref, metadata_json) {
                Ok(message) => json!({"id": correlation_id, "ok": true, "message": message_json(message)}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "AttachmentRegister" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let project_path = request.get("project_path").and_then(Value::as_str).unwrap_or("");
            let filename = request.get("filename").and_then(Value::as_str).unwrap_or("");
            let mime_type = request.get("mime_type").and_then(Value::as_str).unwrap_or("");
            let size_bytes = request.get("size_bytes").and_then(Value::as_i64).unwrap_or(0);
            let content_hash = request.get("content_hash").and_then(Value::as_str).unwrap_or("");
            let rel_path = request.get("rel_path").and_then(Value::as_str).unwrap_or("");
            match daemon.register_attachment(conversation_id, project_path, filename, mime_type, size_bytes, content_hash, rel_path) {
                Ok(attachment) => json!({"id": correlation_id, "ok": true, "attachment": attachment_json(attachment)}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "AttachmentList" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.list_attachments(conversation_id) {
                Ok(attachments) => json!({"id": correlation_id, "ok": true, "attachments": attachments.into_iter().map(attachment_json).collect::<Vec<_>>()}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "AttachmentPath" => {
            let attachment_id = request.get("attachment_id").and_then(Value::as_str).unwrap_or("");
            let project_path = request.get("project_path").and_then(Value::as_str).unwrap_or("");
            match daemon.attachment_path(attachment_id, project_path) {
                Ok(Some(path)) => json!({"id": correlation_id, "ok": true, "path": path}),
                Ok(None) => error_response(correlation_id, "CONVERSATION-ATTACHMENT_NOT_FOUND", "attachment not found".to_string()),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "AttachmentRemove" => {
            let id = request.get("attachment_id").and_then(Value::as_str).unwrap_or("");
            match daemon.remove_attachment(id) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "attachment_id": id}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "GoalSubmit" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let goal = request.get("goal").and_then(Value::as_str).unwrap_or("");
            let attachment_ids = request.get("attachment_ids").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect::<Vec<_>>()
            }).unwrap_or_default();
            match daemon.goal_submit(conversation_id, goal, &attachment_ids) {
                Ok((mission_id, session_id)) => json!({"id": correlation_id, "ok": true, "mission_id": mission_id.to_string(), "session_id": session_id.to_string()}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "ConversationActivity" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.conversation_activity(conversation_id) {
                Ok(mut payload) => {
                    payload["id"] = json!(correlation_id);
                    payload["ok"] = json!(true);
                    payload
                }
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
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

fn conversation_json(row: ac_db::ConversationRow) -> Value {
    json!({
        "id": row.id,
        "project_path": row.project_path,
        "mode": row.mode,
        "title": row.title,
        "state": row.state,
        "current_mission_id": row.current_mission_id,
        "created_at_ms": row.created_at_ms,
        "updated_at_ms": row.updated_at_ms,
    })
}

fn message_json(row: ac_db::ConversationMessageRow) -> Value {
    json!({
        "id": row.id,
        "conversation_id": row.conversation_id,
        "role": row.role,
        "content": row.content,
        "mission_ref": row.mission_ref,
        "metadata": row.metadata_json,
        "created_at_ms": row.created_at_ms,
    })
}

fn attachment_json(row: ac_db::AttachmentRow) -> Value {
    json!({
        "id": row.id,
        "conversation_id": row.conversation_id,
        "message_id": row.message_id,
        "project_path": row.project_path,
        "filename": row.filename,
        "mime_type": row.mime_type,
        "size_bytes": row.size_bytes,
        "sha256": row.sha256,
        "sensitivity": row.sensitivity,
        "created_at_ms": row.created_at_ms,
    })
}

fn conversation_with_messages_json(data: crate::ConversationWithMessages) -> Value {
    json!({
        "id": data.conversation.id,
        "project_path": data.conversation.project_path,
        "mode": data.conversation.mode,
        "title": data.conversation.title,
        "state": data.conversation.state,
        "current_mission_id": data.conversation.current_mission_id,
        "created_at_ms": data.conversation.created_at_ms,
        "updated_at_ms": data.conversation.updated_at_ms,
        "messages": data.messages.into_iter().map(message_json).collect::<Vec<_>>(),
        "attachments": data.attachments.into_iter().map(attachment_json).collect::<Vec<_>>(),
    })
}

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

    #[test]
    fn conversation_create_list_get_rename_archive_delete_via_ipc() {
        let (dir, db, lock, socket) = temp_paths("conv-crud");
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Create
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path":"/tmp/test-proj","mode":"GOAL","title":"Test Goal Chat"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // List
        let list = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationList","project_path":"/tmp/test-proj"}),
        );
        assert_eq!(list["ok"], true, "list: {list}");
        let convs = list["conversations"].as_array().unwrap();
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0]["title"], "Test Goal Chat");

        // Get
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c3","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true, "get: {get}");
        assert_eq!(get["conversation"]["title"], "Test Goal Chat");
        assert_eq!(get["conversation"]["mode"], "GOAL");

        // Rename
        let rename = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c4","command":"ConversationRename","conversation_id": cid, "title": "Renamed"}),
        );
        assert_eq!(rename["ok"], true);
        let get2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c5","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get2["conversation"]["title"], "Renamed");

        // Archive
        let archive = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c6","command":"ConversationArchive","conversation_id": cid}),
        );
        assert_eq!(archive["ok"], true);
        let list2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c7","command":"ConversationList","project_path":"/tmp/test-proj"}),
        );
        // Archived conversations should still appear (include_archived=true)
        assert_eq!(list2["conversations"].as_array().unwrap().len(), 1);

        // Delete
        let del = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c8","command":"ConversationDelete","conversation_id": cid}),
        );
        assert_eq!(del["ok"], true);
        let list3 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c9","command":"ConversationList","project_path":"/tmp/test-proj"}),
        );
        assert_eq!(list3["conversations"].as_array().unwrap().len(), 0);

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_message_append_and_retrieve_via_ipc() {
        let (dir, db, lock, socket) = temp_paths("conv-msg");
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path":"/tmp/msg-proj","mode":"GOAL","title":"Msg Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let append = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"m1","command":"MessageAppend","conversation_id": cid, "role": "user", "content": "Hello from conversation test"}),
        );
        assert_eq!(append["ok"], true, "append: {append}");
        assert_eq!(append["message"]["role"], "user");
        assert_eq!(append["message"]["content"], "Hello from conversation test");
        assert!(append["message"]["id"].as_str().unwrap().starts_with("msg-"));

        // Retrieve via ConversationGet
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["content"], "Hello from conversation test");

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_project_isolation_via_ipc() {
        let (dir, db, lock, socket) = temp_paths("conv-iso");
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let c1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path":"/proj/A","mode":"GOAL","title":"A1"}),
        );
        assert_eq!(c1["ok"], true);
        let c2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path":"/proj/A","mode":"GOAL","title":"A2"}),
        );
        assert_eq!(c2["ok"], true);
        let c3 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c3","command":"ConversationCreate","project_path":"/proj/B","mode":"GOAL","title":"B1"}),
        );
        assert_eq!(c3["ok"], true);

        let list_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"la","command":"ConversationList","project_path":"/proj/A"}),
        );
        assert_eq!(list_a["conversations"].as_array().unwrap().len(), 2);

        let list_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"lb","command":"ConversationList","project_path":"/proj/B"}),
        );
        assert_eq!(list_b["conversations"].as_array().unwrap().len(), 1);

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_operations_return_errors_for_invalid_inputs() {
        let (dir, db, lock, socket) = temp_paths("conv-err");
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Get non-existent conversation
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"e1","command":"ConversationGet","conversation_id":"conv-nope"}),
        );
        assert!(!get["ok"].as_bool().unwrap());
        assert_eq!(get["error"]["code"], "CONVERSATION-NOT_FOUND");

        // MessageAppend on non-existent conversation
        let append = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"e2","command":"MessageAppend","conversation_id":"conv-nope","role":"user","content":"test"}),
        );
        assert!(!append["ok"].as_bool().unwrap());
        assert_eq!(append["error"]["code"], "CONVERSATION-NOT_FOUND");

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn goal_submit_links_conversation_to_real_mission() {
        let (dir, db, lock, socket) = temp_paths("conv-goal");
        // A real project workspace directory is required by goal_submit
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Goal Chat"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Original request must be preserved and submitted as a real mission.
        let goal = "Fix the authentication bug where expired refresh tokens are accepted.";
        let submit = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": goal}),
        );
        assert_eq!(submit["ok"], true, "submit: {submit}");
        let mission_id = submit["mission_id"].as_str().unwrap().to_string();
        assert!(mission_id.starts_with("mission-"), "mission id prefix: {mission_id}");
        assert!(submit["session_id"].as_str().unwrap().starts_with("session-"));

        // The conversation must now be attached to the mission.
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        assert_eq!(
            get["conversation"]["current_mission_id"].as_str().unwrap(),
            mission_id
        );
        // The exact original request is preserved as a user message.
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], goal);
        assert_eq!(msgs[0]["mission_ref"].as_str().unwrap(), mission_id);

        // Cancel immediately (matching the durable-submission test pattern) so
        // the coordinator worker never executes the mission against real
        // providers in the test environment.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mission_id}),
        );
        assert_eq!(cancel["ok"], true, "cancel: {cancel}");

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn attachment_register_list_path_and_remove_via_ipc() {
        let (dir, db, lock, socket) = temp_paths("conv-att");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Attach Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Write a real attachment file inside the project workspace, as the
        // Tauri add_attachment command does.
        let att_dir = project_dir.join(".agentcode").join("attachments").join(&cid);
        fs::create_dir_all(&att_dir).unwrap();
        let content = b"hello attachment world";
        let rel_path = format!(".agentcode/attachments/{cid}/img.png");
        fs::write(project_dir.join(&rel_path), content).unwrap();
        let hash = format!(
            "fnv1a64:{:016x}",
            content
                .iter()
                .fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
        );

        let reg = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"img.png","mime_type":"image/png","size_bytes": content.len() as i64,"content_hash": hash,"rel_path": rel_path}),
        );
        assert_eq!(reg["ok"], true, "register: {reg}");
        let attachment = &reg["attachment"];
        assert_eq!(attachment["filename"], "img.png");
        assert_eq!(attachment["mime_type"], "image/png");
        assert_eq!(attachment["size_bytes"], content.len() as i64);
        assert!(attachment["id"].as_str().unwrap().starts_with("att-"));
        let att_id = attachment["id"].as_str().unwrap().to_string();

        // List
        let list = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a2","command":"AttachmentList","conversation_id": cid}),
        );
        assert_eq!(list["ok"], true);
        assert_eq!(list["attachments"].as_array().unwrap().len(), 1);

        // Path (project isolation + workspace boundary)
        let path = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a3","command":"AttachmentPath","attachment_id": att_id, "project_path": project_path}),
        );
        assert_eq!(path["ok"], true, "path: {path}");
        let resolved = path["path"].as_str().unwrap();
        assert!(resolved.ends_with("img.png"));
        let project_canonical = fs::canonicalize(&project_dir).unwrap();
        assert!(
            resolved.starts_with(project_canonical.to_string_lossy().as_ref()),
            "resolved {resolved} must be inside project {project_canonical:?}"
        );

        // Remove
        let remove = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a4","command":"AttachmentRemove","attachment_id": att_id}),
        );
        assert_eq!(remove["ok"], true);
        let list2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a5","command":"AttachmentList","conversation_id": cid}),
        );
        assert_eq!(list2["attachments"].as_array().unwrap().len(), 0);

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn attachment_rejects_invalid_mime_oversize_and_workspace_escape() {
        let (dir, db, lock, socket) = temp_paths("conv-att-sec");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Secure Attach"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // 1. Invalid MIME type must be rejected.
        let att_dir = project_dir.join(".agentcode").join("attachments").join(&cid);
        fs::create_dir_all(&att_dir).unwrap();
        let bad = format!(".agentcode/attachments/{cid}/evil.exe");
        fs::write(project_dir.join(&bad), b"x").unwrap();
        let bad_mime = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b1","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"evil.exe","mime_type":"application/x-msdownload","size_bytes":1,"content_hash":"fnv1a64:0000000000000001","rel_path": bad}),
        );
        assert!(!bad_mime["ok"].as_bool().unwrap());
        assert_eq!(bad_mime["error"]["code"], "CONVERSATION-INVALID_MIME");

        // 2. Oversized attachment must be rejected.
        let big = format!(".agentcode/attachments/{cid}/big.bin");
        fs::write(project_dir.join(&big), vec![0u8; 26 * 1024 * 1024]).unwrap();
        let big_att = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b2","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"big.bin","mime_type":"application/octet-stream","size_bytes": 26 * 1024 * 1024,"content_hash":"fnv1a64:0000000000000001","rel_path": big}),
        );
        assert!(!big_att["ok"].as_bool().unwrap());
        assert_eq!(big_att["error"]["code"], "CONVERSATION-INVALID_SIZE");

        // 3. Workspace escape via rel_path traversal must be rejected.
        let outside = project_dir.join("..").join("outside.png");
        fs::create_dir_all(project_dir.parent().unwrap()).unwrap();
        fs::write(&outside, b"png").unwrap();
        let escape = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b3","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"outside.png","mime_type":"image/png","size_bytes":3,"content_hash":"fnv1a64:0000000000000001","rel_path":"../outside.png"}),
        );
        assert!(!escape["ok"].as_bool().unwrap());
        assert_eq!(escape["error"]["code"], "CONVERSATION-FILE_ESCAPE");

        // 4. AttachmentPath for another project must be denied (isolation).
        let other = dir.join("other-project");
        fs::create_dir_all(&other).unwrap();
        let other_path = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b4","command":"AttachmentPath","attachment_id":"att-none","project_path": other.to_string_lossy().to_string()}),
        );
        assert!(!other_path["ok"].as_bool().unwrap());
        assert_eq!(other_path["error"]["code"], "CONVERSATION-ATTACHMENT_NOT_FOUND");

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_activity_projects_mission_block() {
        let (dir, db, lock, socket) = temp_paths("conv-act");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Act Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let goal = "Fix the login bug";
        let submit = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": goal}),
        );
        assert_eq!(submit["ok"], true, "submit: {submit}");
        let mission_id = submit["mission_id"].as_str().unwrap().to_string();

        // ConversationActivity must return a mission block.
        let activity = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
        );
        assert_eq!(activity["ok"], true, "activity: {activity}");
        let missions = activity["missions"].as_array().unwrap();
        assert_eq!(missions.len(), 1, "should have 1 mission, got {missions:?}");
        assert_eq!(missions[0]["mission_id"], mission_id);
        assert!(
            missions[0].get("details").and_then(|v| v.get("state")).is_some(),
            "details should have state"
        );

        // Also verify the conversation get still works.
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["mission_ref"].as_str().unwrap(), mission_id);

        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mission_id}),
        );
        assert_eq!(cancel["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_follow_up_creates_second_mission_in_same_conversation() {
        let (dir, db, lock, socket) = temp_paths("conv-follow");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Follow Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // First mission
        let s1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": "Fix login bug"}),
        );
        assert_eq!(s1["ok"], true);
        let m1 = s1["mission_id"].as_str().unwrap().to_string();

        // Second mission (follow-up in same conversation)
        let s2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s2","command":"GoalSubmit","conversation_id": cid, "goal": "Add tests for the fix"}),
        );
        assert_eq!(s2["ok"], true, "second submit: {s2}");
        let m2 = s2["mission_id"].as_str().unwrap().to_string();
        assert_ne!(m1, m2, "second mission must have a different id");

        // Conversation must have both messages with their mission_refs
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2, "should have 2 messages, got {msgs:?}");
        assert_eq!(msgs[0]["mission_ref"].as_str().unwrap(), m1);
        assert_eq!(msgs[1]["mission_ref"].as_str().unwrap(), m2);

        // current_mission_id must be the latest
        assert_eq!(get["conversation"]["current_mission_id"].as_str().unwrap(), m2);

        // Activity must project both missions
        let activity = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
        );
        assert_eq!(activity["ok"], true);
        let missions = activity["missions"].as_array().unwrap();
        assert_eq!(missions.len(), 2, "should have 2 missions, got {missions:?}");
        let ids: Vec<&str> = missions.iter().map(|m| m["mission_id"].as_str().unwrap()).collect();
        assert!(ids.contains(&m1.as_str()));
        assert!(ids.contains(&m2.as_str()));

        // Cancel both
        for mid in [&m1, &m2] {
            let _ = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"x","command":"CancelMission","mission_id": mid}),
            );
        }

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_activity_deduplicates_repeated_mission_refs() {
        let (dir, db, lock, socket) = temp_paths("conv-dedup");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Dedup Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Submit a single goal; the daemon creates one user message with mission_ref.
        let s1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": "Fix the bug"}),
        );
        let mid = s1["mission_id"].as_str().unwrap().to_string();

        // Append a second user message manually, referencing the same mission.
        let append = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"m2","command":"MessageAppend","conversation_id": cid, "role": "user", "content": "Also fix this", "mission_ref": mid}),
        );
        assert_eq!(append["ok"], true);

        // Activity must deduplicate: only one mission block despite two messages.
        let activity = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
        );
        assert_eq!(activity["ok"], true);
        let missions = activity["missions"].as_array().unwrap();
        assert_eq!(missions.len(), 1, "should have 1 mission (deduped), got {missions:?}");

        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mid}),
        );
        assert_eq!(cancel["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_activity_restores_after_daemon_restart() {
        let (dir, db, lock, socket) = temp_paths("conv-restart");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Restart Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let s1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": "Fix the bug"}),
        );
        let mid = s1["mission_id"].as_str().unwrap().to_string();

        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mid}),
        );
        assert_eq!(cancel["ok"], true);

        // Shut down the daemon.
        server.cleanup();
        daemon.shutdown().unwrap();

        // Reopen the same DB and restart.
        let mut daemon2 = DaemonService::open(&db, &lock).unwrap();
        daemon2.start().unwrap();
        let Some((server2, listener2)) = bind_or_skip(&socket, None) else {
            daemon2.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // ConversationGet must still return the messages.
        let get = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["mission_ref"].as_str().unwrap(), mid);

        // ConversationActivity must still project the mission.
        let activity = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
        );
        assert_eq!(activity["ok"], true);
        let missions = activity["missions"].as_array().unwrap();
        assert_eq!(missions.len(), 1, "activity after restart: {activity}");
        assert_eq!(missions[0]["mission_id"], mid);

        server2.cleanup();
        daemon2.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_activity_project_isolation() {
        let (dir, db, lock, socket) = temp_paths("conv-act-iso");
        let project_a = dir.join("project-a");
        let project_b = dir.join("project-b");
        fs::create_dir_all(&project_a).unwrap();
        fs::create_dir_all(&project_b).unwrap();
        let path_a = project_a.to_string_lossy().to_string();
        let path_b = project_b.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Create conversation A, submit a goal.
        let create_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": path_a, "mode":"GOAL","title":"Project A"}),
        );
        let cid_a = create_a["conversation_id"].as_str().unwrap().to_string();
        let s1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid_a, "goal": "Fix A"}),
        );
        let mid_a = s1["mission_id"].as_str().unwrap().to_string();

        // Create conversation B, submit a different goal.
        let create_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": path_b, "mode":"GOAL","title":"Project B"}),
        );
        let cid_b = create_b["conversation_id"].as_str().unwrap().to_string();
        let s2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s2","command":"GoalSubmit","conversation_id": cid_b, "goal": "Fix B"}),
        );
        let mid_b = s2["mission_id"].as_str().unwrap().to_string();

        // Activity for conversation A must only contain mission A.
        let activity_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid_a}),
        );
        assert_eq!(activity_a["ok"], true);
        let missions_a: Vec<&str> = activity_a["missions"].as_array().unwrap().iter()
            .map(|m| m["mission_id"].as_str().unwrap()).collect();
        assert!(missions_a.contains(&mid_a.as_str()), "project A should have mission A");
        assert!(!missions_a.contains(&mid_b.as_str()), "project A should NOT have mission B");

        // Activity for conversation B must only contain mission B.
        let activity_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a2","command":"ConversationActivity","conversation_id": cid_b}),
        );
        assert_eq!(activity_b["ok"], true);
        let missions_b: Vec<&str> = activity_b["missions"].as_array().unwrap().iter()
            .map(|m| m["mission_id"].as_str().unwrap()).collect();
        assert!(missions_b.contains(&mid_b.as_str()), "project B should have mission B");
        assert!(!missions_b.contains(&mid_a.as_str()), "project B should NOT have mission A");

        // Cancel both
        for mid in [&mid_a, &mid_b] {
            let _ = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"x","command":"CancelMission","mission_id": mid}),
            );
        }

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn goal_submit_with_text_attachment_feeds_goal_context() {
        // Proves that when a text attachment is registered and then submitted
        // via GoalSubmit, the attachment content is loaded and the resulting
        // Goal object carries the typed ContextAttachment.
        let (dir, db, lock, socket) = temp_paths("conv-att-ctx");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Create conversation
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Ctx Attach"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Write a text attachment file inside the workspace.
        let att_dir = project_dir.join(".agentcode").join("attachments").join(&cid);
        fs::create_dir_all(&att_dir).unwrap();
        let text_content = b"fn main() { println!(\"hello from attachment\"); }";
        let rel_path = format!(".agentcode/attachments/{cid}/code.rs");
        fs::write(project_dir.join(&rel_path), text_content).unwrap();
        let hash = format!(
            "fnv1a64:{:016x}",
            text_content.iter()
                .fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
        );

        // Register the attachment.
        let reg = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"code.rs","mime_type":"text/plain","size_bytes": text_content.len() as i64,"content_hash": hash,"rel_path": rel_path}),
        );
        assert_eq!(reg["ok"], true, "register: {reg}");
        let att_id = reg["attachment"]["id"].as_str().unwrap().to_string();

        // Submit goal with this attachment.
        let goal = "Fix the code";
        let submit = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": goal, "attachment_ids": [att_id]}),
        );
        assert_eq!(submit["ok"], true, "submit: {submit}");
        let mission_id = submit["mission_id"].as_str().unwrap().to_string();

        // The message must be linked to the attachment.
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let atts = get["conversation"]["attachments"].as_array().unwrap();
        assert_eq!(atts.len(), 1, "should have 1 attachment, got {atts:?}");
        assert_eq!(atts[0]["filename"], "code.rs");
        // The message_id on the attachment links to the user message.
        assert!(
            atts[0].get("message_id").and_then(|v| v.as_str()).is_some(),
            "attachment should be linked to a message"
        );

        // The mission is created and the context pipeline will receive the
        // attachment content through the Goal::with_attachments mechanism.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mission_id}),
        );
        assert_eq!(cancel["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_activity_projects_failed_mission_blocks() {
        let (dir, db, lock, socket) = temp_paths("conv-fail");
        let project_dir = dir.join("workspace");
        let _ = fs::create_dir_all(project_dir.join("src"));
        // Create a minimal git repo so the coordinator doesn't abort immediately.
        let _ = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&project_dir)
            .output();
        let _ = std::process::Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@test", "commit", "--allow-empty", "-m", "initial"])
            .current_dir(&project_dir)
            .output();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Fail Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Submit a goal, then immediately cancel so the coordinator records a
        // terminal state (cancelled) rather than trying to run the mission.
        let s1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": "Test mission for failure projection"}),
        );
        assert_eq!(s1["ok"], true);
        let mid = s1["mission_id"].as_str().unwrap().to_string();

        // Cancel immediately so the coordinator never runs against providers.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mid}),
        );
        assert_eq!(cancel["ok"], true);

        // Poll until the coordinator records the terminal cancelled state.
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let activity = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
            );
            assert_eq!(activity["ok"], true);
            let missions = activity["missions"].as_array().unwrap();
            assert_eq!(missions.len(), 1, "should have 1 mission, got {missions:?}");
            assert_eq!(missions[0]["mission_id"], mid);
            let state = missions[0]["details"]["state"].as_str().unwrap_or("");
            // The mission may be cancelled or failed; both are terminal.
            if state == "cancelled" || state == "failed" || state.starts_with("failed:") {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "mission did not reach terminal; last state={state}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }

        // Cancel again if not already cancelled (idempotent).
        let _ = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x2","command":"CancelMission","mission_id": mid}),
        );

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_activity_projects_cancelled_mission_blocks() {
        let (dir, db, lock, socket) = temp_paths("conv-cancel");
        let project_dir = dir.join("workspace");
        let _ = fs::create_dir_all(project_dir.join("src"));
        // Create a minimal git repo so the coordinator doesn't abort immediately.
        let _ = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&project_dir)
            .output();
        let _ = std::process::Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@test", "commit", "--allow-empty", "-m", "initial"])
            .current_dir(&project_dir)
            .output();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Cancel Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let s1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": "Test mission for cancellation"}),
        );
        assert_eq!(s1["ok"], true);
        let mid = s1["mission_id"].as_str().unwrap().to_string();

        // Cancel immediately after submission.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mid}),
        );
        assert_eq!(cancel["ok"], true);

        // Poll until the mission reaches a terminal state.
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let activity = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
            );
            assert_eq!(activity["ok"], true);
            let missions = activity["missions"].as_array().unwrap();
            assert_eq!(missions.len(), 1, "should have 1 mission, got {missions:?}");
            let state = missions[0]["details"]["state"].as_str().unwrap_or("");
            // The coordinator may cancel or fail the mission; both are terminal.
            if state == "cancelled" || state == "failed" || state.starts_with("failed:") {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "mission did not reach terminal; last state={state}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn conversation_activity_summary_reports_failures_tools_and_retries() {
        let (dir, db, lock, socket) = temp_paths("conv-summary");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Summary Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let s1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": "Summary test mission"}),
        );
        assert_eq!(s1["ok"], true);
        let mid = s1["mission_id"].as_str().unwrap().to_string();

        // ConversationActivity must include a factual summary section.
        let activity = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
        );
        assert_eq!(activity["ok"], true, "activity: {activity}");
        let missions = activity["missions"].as_array().unwrap();
        assert_eq!(missions.len(), 1);
        let summary = &missions[0]["summary"];
        assert!(summary.is_object(), "summary object expected: {summary}");
        assert!(summary.get("tools_used").is_some(), "tools_used");
        assert!(summary.get("commands").is_some(), "commands");
        assert!(summary.get("files_changed").is_some(), "files_changed");
        assert!(summary.get("failure_classes").is_some(), "failure_classes");
        assert!(summary.get("retry_count").is_some(), "retry_count");

        // Cancelled so the coordinator never runs the mission against providers.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mid}),
        );
        assert_eq!(cancel["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn attachment_context_skips_cross_project_and_missing_files() {
        // Proves the attachment→context pipeline refuses to read files that do
        // not belong to the conversation's project (workspace boundary) and
        // silently skips files that no longer exist — nothing foreign enters
        // the model context.
        let (dir, db, lock, socket) = temp_paths("conv-att-ctx-sec");
        let project_a = dir.join("project-a");
        let project_b = dir.join("project-b");
        fs::create_dir_all(&project_a).unwrap();
        fs::create_dir_all(&project_b).unwrap();
        let path_a = project_a.to_string_lossy().to_string();
        let path_b = project_b.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Conversation in project A, then register a real text attachment that
        // belongs to project A.
        let create_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": path_a, "mode":"GOAL","title":"Proj A"}),
        );
        let cid_a = create_a["conversation_id"].as_str().unwrap().to_string();

        let att_dir = project_a.join(".agentcode").join("attachments").join(&cid_a);
        fs::create_dir_all(&att_dir).unwrap();
        let content = b"fn secret() -> u32 { 42 }";
        let rel_path = format!(".agentcode/attachments/{cid_a}/a.rs");
        fs::write(project_a.join(&rel_path), content).unwrap();
        let hash = format!(
            "fnv1a64:{:016x}",
            content.iter()
                .fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
        );
        let reg = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"AttachmentRegister","conversation_id": cid_a, "project_path": path_a, "filename":"a.rs","mime_type":"text/plain","size_bytes": content.len() as i64,"content_hash": hash,"rel_path": rel_path}),
        );
        assert_eq!(reg["ok"], true, "register: {reg}");
        let att_id = reg["attachment"]["id"].as_str().unwrap().to_string();

        // The conversation's project must match the attachment's project, so
        // load_goal_attachments must skip it when asked for project B.
        let ctx = daemon
            .load_goal_attachments(std::slice::from_ref(&att_id), &path_b)
            .unwrap();
        assert!(
            ctx.is_empty(),
            "cross-project attachment must not enter context: {ctx:?}"
        );

        // A missing file (registered but deleted) must be skipped, not crash.
        let rel_missing = format!(".agentcode/attachments/{cid_a}/gone.rs");
        fs::write(project_a.join(&rel_missing), b"gone").unwrap();
        let hash2 = format!(
            "fnv1a64:{:016x}",
            b"gone".iter()
                .fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
        );
        let reg2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a2","command":"AttachmentRegister","conversation_id": cid_a, "project_path": path_a, "filename":"gone.rs","mime_type":"text/plain","size_bytes":4,"content_hash": hash2,"rel_path": rel_missing}),
        );
        assert_eq!(reg2["ok"], true);
        let att2_id = reg2["attachment"]["id"].as_str().unwrap().to_string();
        fs::remove_file(project_a.join(&rel_missing)).unwrap();
        let ctx2 = daemon
            .load_goal_attachments(std::slice::from_ref(&att2_id), &path_a)
            .unwrap();
        assert!(
            ctx2.is_empty(),
            "missing attachment file must not enter context: {ctx2:?}"
        );

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn attachment_context_never_injects_oversized_content() {
        // Proves that even when a large attachment exists on disk, the context
        // pipeline only ever reads a bounded prefix — oversized content cannot
        // flood the model prompt.
        let (dir, db, lock, socket) = temp_paths("conv-att-ctx-big");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Big Attach"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Write a 1 MB text file (below the registration cap of 25 MB so it
        // registers, but far above the context pipeline's 32 KB read bound).
        let att_dir = project_dir.join(".agentcode").join("attachments").join(&cid);
        fs::create_dir_all(&att_dir).unwrap();
        let big = vec![b'a'; 1024 * 1024];
        let rel_path = format!(".agentcode/attachments/{cid}/big.txt");
        fs::write(project_dir.join(&rel_path), &big).unwrap();
        let hash = format!(
            "fnv1a64:{:016x}",
            big.iter()
                .fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
        );
        let reg = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"big.txt","mime_type":"text/plain","size_bytes": big.len() as i64,"content_hash": hash,"rel_path": rel_path}),
        );
        assert_eq!(reg["ok"], true, "register: {reg}");
        let att_id = reg["attachment"]["id"].as_str().unwrap().to_string();

        // The context attachment must carry bounded content (≤ 32 KB).
        let ctx = daemon
            .load_goal_attachments(std::slice::from_ref(&att_id), &project_path)
            .unwrap();
        assert_eq!(ctx.len(), 1);
        match &ctx[0].content {
            ac_agent::AttachmentContent::Text(text) => {
                assert!(
                    text.len() <= 32768,
                    "context content must be bounded, got {} bytes",
                    text.len()
                );
            }
            other => panic!("text attachment should be Text, got {other:?}"),
        }

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

}
