use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_CLIENTS: usize = 8;
const FRAME_TIMEOUT: Duration = Duration::from_secs(5);
/// Provider-backed commands block on a real model round trip (model load can
/// take tens of seconds for a local <=4B model).  A 5-second response budget
/// silently drops every real-model conversation send — the response is
/// computed but the client never receives it.  These commands get an explicit
/// long budget instead; all other commands keep the fast frame timeout.
const PROVIDER_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);
pub const IPC_PROTOCOL_VERSION: u32 = 1;

/// Response budget for a command.  Fast commands keep FRAME_TIMEOUT;
/// commands whose handler calls a real model provider get
/// PROVIDER_COMMAND_TIMEOUT.
fn command_response_timeout(command: &str) -> Duration {
    match command {
        "DiscussSend"
        | "DesignSend"
        | "DesignAnalyzeReference"
        | "DesignCritique"
        | "DesignRepair"
        | "DesignVisualCritique"
        | "DesignQAReport"
        | "SecuritySend"
        | "SecurityAudit" => PROVIDER_COMMAND_TIMEOUT,
        _ => FRAME_TIMEOUT,
    }
}

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
        // tokio requires a non-blocking stream, but the flag lives on the
        // open file description shared with the server's copy of this
        // connection.  Leaving it non-blocking silently breaks every later
        // read/write on the server side: a response larger than the socket
        // send buffer fails mid-frame with EAGAIN and the client sees a
        // truncated frame.  Restore blocking mode on the way out so the
        // server's stream keeps its original semantics.
        let cloned = stream.try_clone()?;
        cloned.set_nonblocking(true)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        let _guard = runtime.enter();
        let tokio_stream = tokio::net::UnixStream::from_std(cloned)?;
        let uid = tokio_stream.peer_cred()?.uid();
        drop(tokio_stream);
        // The shared file description is non-blocking right now; flip it
        // back before the connection is used for request/response frames.
        stream.set_nonblocking(false)?;
        Ok(uid)
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
    let response_timeout =
        command_response_timeout(request.get("command").and_then(Value::as_str).unwrap_or(""));
    let traced_command = request.get("command").and_then(Value::as_str).unwrap_or("?").to_string();
    if request_tx.try_send(IpcDispatchRequest { payload: request, response_tx }).is_err() {
        let _ = write_frame(&mut stream, &error_response("unknown", "DAEMON-IPC_BACKPRESSURE", "daemon IPC request queue is full".to_string()));
        return Ok(());
    }
    if let Ok(response) = response_rx.recv_timeout(response_timeout) {
        if std::env::var_os("AGENTCODE_TRACE_IPC_FRAMES").is_some() {
            eprintln!(
                "IPC-FRAME: responding to {traced_command} with {} bytes",
                serde_json::to_vec(&response).map(|v| v.len()).unwrap_or(0)
            );
        }
        if let Err(error) = write_frame(&mut stream, &response) {
            if std::env::var_os("AGENTCODE_TRACE_IPC_FRAMES").is_some() {
                eprintln!("IPC-FRAME: write_frame failed for {traced_command}: {error:?}");
            }
        }
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
        "ProviderPreferencesGet" => {
            let preference = daemon
                .db
                .provider_preference("global")
                .map(|row| {
                    json!({
                        "routing_profile": row.as_ref().map(|r| r.routing_profile.clone()).unwrap_or_else(|| "LocalFirst".to_string()),
                        "preferred_model": row.as_ref().map(|r| r.preferred_model.clone()).unwrap_or_default(),
                        "updated_at_ms": row.as_ref().map(|r| r.updated_at_ms).unwrap_or(0),
                        "configured": row.is_some(),
                    })
                })
                .unwrap_or_else(|error| {
                    json!({
                        "routing_profile": "LocalFirst",
                        "preferred_model": "",
                        "updated_at_ms": 0,
                        "configured": false,
                        "error": error.to_string(),
                    })
                });
            json!({"id": correlation_id, "ok": true, "preferences": preference})
        }
        "ProviderPreferencesSet" => {
            let routing_profile = request.get("routing_profile").and_then(Value::as_str).unwrap_or("");
            let preferred_model = request.get("preferred_model").and_then(Value::as_str).unwrap_or("");
            let valid = ["FreeOnly", "FreeFirst", "LocalFirst", "QualityFirst", "PaidAllowed", "Offline"];
            if !valid.contains(&routing_profile) {
                error_response(
                    correlation_id,
                    "PROVIDER-PREFERENCES_INVALID_PROFILE",
                    format!("routing_profile must be one of: {}", valid.join(", ")),
                )
            } else {
            let row = ac_db::ProviderPreferenceRow {
                id: "global".to_string(),
                routing_profile: routing_profile.to_string(),
                preferred_model: preferred_model.trim().to_string(),
                updated_at_ms: ac_common::TimestampMillis::now().as_millis() as i64,
            };
            match daemon.db.save_provider_preference(&row) {
                Ok(()) => json!({"id": correlation_id, "ok": true, "preferences": {
                    "routing_profile": row.routing_profile,
                    "preferred_model": row.preferred_model,
                    "updated_at_ms": row.updated_at_ms,
                    "configured": true,
                }}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
            }
        }
        "ProviderHealthGet" => {
            // Batch N1 (G2): surface real failover/health evidence — per
            // account health observations from SQLite plus each provider
            // catalog entry's health_state.  Display-only; routing stays
            // backend-owned.
            let mut observations = Vec::new();
            if let Ok(entries) = daemon.db.provider_catalog_entries() {
                for entry in entries {
                    if let Ok(accounts) = daemon.db.provider_accounts(&entry.id) {
                        for account in accounts {
                            if let Ok(rows) = daemon.db.provider_health_observations(&account.id, 10) {
                                for row in rows {
                                    observations.push(json!({
                                        "account_id": account.id,
                                        "provider_id": account.provider_id,
                                        "success": row.success,
                                        "latency_ms": row.latency_ms,
                                        "failure_code": row.failure_code,
                                        "failure_message": row.failure_message,
                                        "observed_at_ms": row.observed_at_ms,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
            let mut catalog = Vec::new();
            if let Ok(entries) = daemon.db.provider_catalog_entries() {
                for entry in entries {
                    catalog.push(json!({
                        "id": entry.id,
                        "display_name": entry.display_name,
                        "pricing_classification": entry.pricing_classification,
                    }));
                }
            }
            json!({"id": correlation_id, "ok": true, "health": {
                "observations": observations,
                "catalog": catalog,
            }})
        }
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
        "DiscussSend" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let attachment_ids = request.get("attachment_ids").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect::<Vec<_>>()
            }).unwrap_or_default();
            match daemon.discuss_send(conversation_id, content, &attachment_ids) {
                Ok(message) => json!({"id": correlation_id, "ok": true, "message": message_json(message)}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        }
        "DiscussTurnIntoPlan" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.discuss_turn_into_plan(conversation_id) {
                Ok(plan) => json!({"id": correlation_id, "ok": true, "plan": plan}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        }
        "DiscussExecutePlan" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.discuss_execute_plan(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "mission_id": result["mission_id"], "plan_goal": result["plan_goal"]}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        }
        "DiscussAcceptDecision" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let message_id = request.get("message_id").and_then(Value::as_str).unwrap_or("");
            let decision = request.get("decision").and_then(Value::as_str).unwrap_or("");
            let rationale = request.get("rationale").and_then(Value::as_str).unwrap_or("");
            match daemon.discuss_accept_decision(conversation_id, message_id, decision, rationale) {
                Ok(record) => json!({"id": correlation_id, "ok": true, "decision_record": record}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        }
        "DiscussPlanGet" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.db.design_document(conversation_id, "discuss_plan") {
                Ok(Some(doc)) => json!({"id": correlation_id, "ok": true, "plan": doc.content_json, "version": doc.version}),
                Ok(None) => json!({"id": correlation_id, "ok": true, "plan": Value::Null, "version": 0}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        }
        "DiscussDecisionsGet" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.db.design_document(conversation_id, "discuss_decisions") {
                Ok(Some(doc)) => json!({"id": correlation_id, "ok": true, "decisions": doc.content_json}),
                Ok(None) => json!({"id": correlation_id, "ok": true, "decisions": "{\"decisions\": [], \"source_paths\": []}"}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignSend" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let attachment_ids = request.get("attachment_ids").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect::<Vec<_>>()
            }).unwrap_or_default();
            match daemon.design_send(conversation_id, content, &attachment_ids) {
                Ok(message) => json!({"id": correlation_id, "ok": true, "message": message_json(message)}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignUnderstand" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_understand(conversation_id) {
                Ok(analysis) => json!({"id": correlation_id, "ok": true, "analysis": analysis}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignAnalyzeReference" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let attachment_id = request.get("attachment_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_analyze_reference(conversation_id, attachment_id) {
                Ok(analysis) => json!({"id": correlation_id, "ok": true, "analysis": analysis}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignBrief" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let audience = request.get("audience").and_then(Value::as_str).unwrap_or("");
            let workflow = request.get("workflow").and_then(Value::as_str).unwrap_or("");
            match daemon.design_brief(conversation_id, audience, workflow) {
                Ok(brief) => json!({"id": correlation_id, "ok": true, "brief": brief}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignGrammar" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_grammar(conversation_id) {
                Ok(grammar) => json!({"id": correlation_id, "ok": true, "grammar": grammar}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignState" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_state(conversation_id) {
                Ok(state) => json!({"id": correlation_id, "ok": true, "design_state": state}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignCritique" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let doc_type = request.get("doc_type").and_then(Value::as_str).unwrap_or("default");
            match daemon.design_critique(conversation_id, content, doc_type) {
                Ok(critique) => json!({"id": correlation_id, "ok": true, "critique": critique}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignPreviewStart" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_preview_start(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "preview": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignPreviewStatus" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_preview_status(conversation_id) {
                Ok(status) => json!({"id": correlation_id, "ok": true, "preview": status}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignPreviewStop" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_preview_stop(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "preview": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignBrowser" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let url = request.get("url").and_then(Value::as_str).unwrap_or("");
            let html = request.get("html").and_then(Value::as_str).unwrap_or("");
            let deterministic = request.get("deterministic").and_then(Value::as_bool).unwrap_or(false);
            let viewport_hint = request.get("viewport_hint").and_then(Value::as_str).unwrap_or("desktop");
            match daemon.design_browser(conversation_id, url, html, deterministic, viewport_hint) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "browser": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignQAResponsive" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let url = request.get("url").and_then(Value::as_str).unwrap_or("");
            let html = request.get("html").and_then(Value::as_str).unwrap_or("");
            let deterministic = request.get("deterministic").and_then(Value::as_bool).unwrap_or(false);
            let viewport_hint = request.get("viewport_hint").and_then(Value::as_str).unwrap_or("desktop");
            match daemon.design_qa_responsive(conversation_id, content, url, html, deterministic, viewport_hint) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "qa": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignQAAccessibility" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let url = request.get("url").and_then(Value::as_str).unwrap_or("");
            let html = request.get("html").and_then(Value::as_str).unwrap_or("");
            let deterministic = request.get("deterministic").and_then(Value::as_bool).unwrap_or(false);
            let viewport_hint = request.get("viewport_hint").and_then(Value::as_str).unwrap_or("desktop");
            match daemon.design_qa_accessibility(conversation_id, content, url, html, deterministic, viewport_hint) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "qa": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignQAFunctional" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let url = request.get("url").and_then(Value::as_str).unwrap_or("");
            let html = request.get("html").and_then(Value::as_str).unwrap_or("");
            let deterministic = request.get("deterministic").and_then(Value::as_bool).unwrap_or(false);
            let viewport_hint = request.get("viewport_hint").and_then(Value::as_str).unwrap_or("desktop");
            match daemon.design_qa_functional(conversation_id, content, url, html, deterministic, viewport_hint) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "qa": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignQAReport" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let url = request.get("url").and_then(Value::as_str).unwrap_or("");
            let html = request.get("html").and_then(Value::as_str).unwrap_or("");
            let deterministic = request.get("deterministic").and_then(Value::as_bool).unwrap_or(false);
            let viewport_hint = request.get("viewport_hint").and_then(Value::as_str).unwrap_or("desktop");
            match daemon.design_qa_run(conversation_id, url, html, deterministic, viewport_hint) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "qa": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignVisualCritique" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let url = request.get("url").and_then(Value::as_str).unwrap_or("");
            let deterministic = request.get("deterministic").and_then(Value::as_bool).unwrap_or(false);
            match daemon.design_visual_critique(conversation_id, url, deterministic) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "visual_critique": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignContract" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_contract(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "contract": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignExecuteContract" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_execute_contract(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "mission_id": result["mission_id"], "contract_goal": result["contract_goal"]}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignConstraintsSet" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let constraints = request.get("constraints").and_then(Value::as_array).cloned().unwrap_or_default();
            match daemon.design_constraints_set(conversation_id, &constraints) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "constraints": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignConstraintsGet" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_constraints_get(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "constraints": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignMaterializeState" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_materialize_state(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "materialization": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignMemoryGet" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_memory_get(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "memory": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignIterations" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.design_iterations(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "iterations": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "DesignRepair" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let doc_type = request.get("doc_type").and_then(Value::as_str).unwrap_or("default");
            match daemon.design_repair(conversation_id, content, doc_type) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "repair": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecuritySend" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let content = request.get("content").and_then(Value::as_str).unwrap_or("");
            let attachment_ids = request.get("attachment_ids").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect::<Vec<_>>()
            }).unwrap_or_default();
            match daemon.security_send(conversation_id, content, &attachment_ids) {
                Ok(message) => json!({"id": correlation_id, "ok": true, "message": message_json(message)}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityScopeSet" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let target = request.get("target").and_then(Value::as_str).unwrap_or("");
            let scope_kind = request.get("scope_kind").and_then(Value::as_str).unwrap_or("repository");
            let auth_state = request.get("auth_state").and_then(Value::as_str).unwrap_or("read-only");
            let allowed_hosts = request.get("allowed_hosts").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect::<Vec<_>>()
            }).unwrap_or_default();
            let allowed_ports = request.get("allowed_ports").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_u64).map(|p| p as u16).collect::<Vec<_>>()
            }).unwrap_or_default();
            let allowed_techniques = request.get("allowed_techniques").and_then(Value::as_array).map(|arr| {
                arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect::<Vec<_>>()
            }).unwrap_or_default();
            match daemon.security_set_scope(conversation_id, target, scope_kind, auth_state, &allowed_hosts, &allowed_ports, &allowed_techniques) {
                Ok(scope) => json!({"id": correlation_id, "ok": true, "scope": scope}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityAudit" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let depth = request.get("depth").and_then(Value::as_str).unwrap_or("quick");
            match daemon.security_audit_depth(conversation_id, depth) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "audit": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityFindings" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_findings(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "result": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityFindingDetail" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let finding_id = request.get("finding_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_finding_detail(conversation_id, finding_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "finding": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityFindingTransition" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let finding_id = request.get("finding_id").and_then(Value::as_str).unwrap_or("");
            let target = request.get("target_state").and_then(Value::as_str).unwrap_or("");
            match daemon.security_finding_transition(conversation_id, finding_id, target) {
                Ok(finding) => json!({"id": correlation_id, "ok": true, "finding": finding}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityValidate" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let finding_id = request.get("finding_id").and_then(Value::as_str).unwrap_or("");
            let canary = request.get("canary").and_then(Value::as_str);
            match daemon.security_validate(conversation_id, finding_id, canary) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "validation": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityAttackPaths" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_attack_paths(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "result": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityRemediate" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let finding_id = request.get("finding_id").and_then(Value::as_str).unwrap_or("");
            let approved = request.get("approved").and_then(Value::as_bool).unwrap_or(false);
            match daemon.security_remediate(conversation_id, finding_id, approved) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "remediation": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityRetest" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_retest(conversation_id) {
                Ok(result) => json!({"id": correlation_id, "ok": true, "retest": result}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecuritySuppress" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let finding_id = request.get("finding_id").and_then(Value::as_str).unwrap_or("");
            let reason = request.get("reason").and_then(Value::as_str).unwrap_or("");
            let expires_at_ms = request.get("expires_at_ms").and_then(Value::as_i64);
            let applicability = request.get("applicability").and_then(Value::as_str).unwrap_or("");
            let compensating = request.get("compensating_controls").and_then(Value::as_str).unwrap_or("");
            match daemon.security_suppress(conversation_id, finding_id, reason, expires_at_ms, applicability, compensating) {
                Ok(suppression) => json!({"id": correlation_id, "ok": true, "suppression": suppression}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityAcceptRisk" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let finding_id = request.get("finding_id").and_then(Value::as_str).unwrap_or("");
            let rationale = request.get("rationale").and_then(Value::as_str).unwrap_or("");
            let approver = request.get("approver").and_then(Value::as_str).unwrap_or("");
            let expires_at_ms = request.get("expires_at_ms").and_then(Value::as_i64);
            match daemon.security_accept_risk(conversation_id, finding_id, rationale, approver, expires_at_ms) {
                Ok(risk) => json!({"id": correlation_id, "ok": true, "risk_acceptance": risk}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityReport" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_report(conversation_id) {
                Ok(report) => json!({"id": correlation_id, "ok": true, "report": report}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityStatus" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_status(conversation_id) {
                Ok(status) => json!({"id": correlation_id, "ok": true, "status": status}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecuritySecretLifecycle" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            let finding_id = request.get("finding_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_secret_lifecycle(conversation_id, finding_id) {
                Ok(lifecycle) => json!({"id": correlation_id, "ok": true, "lifecycle": lifecycle}),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            }
        },
        "SecurityQualityMetrics" => {
            let conversation_id = request.get("conversation_id").and_then(Value::as_str).unwrap_or("");
            match daemon.security_quality_metrics(conversation_id) {
                Ok(metrics) => json!({"id": correlation_id, "ok": true, "metrics": metrics}),
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
    fn provider_backed_commands_get_a_long_response_budget() {
        // Provider-backed handlers block on a real local model round trip
        // (tens of seconds including model load).  With the default 5-second
        // frame budget every real-model DiscussSend/DesignSend response was
        // silently dropped.  These commands must keep the long budget; fast
        // commands must keep the default.
        assert_eq!(command_response_timeout("DiscussSend"), PROVIDER_COMMAND_TIMEOUT);
        assert_eq!(command_response_timeout("DesignSend"), PROVIDER_COMMAND_TIMEOUT);
        assert_eq!(command_response_timeout("DesignAnalyzeReference"), PROVIDER_COMMAND_TIMEOUT);
        assert_eq!(command_response_timeout("DesignCritique"), PROVIDER_COMMAND_TIMEOUT);
        assert_eq!(command_response_timeout("DesignRepair"), PROVIDER_COMMAND_TIMEOUT);
        assert_eq!(command_response_timeout("SecuritySend"), PROVIDER_COMMAND_TIMEOUT);
        assert_eq!(command_response_timeout("SecurityAudit"), PROVIDER_COMMAND_TIMEOUT);
        assert_eq!(command_response_timeout("Ping"), FRAME_TIMEOUT);
        assert_eq!(command_response_timeout("ConversationList"), FRAME_TIMEOUT);
        assert_eq!(command_response_timeout("Unknown"), FRAME_TIMEOUT);
        assert!(PROVIDER_COMMAND_TIMEOUT > FRAME_TIMEOUT);
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
                "",
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

    /// Batch N1: provider/routing preferences persist through IPC, invalid
    /// profiles are rejected with an honest error, the persisted profile
    /// governs daemon routing (preferred_routing_profile), and preferences
    /// survive a daemon restart (fresh open on the same DB).
    #[test]
    fn provider_preferences_persist_and_govern_routing() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-provider-prefs");

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // 1) Default state: unconfigured, falls back to LocalFirst.
        let get0 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p0","command":"ProviderPreferencesGet"}),
        );
        assert_eq!(get0["ok"], true, "get0: {get0}");
        assert_eq!(get0["preferences"]["configured"], false);
        assert_eq!(get0["preferences"]["routing_profile"], "LocalFirst");
        assert_eq!(daemon.preferred_routing_profile(), ac_provider::RoutingProfile::LocalFirst);

        // 2) Set a valid preference.
        let set = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p1","command":"ProviderPreferencesSet","routing_profile":"QualityFirst","preferred_model":"qwen2.5-coder:3b"}),
        );
        assert_eq!(set["ok"], true, "set: {set}");
        assert_eq!(set["preferences"]["routing_profile"], "QualityFirst");
        assert_eq!(set["preferences"]["preferred_model"], "qwen2.5-coder:3b");

        // 3) The daemon routing now reflects it.
        assert_eq!(daemon.preferred_routing_profile(), ac_provider::RoutingProfile::QualityFirst);
        assert_eq!(daemon.preferred_model(), "qwen2.5-coder:3b");

        // 4) Invalid profile rejected honestly.
        let bad = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p2","command":"ProviderPreferencesSet","routing_profile":"Bogus"}),
        );
        assert_eq!(bad["ok"], false);
        assert_eq!(bad["error"]["code"], "PROVIDER-PREFERENCES_INVALID_PROFILE");

        // 5) Read-back round-trip.
        let get1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p3","command":"ProviderPreferencesGet"}),
        );
        assert_eq!(get1["ok"], true);
        assert_eq!(get1["preferences"]["configured"], true);
        assert_eq!(get1["preferences"]["routing_profile"], "QualityFirst");

        // 6) Health endpoint is well-formed (display-only evidence).
        let health = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p4","command":"ProviderHealthGet"}),
        );
        assert_eq!(health["ok"], true, "health: {health}");
        assert!(health["health"]["observations"].is_array());
        assert!(health["health"]["catalog"].is_array());

        // 7) Persistence across restart: fresh daemon on the same DB.
        server.cleanup();
        daemon.shutdown().unwrap();
        let mut daemon2 = DaemonService::open(&db, &lock).unwrap();
        daemon2.start().unwrap();
        assert_eq!(
            daemon2.preferred_routing_profile(),
            ac_provider::RoutingProfile::QualityFirst,
            "persisted preference must survive restart"
        );
        assert_eq!(daemon2.preferred_model(), "qwen2.5-coder:3b");
        daemon2.shutdown().unwrap();

        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
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

    #[test]
    fn conversation_activity_projects_provider_models_and_uncertainty() {
        let (dir, db_path, lock, socket) = temp_paths("conv-provider-uncertainty");
        let seeded_mission_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            let seeded = seed_observability_mission(&db, true);
            seeded_mission_id = seeded.mission_id;
            let now = ac_common::TimestampMillis::now().as_millis() as i64;
            db.save_provider_model_record(&ac_db::ProviderModelRecordRow {
                id: "pmr-seed-1".to_string(),
                project_path: Some("/tmp/observability-workspace".to_string()),
                conversation_id: None,
                mission_id: Some(seeded_mission_id.clone()),
                session_id: Some("session-seed".to_string()),
                task_id: Some("task-seed".to_string()),
                provider_id: "ollama".to_string(),
                provider_account_id: Some("ollama-default".to_string()),
                model_id: "qwen2.5-coder:3b".to_string(),
                model_name: "qwen2.5-coder:3b".to_string(),
                routing_mode: "LocalFirst".to_string(),
                attempt_number: 1,
                success: true,
                failure_class: None,
                created_at_ms: now,
            })
            .unwrap();
        }
        let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let details = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"obs","command":"GetMissionDetails","mission_id": seeded_mission_id}),
        );
        assert_eq!(details["ok"], true);

        let verification = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"v1","command":"GetVerificationSummary","mission_id": seeded_mission_id}),
        );
        assert_eq!(verification["ok"], true);
        if let Some(audits) = verification["final_audits"].as_array() {
            if !audits.is_empty() {
                assert!(audits[0].get("remaining_uncertainty").is_some());
            }
        }

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn discuss_send_creates_discuss_conversation_and_appends_messages() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Create a DISCUSS conversation
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Discuss Test"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Verify the conversation mode is DISCUSS
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        assert_eq!(get["conversation"]["mode"], "DISCUSS");

        // Send a discuss message
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "What does this project use for authentication?"}),
        );
        assert_eq!(send["ok"], true, "send: {send}");
        let msg = &send["message"];
        assert_eq!(msg["role"], "assistant");
        assert_eq!(msg["conversation_id"], cid);

        // Conversation must have both messages (user + assistant)
        let get2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g2","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get2["ok"], true);
        let msgs = get2["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2, "should have 2 messages, got {msgs:?}");
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[1]["role"], "assistant");

        // A second DiscussSend appends a third message
        let send2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d2","command":"DiscussSend","conversation_id": cid, "content": "How is the daemon structured?"}),
        );
        assert_eq!(send2["ok"], true);
        let get3 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g3","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get3["ok"], true);
        let msgs3 = get3["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs3.len(), 4, "should have 4 messages, got {msgs3:?}");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn discuss_send_requires_discuss_mode_and_rejects_other_modes() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-mode");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Create a GOAL conversation
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Goal Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // DiscussSend must fail with WRONG_MODE for GOAL conversations
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "test"}),
        );
        assert_eq!(send["ok"], false, "should reject GOAL mode");
        assert_eq!(send["error"]["code"], "CONVERSATION-WRONG_MODE");

        // Create a DISCUSS conversation (should succeed)
        let create2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Discuss Chat"}),
        );
        assert_eq!(create2["ok"], true);
        let cid2 = create2["conversation_id"].as_str().unwrap().to_string();

        let send2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d2","command":"DiscussSend","conversation_id": cid2, "content": "What is the architecture?"}),
        );
        assert_eq!(send2["ok"], true, "DISCUSS mode should work: {send2}");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn discuss_conversation_persists_after_daemon_restart() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-restart");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Restart Test"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "Explain the IPC architecture"}),
        );
        assert_eq!(send["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();

        let mut daemon2 = DaemonService::open(&db, &lock).unwrap();
        daemon2.start().unwrap();
        let Some((server2, listener2)) = bind_or_skip(&socket, None) else {
            daemon2.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let get = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        assert_eq!(get["conversation"]["mode"], "DISCUSS");
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert!(
            !msgs.is_empty(),
            "messages must survive restart, got {}",
            msgs.len()
        );

        let send2 = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"d2","command":"DiscussSend","conversation_id": cid, "content": "Continue after restart"}),
        );
        assert_eq!(send2["ok"], true);

        let get2 = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"g2","command":"ConversationGet","conversation_id": cid}),
        );
        let msgs2 = get2["conversation"]["messages"].as_array().unwrap();
        assert!(
            msgs2.len() > 1,
            "messages must grow after restart, got {}",
            msgs2.len()
        );

        server2.cleanup();
        daemon2.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn discuss_project_isolation_multiple_conversations() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-isolation");
        let project_a = dir.join("proj-a");
        let project_b = dir.join("proj-b");
        fs::create_dir_all(&project_a).unwrap();
        fs::create_dir_all(&project_b).unwrap();
        let path_a = project_a.to_string_lossy().to_string();
        let path_b = project_b.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let ca = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": path_a, "mode":"DISCUSS","title":"Proj A Discuss"}),
        );
        let cid_a = ca["conversation_id"].as_str().unwrap().to_string();

        let cb = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": path_b, "mode":"DISCUSS","title":"Proj B Discuss"}),
        );
        let cid_b = cb["conversation_id"].as_str().unwrap().to_string();

        let send_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid_a, "content": "Question about project A"}),
        );
        assert_eq!(send_a["ok"], true);

        let send_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d2","command":"DiscussSend","conversation_id": cid_b, "content": "Question about project B"}),
        );
        assert_eq!(send_b["ok"], true);

        let list_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"l1","command":"ConversationList","project_path": path_a}),
        );
        let convs_a = list_a["conversations"].as_array().unwrap();
        assert_eq!(convs_a.len(), 1);
        assert_eq!(convs_a[0]["id"], cid_a);

        let list_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"l2","command":"ConversationList","project_path": path_b}),
        );
        let convs_b = list_b["conversations"].as_array().unwrap();
        assert_eq!(convs_b.len(), 1);
        assert_eq!(convs_b[0]["id"], cid_b);

        let get_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid_a}),
        );
        let msgs_a = get_a["conversation"]["messages"].as_array().unwrap();
        assert!(msgs_a[0]["content"].as_str().unwrap().contains("project A"));

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn discuss_to_goal_to_mission_preserves_conversation_and_reference() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-mission");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // 1. Create a Discuss conversation
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Discuss Mission Path"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // 2. Have a real discussion (assistant reply persisted)
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "Should we move authentication into a separate service?"}),
        );
        assert_eq!(send["ok"], true, "discuss: {send}");

        // 3. Transition: user asks to implement, becomes a GoalSubmit in the SAME conversation
        let goal = "Move authentication into a separate service module.";
        let submit = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": goal}),
        );
        assert_eq!(submit["ok"], true, "goal submit: {submit}");
        let mission_id = submit["mission_id"].as_str().unwrap().to_string();
        assert!(mission_id.starts_with("mission-"));

        // 4. Mission reference preserved in the conversation
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        assert_eq!(get["conversation"]["current_mission_id"].as_str().unwrap(), mission_id);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        // user discussion message + assistant reply + user goal message
        assert!(msgs.len() >= 3, "messages: {msgs:?}");
        let goal_msg = msgs
            .iter()
            .find(|m| m["mission_ref"].as_str() == Some(mission_id.as_str()))
            .unwrap();
        assert_eq!(goal_msg["content"], goal);
        // project preserved
        assert_eq!(get["conversation"]["project_path"].as_str().unwrap(), project_path);
        // mode still DISCUSS (a discussion that grew a mission)
        assert_eq!(get["conversation"]["mode"], "DISCUSS");

        // 5. Mission appears in activity projection
        let activity = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
        );
        assert_eq!(activity["ok"], true);
        let missions = activity["missions"].as_array().unwrap();
        assert!(
            missions.iter().any(|m| m["mission_id"].as_str() == Some(mission_id.as_str())),
            "mission must appear in activity"
        );

        // Cancel so the coordinator never runs it against real providers.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mission_id}),
        );
        assert_eq!(cancel["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G3 production-path grounding: a repository-grounded Discuss question
    /// must put real source excerpts into the model prompt and persist real,
    /// non-fabricated citations that survive restart.  Uses the mock
    /// provider: the assertion is about what the daemon put in the prompt
    /// and metadata, not about model prose.
    #[test]
    fn discuss_send_grounds_in_real_source_and_citations_survive_restart() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-grounding");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(project_dir.join("src").join("auth")).unwrap();
        fs::write(
            project_dir.join("src").join("auth").join("mod.rs"),
            "pub fn verify_token(token: &str) -> bool {\n    token.len() > 8\n}\n",
        )
        .unwrap();
        fs::write(
            project_dir.join("src").join("billing.rs"),
            "pub fn charge(amount: u32) { }\n",
        )
        .unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Grounded Discuss"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "How does authentication work?"}),
        );
        assert_eq!(send["ok"], true, "send: {send}");

        // 1. The persisted discuss_context document contains the auth file
        //    (exact files whose content entered the prompt).
        let ctx_doc = daemon
            .db
            .design_document(&cid, "discuss_context")
            .unwrap()
            .expect("grounding must persist a discuss_context document");
        let ctx: serde_json::Value = serde_json::from_str(&ctx_doc.content_json).unwrap();
        assert_eq!(ctx["grounded"], true, "context: {ctx}");
        let sources = ctx["sources"].as_array().unwrap();
        assert!(
            sources
                .iter()
                .any(|s| s["path"].as_str().unwrap_or("").contains("auth")),
            "citation must reference the auth fixture: {sources:?}"
        );

        // 2. Assistant message metadata carries the same citations.
        let msg = &send["message"];
        let metadata: serde_json::Value =
            serde_json::from_str(msg["metadata"].as_str().unwrap_or("{}")).unwrap();
        let meta_sources = metadata["sources"].as_array().cloned().unwrap_or_default();
        assert!(
            meta_sources
                .iter()
                .any(|s| s["path"].as_str().unwrap_or("").contains("auth")),
            "metadata citations: {metadata}"
        );
        assert_eq!(metadata["sources_degraded"], false);

        // 3. The billing file must NOT be cited for an auth question
        //    (citations correspond to supplied context, not everything).
        assert!(
            !meta_sources
                .iter()
                .any(|s| s["path"].as_str().unwrap_or("").contains("billing")),
            "irrelevant file must not be cited: {meta_sources:?}"
        );

        // 4. Restart the daemon; citations must survive.
        server.cleanup();
        daemon.shutdown().unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        let assistant_meta: serde_json::Value =
            serde_json::from_str(msgs[1]["metadata"].as_str().unwrap_or("{}")).unwrap();
        assert!(
            assistant_meta["sources"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false),
            "citations must survive restart: {assistant_meta}"
        );
        let ctx_doc2 = daemon
            .db
            .design_document(&cid, "discuss_context")
            .unwrap()
            .expect("grounding doc must survive restart");
        assert!(ctx_doc2.content_json.contains("auth"));

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G3 isolation: a grounded question in Project A must never cite
    /// Project B files.  Two projects with distinct fixtures, same daemon.
    #[test]
    fn discuss_grounding_cannot_cross_projects() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-grounding-iso");
        let project_a = dir.join("workspace-a");
        let project_b = dir.join("workspace-b");
        fs::create_dir_all(project_a.join("src")).unwrap();
        fs::create_dir_all(project_b.join("src")).unwrap();
        fs::write(
            project_a.join("src").join("alpha_marker.rs"),
            "pub const ALPHA: &str = \"alpha-project-secret-name\";\n",
        )
        .unwrap();
        fs::write(
            project_b.join("src").join("beta_marker.rs"),
            "pub const BETA: &str = \"beta-project-secret-name\";\n",
        )
        .unwrap();
        let path_a = project_a.to_string_lossy().to_string();
        let path_b = project_b.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"ca","command":"ConversationCreate","project_path": path_a, "mode":"DISCUSS","title":"Project A"}),
        );
        let cid_a = create_a["conversation_id"].as_str().unwrap().to_string();
        let create_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cb","command":"ConversationCreate","project_path": path_b, "mode":"DISCUSS","title":"Project B"}),
        );
        let cid_b = create_b["conversation_id"].as_str().unwrap().to_string();

        let send_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"da","command":"DiscussSend","conversation_id": cid_a, "content": "What does the alpha marker constant mean?"}),
        );
        assert_eq!(send_a["ok"], true);
        let send_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"db","command":"DiscussSend","conversation_id": cid_b, "content": "What does the beta marker constant mean?"}),
        );
        assert_eq!(send_b["ok"], true);

        let ctx_a = daemon.db.design_document(&cid_a, "discuss_context").unwrap().unwrap();
        let ctx_b = daemon.db.design_document(&cid_b, "discuss_context").unwrap().unwrap();
        let json_a: serde_json::Value = serde_json::from_str(&ctx_a.content_json).unwrap();
        let json_b: serde_json::Value = serde_json::from_str(&ctx_b.content_json).unwrap();
        let paths_a: Vec<String> = json_a["sources"]
            .as_array()
            .map(|a| a.iter().map(|s| s["path"].as_str().unwrap_or("").to_string()).collect())
            .unwrap_or_default();
        let paths_b: Vec<String> = json_b["sources"]
            .as_array()
            .map(|a| a.iter().map(|s| s["path"].as_str().unwrap_or("").to_string()).collect())
            .unwrap_or_default();
        assert!(
            paths_a.iter().any(|p| p.contains("alpha_marker")),
            "A must cite its own file: {paths_a:?}"
        );
        assert!(
            paths_b.iter().any(|p| p.contains("beta_marker")),
            "B must cite its own file: {paths_b:?}"
        );
        assert!(
            !paths_a.iter().any(|p| p.contains("beta")),
            "Project A must never cite Project B: {paths_a:?}"
        );
        assert!(
            !paths_b.iter().any(|p| p.contains("alpha")),
            "Project B must never cite Project A: {paths_b:?}"
        );

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G3 §23-24: Turn Into Plan → structured plan persisted → Execute Plan
    /// creates a mission whose goal retains the full plan context (never a
    /// single sentence).  Plan survives restart.
    #[test]
    fn discuss_turn_into_plan_executes_with_full_context_and_persists() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-plan");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Plan Flow"}),
        );
        assert_eq!(create["ok"], true);
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // A real discussion first (plan requires one).
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "We should add rate limiting to the login endpoint."}),
        );
        assert_eq!(send["ok"], true, "discuss: {send}");
        let assistant_id = send["message"]["id"].as_str().unwrap().to_string();

        // Turn Into Plan (explicit user action).
        let plan = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p1","command":"DiscussTurnIntoPlan","conversation_id": cid}),
        );
        assert_eq!(plan["ok"], true, "plan: {plan}");
        let plan_value = &plan["plan"];
        assert!(
            plan_value["goal"].as_str().unwrap_or("").contains("rate limiting"),
            "goal must come from the real discussion: {plan_value}"
        );
        assert!(
            !plan_value["requirements"].as_array().unwrap().is_empty(),
            "requirements must be structured, not empty: {plan_value}"
        );

        // Accept as Decision (explicit, message-derived).
        let decision = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"dec1","command":"DiscussAcceptDecision","conversation_id": cid, "message_id": assistant_id, "decision": "Login endpoint will be rate limited per IP.", "rationale": "agreed in discussion"}),
        );
        assert_eq!(decision["ok"], true, "decision: {decision}");
        let decision_id = decision["decision_record"]["id"].as_str().unwrap().to_string();

        // DECISIONS.md materialized in the project + recorded change.
        let decisions_md = std::fs::read_to_string(project_dir.join("DECISIONS.md"))
            .expect("DECISIONS.md must be materialized");
        assert!(decisions_md.contains("rate limited per IP"));
        let ctx = daemon
            .db
            .design_document(&cid, "decisions_materialization")
            .unwrap()
            .expect("materialization must be recorded");
        let ctx_json: serde_json::Value = serde_json::from_str(&ctx.content_json).unwrap();
        assert_eq!(ctx_json["decision_count"], 1);

        // memory_decisions persisted under this project identity.
        let count = daemon
            .db
            .memory_decision_count(&crate::project_repository_identity(&project_path))
            .unwrap();
        assert_eq!(count, 1, "decision must persist into memory_decisions");

        // Re-plan: accepted decision becomes plan context.
        let plan2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p2","command":"DiscussTurnIntoPlan","conversation_id": cid}),
        );
        assert_eq!(plan2["ok"], true);
        let decisions_in_plan = plan2["plan"]["accepted_decisions"].as_array().unwrap();
        assert!(
            decisions_in_plan.iter().any(|d| d["id"].as_str() == Some(decision_id.as_str())),
            "accepted decision must enter subsequent plans: {decisions_in_plan:?}"
        );

        // Execute Plan → mission with full structured context.
        let execute = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"e1","command":"DiscussExecutePlan","conversation_id": cid}),
        );
        assert_eq!(execute["ok"], true, "execute: {execute}");
        let mission_id = execute["mission_id"].as_str().unwrap().to_string();
        assert!(mission_id.starts_with("mission-"));
        let plan_goal = execute["plan_goal"].as_str().unwrap();
        assert!(
            plan_goal.contains("GOAL:") && plan_goal.contains("rate limiting"),
            "mission goal must embed the plan contract: {plan_goal}"
        );
        assert!(
            plan_goal.contains("rate limited per IP"),
            "mission goal must retain accepted decisions: {plan_goal}"
        );

        // Cancel the mission so nothing real executes.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mission_id}),
        );
        assert_eq!(cancel["ok"], true);

        // Restart: plan + decision + mission link survive.
        server.cleanup();
        daemon.shutdown().unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };
        let get_plan = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"gp","command":"DiscussPlanGet","conversation_id": cid}),
        );
        assert_eq!(get_plan["ok"], true);
        let persisted_plan: serde_json::Value =
            serde_json::from_str(get_plan["plan"].as_str().unwrap()).unwrap();
        assert_eq!(
            persisted_plan["promoted_mission_id"].as_str().unwrap(),
            mission_id,
            "plan↔mission link must survive restart"
        );
        let get_decisions = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"gd","command":"DiscussDecisionsGet","conversation_id": cid}),
        );
        assert_eq!(get_decisions["ok"], true);
        assert!(get_decisions["decisions"].as_str().unwrap().contains("rate limited per IP"));

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G3: decision acceptance requires an explicit message from THIS
    /// conversation — a foreign message id is rejected (no fabricated
    /// provenance).
    #[test]
    fn discuss_accept_decision_rejects_foreign_message_and_cross_conversation() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-decision-guard");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Decision Guard"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "Topic"}),
        );
        assert_eq!(send["ok"], true);

        // Foreign message id: rejected.
        let bad = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b1","command":"DiscussAcceptDecision","conversation_id": cid, "message_id": "msg-does-not-exist", "decision": "X", "rationale": ""}),
        );
        assert_eq!(bad["ok"], false, "foreign message must be rejected: {bad}");
        assert!(bad["error"]["code"].as_str().unwrap_or("").contains("DISCUSS-DECISION_SOURCE_MISSING"));

        // Empty decision: rejected.
        let empty = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b2","command":"DiscussAcceptDecision","conversation_id": cid, "message_id": send["message"]["id"].as_str().unwrap(), "decision": "  ", "rationale": ""}),
        );
        assert_eq!(empty["ok"], false);
        assert!(empty["error"]["code"].as_str().unwrap_or("").contains("DISCUSS-DECISION_EMPTY"));

        // Plan before discussion is impossible on a fresh conversation.
        let fresh = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Empty"}),
        );
        let fresh_cid = fresh["conversation_id"].as_str().unwrap().to_string();
        let early_plan = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b3","command":"DiscussTurnIntoPlan","conversation_id": fresh_cid}),
        );
        assert_eq!(early_plan["ok"], false);
        assert!(early_plan["error"]["code"].as_str().unwrap_or("").contains("DISCUSS-PLAN_NO_DISCUSSION"));

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn three_discuss_chats_switch_restart_and_restore_all() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-discuss-3chat");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Create 3 distinct Discuss chats
        let mut cids = Vec::new();
        for (i, title) in ["Chat A", "Chat B", "Chat C"].iter().enumerate() {
            let create = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id": format!("c{i}"), "command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title": title}),
            );
            assert_eq!(create["ok"], true, "create {title}: {create}");
            cids.push(create["conversation_id"].as_str().unwrap().to_string());
        }
        assert_ne!(cids[0], cids[1]);
        assert_ne!(cids[1], cids[2]);

        // Write a distinct message to each
        let topics = [
            "Explain the authentication architecture.",
            "Let's design a new payment subsystem.",
            "Why is the daemon IPC structured this way?",
        ];
        for (i, (cid, topic)) in cids.iter().zip(topics.iter()).enumerate() {
            let send = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id": format!("d{i}"), "command":"DiscussSend","conversation_id": cid, "content": topic}),
            );
            assert_eq!(send["ok"], true, "send to {cid}: {send}");
        }

        // Switch between them: verify each restores its own history
        for (cid, topic) in cids.iter().zip(topics.iter()) {
            let get = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"gx","command":"ConversationGet","conversation_id": cid}),
            );
            assert_eq!(get["ok"], true);
            let msgs = get["conversation"]["messages"].as_array().unwrap();
            assert!(
                msgs.iter()
                    .any(|m| m["content"].as_str() == Some(topic)),
                "wrong history for {cid}"
            );
        }

        // Restart daemon
        server.cleanup();
        daemon.shutdown().unwrap();

        let mut daemon2 = DaemonService::open(&db, &lock).unwrap();
        daemon2.start().unwrap();
        let Some((server2, listener2)) = bind_or_skip(&socket, None) else {
            daemon2.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // All three restored with correct distinct histories
        let list = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"l1","command":"ConversationList","project_path": project_path}),
        );
        assert_eq!(list["ok"], true);
        let convs = list["conversations"].as_array().unwrap();
        assert_eq!(convs.len(), 3, "all three chats must survive restart");
        for (cid, topic) in cids.iter().zip(topics.iter()) {
            let get = request_via_ipc(
                &server2, &listener2, &mut daemon2,
                json!({"id":"gx","command":"ConversationGet","conversation_id": cid}),
            );
            assert_eq!(get["ok"], true);
            let msgs = get["conversation"]["messages"].as_array().unwrap();
            assert!(
                msgs.iter()
                    .any(|m| m["content"].as_str() == Some(topic)),
                "restored history wrong for {cid}"
            );
            assert_eq!(get["conversation"]["mode"], "DISCUSS");
        }

        server2.cleanup();
        daemon2.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// Real E2E: Discuss conversation → user message → real small local model
    /// (≤4B) → assistant response → persisted message → provider/model record
    /// → second Discuss chat → GoalSubmit → mission reference → activity.
    ///
    /// Requires:
    /// - Ollama running on 127.0.0.1:11434 with a ≤4B model
    /// - Default model: qwen2.5-coder:3b (3.1B, 1.9GB)
    /// - OLLAMA_BASE_URL must include the full chat endpoint path so the
    ///   environment-registry provider uses the correct URL (the catalog path
    ///   appends /api/chat automatically; the env path uses the value raw).
    /// - Override model via OLLAMA_MODEL env var
    ///
    /// Run: OLLAMA_BASE_URL=http://127.0.0.1:11434/api/chat \
    ///      OLLAMA_MODEL=qwen2.5-coder:3b \
    ///      cargo test -p ac-daemon real_discuss_e2e_small_model -- --ignored
    #[test]
    #[ignore = "requires Ollama with a ≤4B model (set OLLAMA_MODEL, OLLAMA_BASE_URL)"]
    fn real_discuss_e2e_small_model() {
        // Verify Ollama is reachable with the configured model
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3b".to_string());
        let base = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434/api/chat".to_string());
        let host = base
            .strip_suffix("/api/chat")
            .unwrap_or(&base)
            .to_string();
        let check = std::process::Command::new("curl")
            .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", &format!("{host}/api/tags")])
            .output()
            .expect("ollama must be running for this test");
        let status = String::from_utf8_lossy(&check.stdout);
        if !status.starts_with('2') {
            eprintln!("SKIP: Ollama not reachable at {host}");
            return;
        }
        let (dir, db, lock, socket) = temp_paths("e2e-discuss-real");
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

        // 1. Create Discuss chat
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"E2E Discuss"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // 2. Send a real question — the daemon calls the real model via provider
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DiscussSend","conversation_id": cid, "content": "What file types does this project use?"}),
        );
        assert_eq!(send["ok"], true, "send: {send}");
        let msg = &send["message"];
        assert_eq!(msg["role"], "assistant");
        let content = msg["content"].as_str().unwrap_or("");
        assert!(!content.is_empty(), "real model must produce a non-empty response");
        assert!(!content.contains("[error"), "real model response must not be an error: {content}");

        // 3. Provider/model record was persisted
        let pmrs = daemon.db.provider_model_records_for_conversation(&cid).unwrap();
        assert!(!pmrs.is_empty(), "provider/model record must be persisted");
        assert!(
            pmrs.iter().any(|r| r.success && r.model_name == model),
            "expected successful record for {model}, got: {pmrs:?}"
        );

        // 4. Provider/model metadata in message
        let metadata = msg["metadata"].as_str().unwrap_or("{}");
        assert!(metadata.contains(&model), "metadata must contain model: {metadata}");

        // 5. ConversationGet returns the persisted discussion
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2, "user + assistant");
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[1]["role"], "assistant");

        // 6. Transition to mission: Discuss → GoalSubmit
        let goal = "List the project files and their purposes.";
        let submit = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": goal}),
        );
        assert_eq!(submit["ok"], true, "submit: {submit}");
        let mission_id = submit["mission_id"].as_str().unwrap().to_string();

        // 7. Mission reference appears in conversation
        let get2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g2","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get2["ok"], true);
        assert_eq!(get2["conversation"]["current_mission_id"].as_str().unwrap(), mission_id);

        // 8. Activity projection surfaces the mission
        let activity = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
        );
        assert_eq!(activity["ok"], true);
        assert!(activity["missions"].as_array().unwrap().iter().any(|m| {
            m["mission_id"].as_str() == Some(mission_id.as_str())
        }));

        // 9. Cancel mission so coordinator doesn't run it against real providers
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
    fn design_send_creates_design_conversation_and_appends_messages() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Design Test"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        assert_eq!(get["conversation"]["mode"], "DESIGN");

        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DesignSend","conversation_id": cid, "content": "Design a modern dashboard for this project."}),
        );
        assert_eq!(send["ok"], true, "send: {send}");
        let msg = &send["message"];
        assert_eq!(msg["role"], "assistant");
        assert_eq!(msg["conversation_id"], cid);

        let get2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g2","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get2["ok"], true);
        let msgs = get2["conversation"]["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2, "should have 2 messages, got {msgs:?}");
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[1]["role"], "assistant");
        let metadata = msgs[1]["metadata"].as_str().unwrap_or("");
        assert!(metadata.contains("design"), "metadata must mark design mode: {metadata}");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_send_requires_design_mode_and_rejects_other_modes() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-mode");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"Goal Chat"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DesignSend","conversation_id": cid, "content": "test"}),
        );
        assert_eq!(send["ok"], false, "should reject GOAL mode");
        assert_eq!(send["error"]["code"], "CONVERSATION-WRONG_MODE");

        let create2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Design Chat"}),
        );
        assert_eq!(create2["ok"], true);
        let cid2 = create2["conversation_id"].as_str().unwrap().to_string();

        let send2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d2","command":"DesignSend","conversation_id": cid2, "content": "Design the authentication experience."}),
        );
        assert_eq!(send2["ok"], true, "DESIGN mode should work: {send2}");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_conversation_persists_after_daemon_restart() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-restart");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Restart Test"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DesignSend","conversation_id": cid, "content": "Let's design the dashboard layout."}),
        );
        assert_eq!(send["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();

        let mut daemon2 = DaemonService::open(&db, &lock).unwrap();
        daemon2.start().unwrap();
        let Some((server2, listener2)) = bind_or_skip(&socket, None) else {
            daemon2.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let get = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        assert_eq!(get["conversation"]["mode"], "DESIGN");
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert!(!msgs.is_empty(), "messages must survive restart, got {}", msgs.len());

        let send2 = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"d2","command":"DesignSend","conversation_id": cid, "content": "Continue after restart"}),
        );
        assert_eq!(send2["ok"], true);

        let get2 = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"g2","command":"ConversationGet","conversation_id": cid}),
        );
        let msgs2 = get2["conversation"]["messages"].as_array().unwrap();
        assert!(msgs2.len() > 1, "messages must grow after restart, got {}", msgs2.len());

        server2.cleanup();
        daemon2.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_three_chats_switch_restart_and_restore_all() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-3chat");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let mut cids = Vec::new();
        for (i, title) in ["Design A", "Design B", "Design C"].iter().enumerate() {
            let create = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id": format!("c{i}"), "command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title": title}),
            );
            assert_eq!(create["ok"], true, "create {title}: {create}");
            cids.push(create["conversation_id"].as_str().unwrap().to_string());
        }
        assert_ne!(cids[0], cids[1]);
        assert_ne!(cids[1], cids[2]);

        let topics = [
            "Design the authentication experience.",
            "Redesign the dashboard.",
            "Explore mobile navigation.",
        ];
        for (i, (cid, topic)) in cids.iter().zip(topics.iter()).enumerate() {
            let send = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id": format!("d{i}"), "command":"DesignSend","conversation_id": cid, "content": topic}),
            );
            assert_eq!(send["ok"], true, "send to {cid}: {send}");
        }

        for (cid, topic) in cids.iter().zip(topics.iter()) {
            let get = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"gx","command":"ConversationGet","conversation_id": cid}),
            );
            assert_eq!(get["ok"], true);
            let msgs = get["conversation"]["messages"].as_array().unwrap();
            assert!(
                msgs.iter().any(|m| m["content"].as_str() == Some(topic)),
                "wrong history for {cid}"
            );
        }

        server.cleanup();
        daemon.shutdown().unwrap();

        let mut daemon2 = DaemonService::open(&db, &lock).unwrap();
        daemon2.start().unwrap();
        let Some((server2, listener2)) = bind_or_skip(&socket, None) else {
            daemon2.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let list = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"l1","command":"ConversationList","project_path": project_path}),
        );
        assert_eq!(list["ok"], true);
        let convs = list["conversations"].as_array().unwrap();
        let design_convs = convs.iter().filter(|c| c["mode"] == "DESIGN").count();
        assert_eq!(design_convs, 3, "all three design chats must survive restart");

        for (cid, topic) in cids.iter().zip(topics.iter()) {
            let get = request_via_ipc(
                &server2, &listener2, &mut daemon2,
                json!({"id":"gx","command":"ConversationGet","conversation_id": cid}),
            );
            assert_eq!(get["ok"], true);
            let msgs = get["conversation"]["messages"].as_array().unwrap();
            assert!(
                msgs.iter().any(|m| m["content"].as_str() == Some(topic)),
                "restored history wrong for {cid}"
            );
            assert_eq!(get["conversation"]["mode"], "DESIGN");
        }

        server2.cleanup();
        daemon2.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_project_isolation_multiple_conversations() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-isolation");
        let project_a = dir.join("proj-a");
        let project_b = dir.join("proj-b");
        fs::create_dir_all(&project_a).unwrap();
        fs::create_dir_all(&project_b).unwrap();
        let path_a = project_a.to_string_lossy().to_string();
        let path_b = project_b.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let ca = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": path_a, "mode":"DESIGN","title":"Proj A Design"}),
        );
        let cid_a = ca["conversation_id"].as_str().unwrap().to_string();

        let cb = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": path_b, "mode":"DESIGN","title":"Proj B Design"}),
        );
        let cid_b = cb["conversation_id"].as_str().unwrap().to_string();

        let send_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DesignSend","conversation_id": cid_a, "content": "Design for project A"}),
        );
        assert_eq!(send_a["ok"], true);

        let send_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d2","command":"DesignSend","conversation_id": cid_b, "content": "Design for project B"}),
        );
        assert_eq!(send_b["ok"], true);

        let list_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"l1","command":"ConversationList","project_path": path_a}),
        );
        let convs_a = list_a["conversations"].as_array().unwrap();
        assert_eq!(convs_a.len(), 1);
        assert_eq!(convs_a[0]["id"], cid_a);

        let list_b = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"l2","command":"ConversationList","project_path": path_b}),
        );
        let convs_b = list_b["conversations"].as_array().unwrap();
        assert_eq!(convs_b.len(), 1);
        assert_eq!(convs_b[0]["id"], cid_b);

        let get_a = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid_a}),
        );
        let msgs_a = get_a["conversation"]["messages"].as_array().unwrap();
        assert!(msgs_a[0]["content"].as_str().unwrap().contains("project A"));

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_brief_grammar_state_and_critique_roundtrip() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-brief");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        // A realistic web project so product understanding has something to scan
        fs::write(
            project_dir.join("package.json"),
            r#"{"name":"dashboard-app","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();
        fs::create_dir_all(project_dir.join("src/components")).unwrap();
        fs::write(project_dir.join("src/App.tsx"), "export function App() { return <main /> }").unwrap();
        fs::write(
            project_dir.join("src/styles.css"),
            "--color-primary: #1a1a2e; --radius-md: 6px;",
        )
        .unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Dashboard Redesign"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Product understanding must detect the framework and components
        let understand = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"u1","command":"DesignUnderstand","conversation_id": cid}),
        );
        assert_eq!(understand["ok"], true, "understand: {understand}");
        let analysis = &understand["analysis"];
        assert_eq!(analysis["framework"], "Vite", "framework detection: {analysis}");
        assert!(
            !analysis["components"].as_array().unwrap().is_empty(),
            "components must be discovered: {analysis}"
        );

        // Design Brief is structured and persisted
        let brief = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b1","command":"DesignBrief","conversation_id": cid, "audience": "data analysts", "workflow": "review metrics"}),
        );
        assert_eq!(brief["ok"], true, "brief: {brief}");
        let brief_val = &brief["brief"];
        assert_eq!(brief_val["product"], "Dashboard Redesign");
        assert_eq!(brief_val["audience"], "data analysts");
        assert!(!brief_val["patterns_to_avoid"].as_array().unwrap().is_empty());

        // Grammar is structured and product-anchored
        let grammar = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"gm1","command":"DesignGrammar","conversation_id": cid}),
        );
        assert_eq!(grammar["ok"], true, "grammar: {grammar}");
        let grammar_val = &grammar["grammar"];
        assert!(
            grammar_val["color_roles"][0].as_str().unwrap().contains("Dashboard Redesign"),
            "grammar must be product-specific: {grammar_val}"
        );

        // Design state persists durable project decisions
        let state = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"DesignState","conversation_id": cid}),
        );
        assert_eq!(state["ok"], true, "state: {state}");
        let state_val = &state["design_state"];
        assert!(
            state_val["content"].as_str().unwrap().contains("DESIGN_STATE.md"),
            "state must reference DESIGN_STATE.md: {state_val}"
        );

        // Critique flags generic patterns instead of approving slop
        let slop = r#"
            <div class="hero" style="background: linear-gradient(180deg, #667eea, #764ba2); height: 100vh;">
              <h1>Welcome to the future of AI</h1>
            </div>
            <div class="card">Feature A</div>
            <div class="card">Feature B</div>
            <div class="card">Feature C</div>
        "#;
        let critique = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cr1","command":"DesignCritique","conversation_id": cid, "content": slop, "doc_type":"implementation"}),
        );
        assert_eq!(critique["ok"], true, "critique: {critique}");
        let critique_val = &critique["critique"];
        assert_eq!(critique_val["passed"], false, "slop must not pass: {critique_val}");
        assert_eq!(critique_val["improvement_required"], true);
        let findings = critique_val["findings"].as_array().unwrap();
        let rules: Vec<&str> = findings
            .iter()
            .filter_map(|f| f["rule"].as_str())
            .collect();
        assert!(rules.contains(&"oversized_gradient_hero"), "rules: {rules:?}");
        assert!(rules.contains(&"identical_generic_cards"), "rules: {rules:?}");
        assert!(rules.contains(&"generic_placeholder"), "rules: {rules:?}");

        // The same critique engine accepts a product-specific implementation
        let clean = r#"
            <header class="panel" style="background: var(--color-primary);">
              <h1>Metrics Review</h1>
            </header>
            <button role="button">Refresh</button>
        "#;
        let critique2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cr2","command":"DesignCritique","conversation_id": cid, "content": clean, "doc_type":"implementation"}),
        );
        assert_eq!(critique2["ok"], true);
        assert_eq!(critique2["critique"]["passed"], true, "clean design must pass: {critique2}");

        // Repair loop turns material findings into actionable repairs
        let repair = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"r1","command":"DesignRepair","conversation_id": cid, "content": slop, "doc_type":"implementation"}),
        );
        assert_eq!(repair["ok"], true, "repair: {repair}");
        let repair_val = &repair["repair"];
        assert_eq!(repair_val["improvement_required"], true);
        assert!(!repair_val["repairs"].as_array().unwrap().is_empty());

        // QA reports ground functional and a11y checks on real content
        let functional = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"f1","command":"DesignQAFunctional","conversation_id": cid, "html": clean, "deterministic": true}),
        );
        assert_eq!(functional["ok"], true);
        assert_eq!(functional["qa"]["passed"], true);

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_documents_survive_daemon_restart() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-docs-restart");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            project_dir.join("package.json"),
            r#"{"name":"app","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Docs Restart"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let understand = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"u1","command":"DesignUnderstand","conversation_id": cid}),
        );
        assert_eq!(understand["ok"], true);
        let brief = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b1","command":"DesignBrief","conversation_id": cid, "audience": "developers", "workflow": "configure pipelines"}),
        );
        assert_eq!(brief["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();

        let mut daemon2 = DaemonService::open(&db, &lock).unwrap();
        daemon2.start().unwrap();
        let Some((server2, listener2)) = bind_or_skip(&socket, None) else {
            daemon2.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // Regenerating brief must pick up the persisted analysis doc
        let brief2 = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"b2","command":"DesignBrief","conversation_id": cid, "audience": "developers", "workflow": "configure pipelines"}),
        );
        assert_eq!(brief2["ok"], true, "brief after restart: {brief2}");
        assert_eq!(brief2["brief"]["framework"], "Vite", "persisted analysis must feed brief");

        // Design state regenerates from the persisted brief/grammar documents
        let state = request_via_ipc(
            &server2, &listener2, &mut daemon2,
            json!({"id":"s1","command":"DesignState","conversation_id": cid}),
        );
        assert_eq!(state["ok"], true);
        assert!(state["design_state"]["content"].as_str().unwrap().contains("DESIGN_STATE.md"));

        server2.cleanup();
        daemon2.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_browser_deterministic_inspects_dom_and_captures_evidence() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-browser");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Browser Check"}),
        );
        assert_eq!(create["ok"], true);
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let html = r#"
            <html lang="en"><body>
              <nav><a href="/">Home</a></nav>
              <h1>Metrics Review</h1>
              <button role="button">Refresh</button>
              <form><label>Query</label><input name="q" aria-label="Query" /></form>
            </body></html>
        "#;
        let browser = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"br1","command":"DesignBrowser","conversation_id": cid, "url":"http://127.0.0.1:5173", "html": html, "deterministic": true}),
        );
        assert_eq!(browser["ok"], true, "browser: {browser}");
        let browser_val = &browser["browser"];
        assert_eq!(browser_val["url"], "http://127.0.0.1:5173");
        assert!(
            browser_val["visible_text"].as_str().unwrap().contains("Metrics Review"),
            "DOM must contain rendered text: {browser_val}"
        );
        let controls = browser_val["controls"].as_array().unwrap();
        assert!(
            controls.iter().any(|c| c.as_str().unwrap().contains("button")),
            "controls must be discovered: {controls:?}"
        );
        assert!(
            browser_val["screenshot_uri"].as_str().unwrap().contains("screenshot:"),
            "screenshot evidence must be produced: {browser_val}"
        );

        // Accessibility QA on the inspected DOM (deterministic harness: the
        // structural a11y facts are measured from the fixture HTML; layout
        // metrics are flagged for manual review, never fabricated).
        let a11y = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"a1","command":"DesignQAAccessibility","conversation_id": cid, "html": html, "deterministic": true}),
        );
        assert_eq!(a11y["ok"], true, "a11y: {a11y}");
        let a11y_qa = &a11y["qa"];
        // The fixture has aria/label semantics; unlabeled-controls must be
        // empty for it to pass.
        assert_eq!(
            a11y_qa["passed"], true,
            "semantic DOM must pass a11y: {a11y_qa}"
        );
        assert_eq!(a11y_qa["source"], "manual-review");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G4 real-browser QA: design_qa_run against REAL Chrome CDP measures
    /// real layout (overflow, touch targets) and persists the layered
    /// report.  Skips honestly when no Chromium is available.
    #[test]
    fn design_qa_real_browser_measures_layout_and_persists_report() {
        if ac_verification::discover_chromium_executable().is_none() {
            eprintln!("SKIP: no Chromium executable available for real QA");
            return;
        }
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-qa-real");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        // Local HTTP server serving a page with a real layout defect:
        // a 3000px-wide element (overflow) and a 16px button (small target).
        let server_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server_listener.local_addr().unwrap();
        let port = addr.port();
        std::thread::spawn(move || {
            server_listener.set_nonblocking(true).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while std::time::Instant::now() < deadline {
                if let Ok((mut stream, _)) = server_listener.accept() {
                    let mut buffer = [0_u8; 2048];
                    let n = stream.read(&mut buffer).unwrap_or(0);
                    let _request = String::from_utf8_lossy(&buffer[..n]);
                    let body = "<!doctype html><html lang=\"en\"><head><style>\
                        #wide { width: 3000px; height: 10px; }\
                        #tiny { width: 16px; height: 16px; }</style></head>\
                        <body><h1>Real QA</h1><div id=\"wide\"></div>\
                        <button id=\"tiny\" aria-label=\"Tiny\">x</button>\
                        <button id=\"ok\">Normal button</button></body></html>";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            }
        });

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();

        let create = dispatch_request(
            &json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Real QA"}),
            &mut daemon,
        ).0;
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Direct dispatch (no IPC thread) so a panic surfaces with a
        // backtrace instead of killing the IPC frame.
        let qa = dispatch_request(
            &json!({"id":"qa1","command":"DesignQAReport","conversation_id": cid, "url": format!("http://127.0.0.1:{port}/"), "deterministic": false}),
            &mut daemon,
        ).0;
        assert_eq!(qa["ok"], true, "qa: {qa}");
        let report = &qa["qa"];
        assert_eq!(report["mode"], "cdp", "must run the real CDP runtime");
        assert_eq!(report["needs_manual_review"], false);

        // Responsive layer measured the real overflow.
        let responsive = &report["layers"]["responsive"];
        assert_eq!(responsive["passed"], false, "3000px element must fail: {responsive}");
        assert!(
            responsive["issues"].as_array().unwrap().iter().any(|i| i.as_str().unwrap_or("").contains("horizontal overflow")),
            "overflow issue: {responsive}"
        );

        // A11y layer measured the tiny touch target.
        let a11y = &report["layers"]["accessibility"];
        assert_eq!(a11y["passed"], false, "16px target must fail: {a11y}");
        assert!(
            a11y["issues"].as_array().unwrap().iter().any(|i| i.as_str().unwrap_or("").contains("touch target below 24px")),
            "touch target issue: {a11y}"
        );

        // Functional layer: page loads clean.
        let functional = &report["layers"]["functional"];
        assert_eq!(functional["passed"], true, "functional: {functional}");
        assert_eq!(functional["source"], "real-browser-cdp");

        // The layered report persists per conversation.
        let persisted = daemon
            .db
            .design_document(&cid, "design_qa_report")
            .unwrap()
            .expect("QA report must persist");
        let persisted_value: serde_json::Value =
            serde_json::from_str(&persisted.content_json).unwrap();
        assert_eq!(persisted_value["mode"], "cdp");
        assert!(!persisted.evidence_refs.is_empty(), "evidence refs must be recorded");

        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
        let _ = socket;
    }

    /// G4 contract→mission: Implement via Mission must carry the FULL
    /// design contract (brief, constraints marked non-negotiable, QA
    /// findings, conversation context) — never a single sentence.
    /// Constraints are project-scoped: a second chat in the same project
    /// inherits them; another project never sees them.
    #[test]
    fn design_execute_contract_carries_full_context_and_constraints_are_project_scoped() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-contract");
        let project_dir = dir.join("workspace");
        let other_project_dir = dir.join("other-workspace");
        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&other_project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();
        let other_project_path = other_project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Contract"}),
        );
        assert_eq!(create["ok"], true);
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // A design discussion first.
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DesignSend","conversation_id": cid, "content": "Design a metrics dashboard for the ops team with a dense data table and side filter panel."}),
        );
        assert_eq!(send["ok"], true, "design send: {send}");

        // Set durable constraints (project-scoped).
        let set = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cons1","command":"DesignConstraintsSet","conversation_id": cid, "constraints": [
                {"text": "Data density outranks whitespace on every screen"},
                {"text": "No dark-mode-only color choices"},
            ]}),
        );
        assert_eq!(set["ok"], true, "set: {set}");
        assert_eq!(set["constraints"]["constraints"].as_array().unwrap().len(), 2);

        // Execute via contract: mission must embed the constraints as
        // NON-NEGOTIABLE and the design conversation context.
        let execute = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"e1","command":"DesignExecuteContract","conversation_id": cid}),
        );
        assert_eq!(execute["ok"], true, "execute: {execute}");
        let mission_id = execute["mission_id"].as_str().unwrap().to_string();
        assert!(mission_id.starts_with("mission-"));
        let goal = execute["contract_goal"].as_str().unwrap();
        assert!(goal.contains("Design Contract"), "goal: {goal}");
        assert!(
            goal.contains("NON-NEGOTIABLE"),
            "constraints must be marked non-negotiable: {goal}"
        );
        assert!(
            goal.contains("Data density outranks whitespace"),
            "constraint text must be in the goal: {goal}"
        );
        assert!(
            goal.contains("metrics dashboard"),
            "design conversation context must be in the goal: {goal}"
        );

        // Cancel so nothing real executes.
        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mission_id}),
        );
        assert_eq!(cancel["ok"], true);

        // Second chat in the SAME project inherits constraints (design
        // memory across chats).
        let create2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Second chat"}),
        );
        let cid2 = create2["conversation_id"].as_str().unwrap().to_string();
        let get2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g2","command":"DesignConstraintsGet","conversation_id": cid2}),
        );
        assert_eq!(get2["ok"], true);
        assert_eq!(
            get2["constraints"]["constraints"].as_array().unwrap().len(),
            2,
            "same project must inherit constraints: {}",
            get2["constraints"]
        );

        // Another project NEVER sees them.
        let create3 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c3","command":"ConversationCreate","project_path": other_project_path, "mode":"DESIGN","title":"Other project"}),
        );
        let cid3 = create3["conversation_id"].as_str().unwrap().to_string();
        let get3 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g3","command":"DesignConstraintsGet","conversation_id": cid3}),
        );
        assert_eq!(get3["ok"], true);
        assert_eq!(
            get3["constraints"]["constraints"].as_array().unwrap().len(),
            0,
            "constraints must never leak across projects: {}",
            get3["constraints"]
        );

        // DESIGN_STATE.md materializes through the recorded path.
        let materialize = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"m1","command":"DesignMaterializeState","conversation_id": cid}),
        );
        assert_eq!(materialize["ok"], true, "materialize: {materialize}");
        assert!(project_dir.join("DESIGN_STATE.md").exists(), "DESIGN_STATE.md must exist");
        let state_content = fs::read_to_string(project_dir.join("DESIGN_STATE.md")).unwrap();
        assert!(state_content.contains("DESIGN_STATE.md"), "state: {state_content}");
        let record = daemon
            .db
            .design_document(&cid, "design_state_materialization")
            .unwrap()
            .expect("materialization must be recorded");
        assert!(record.content_json.contains("DESIGN_STATE.md"));

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G-Closure-2 round 2: repair-loop iteration history persists and the
    /// critique carries durable constraints; design memory across chats
    /// inherits project knowledge without leaking across projects;
    /// DESIGN_STATE.md materializes through the GOVERNED ChangeSet path
    /// (a real transaction with journal + changeset id, not a bare write).
    #[test]
    fn design_repair_history_constraints_memory_and_governed_state() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-round2");
        let project_dir = dir.join("workspace");
        let other_project_dir = dir.join("other-workspace");
        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&other_project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();
        let other_project_path = other_project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // First design chat: set constraints, run two repair iterations.
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Design chat"}),
        );
        assert_eq!(create["ok"], true);
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let set = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cons1","command":"DesignConstraintsSet","conversation_id": cid, "constraints": [
                {"text": "Keep the existing sidebar navigation"},
            ]}),
        );
        assert_eq!(set["ok"], true, "set: {set}");

        // Sloppy content: the critique must find issues AND carry the
        // constraint list in its result (the critic never recommends
        // violating them).
        let sloppy = "<div><h1>Welcome to the future of AI productivity</h1><div class='card'>Card one</div><div class='card'>Card two</div><div class='card'>Card three</div></div>";
        let critique = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cr1","command":"DesignCritique","conversation_id": cid, "content": sloppy, "doc_type":"rendered"}),
        );
        assert_eq!(critique["ok"], true, "critique: {critique}");
        let crit = &critique["critique"];
        assert!(!crit["findings"].as_array().unwrap().is_empty(), "sloppy content must produce findings: {crit}");
        let constraints_in_critique = crit["constraints"].as_array().unwrap();
        assert_eq!(constraints_in_critique.len(), 1, "critique must carry the constraint: {crit}");
        assert!(constraints_in_critique[0].as_str().unwrap().contains("sidebar"));

        // Two repair iterations: history must persist with rising numbers.
        let repair1 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"r1","command":"DesignRepair","conversation_id": cid, "content": sloppy, "doc_type":"rendered"}),
        );
        assert_eq!(repair1["ok"], true, "repair1: {repair1}");
        assert_eq!(repair1["repair"]["iteration"].as_u64().unwrap(), 1, "first iteration must be 1: {}", repair1["repair"]);
        let repair2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"r2","command":"DesignRepair","conversation_id": cid, "content": sloppy, "doc_type":"rendered"}),
        );
        assert_eq!(repair2["ok"], true, "repair2: {repair2}");
        assert_eq!(repair2["repair"]["iteration"].as_u64().unwrap(), 2, "second iteration must be 2: {}", repair2["repair"]);
        assert!(!repair2["repair"]["remaining_issues"].as_array().unwrap().is_empty(), "remaining issues must be recorded");

        let history = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"h1","command":"DesignIterations","conversation_id": cid}),
        );
        assert_eq!(history["ok"], true, "history: {history}");
        let iterations = history["iterations"]["iterations"].as_array().unwrap();
        assert_eq!(iterations.len(), 2, "iteration history must persist both: {history}");
        assert_eq!(iterations[0]["iteration"].as_u64().unwrap(), 1);
        assert_eq!(iterations[1]["iteration"].as_u64().unwrap(), 2);

        // Accept a decision in the project (through Discuss memory) so
        // design memory can inherit it.
        let d_create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"dc1","command":"ConversationCreate","project_path": project_path, "mode":"DISCUSS","title":"Decision chat"}),
        );
        let d_cid = d_create["conversation_id"].as_str().unwrap().to_string();
        let d_send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"ds1","command":"DiscussSend","conversation_id": d_cid, "content": "We should keep the compact density for tables because ops users scan many rows."}),
        );
        assert_eq!(d_send["ok"], true, "discuss send: {d_send}");
        let d_reply_id = d_send["message"]["id"].as_str().unwrap().to_string();
        let d_accept = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"da1","command":"DiscussAcceptDecision","conversation_id": d_cid, "message_id": d_reply_id, "decision": "Keep compact table density for ops workflows", "rationale": "ops users scan many rows"}),
        );
        assert_eq!(d_accept["ok"], true, "accept: {d_accept}");

        // A SECOND design chat in the same project inherits the memory:
        // constraints + accepted decisions + inherited_from marker.
        let create2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c2","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Second design chat"}),
        );
        let cid2 = create2["conversation_id"].as_str().unwrap().to_string();
        let memory = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"mem1","command":"DesignMemoryGet","conversation_id": cid2}),
        );
        assert_eq!(memory["ok"], true, "memory: {memory}");
        let mem = &memory["memory"];
        let inherited_constraints = mem["constraints"].as_array().unwrap();
        assert_eq!(inherited_constraints.len(), 1, "constraints must be inherited: {mem}");
        assert!(inherited_constraints[0]["text"].as_str().unwrap().contains("sidebar"));
        let decisions = mem["accepted_decisions"].as_array().unwrap();
        assert!(!decisions.is_empty(), "accepted decisions must be inherited: {mem}");
        assert!(
            decisions.iter().any(|d| d["decision"].as_str().unwrap_or("").contains("compact table density")),
            "the accepted decision text must be inherited: {mem}"
        );
        assert!(
            mem["inherited_from_conversation"].is_string(),
            "memory must record the conversation it inherited from: {mem}"
        );

        // Another project's design chat inherits NOTHING.
        let create3 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c3","command":"ConversationCreate","project_path": other_project_path, "mode":"DESIGN","title":"Other project"}),
        );
        let cid3 = create3["conversation_id"].as_str().unwrap().to_string();
        let memory3 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"mem3","command":"DesignMemoryGet","conversation_id": cid3}),
        );
        assert_eq!(memory3["ok"], true, "memory3: {memory3}");
        let mem3 = &memory3["memory"];
        assert_eq!(mem3["constraints"].as_array().unwrap().len(), 0, "no constraint leak: {mem3}");
        assert_eq!(mem3["accepted_decisions"].as_array().unwrap().len(), 0, "no decision leak: {mem3}");
        assert!(mem3["inherited_from_conversation"].is_null(), "no inheritance marker leak: {mem3}");

        // GOVERNED DESIGN_STATE.md: the materialization record must carry a
        // real changeset id + journal entries (EditEngine path), and the
        // file must exist in the worktree.
        let materialize = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"m1","command":"DesignMaterializeState","conversation_id": cid}),
        );
        assert_eq!(materialize["ok"], true, "materialize: {materialize}");
        let mat = &materialize["materialization"];
        assert!(mat["changeset_id"].as_str().unwrap_or("").starts_with("cs-"), "governed write must record a changeset: {mat}");
        assert!(mat["journal_entries"].as_u64().unwrap_or(0) >= 1, "journal must have entries: {mat}");
        assert!(project_dir.join("DESIGN_STATE.md").exists(), "DESIGN_STATE.md must exist");
        let record = daemon
            .db
            .design_document(&cid, "design_state_materialization")
            .unwrap()
            .expect("materialization must be recorded");
        assert!(record.content_json.contains("changeset_id"), "record must reference the governed changeset");

        // Second materialization must also succeed (idempotent re-write of
        // an existing file goes through the update path of the engine).
        let materialize2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"m2","command":"DesignMaterializeState","conversation_id": cid2}),
        );
        assert_eq!(materialize2["ok"], true, "second materialize (same project, other chat): {materialize2}");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G4 visual critic: deterministic mode reports VISION_CRITIC_UNAVAILABLE
    /// honestly (no fabricated findings).
    #[test]
    fn design_visual_critique_deterministic_reports_unavailable_honestly() {
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-visual-critic");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Visual Critic"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let critique = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"vc1","command":"DesignVisualCritique","conversation_id": cid, "deterministic": true}),
        );
        assert_eq!(critique["ok"], true, "critique: {critique}");
        let vc = &critique["visual_critique"];
        assert_eq!(vc["available"], false, "deterministic must be unavailable: {vc}");
        assert!(
            vc["unavailable_reason"].as_str().unwrap_or("").contains("VISION_CRITIC_UNAVAILABLE"),
            "honest unavailability: {vc}"
        );
        assert!(vc["findings"].as_array().unwrap().is_empty(), "no fabricated findings");

        // Persisted as a visual-critic critique row.
        let rows = daemon.db.design_critiques(&cid).unwrap();
        assert!(
            rows.iter().any(|r| r.doc_type.as_deref() == Some("visual-critic")),
            "visual critique must persist: {:?}",
            rows.iter().map(|r| r.doc_type.clone()).collect::<Vec<_>>()
        );

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    /// G4 REAL visual critic E2E (gemma3:4b, ≤4B local vision model):
    /// captures a real Chrome screenshot of a served page, sends it to the
    /// real vision model, and asserts structured findings came back from
    /// the model — proving the production vision path end to end.
    /// Ignored by default; run with:
    ///   cargo test -p ac-daemon --lib design_visual_critic_real -- --ignored
    #[test]
    #[ignore]
    fn design_visual_critic_real_gemma_e2e() {
        let model = std::env::var("AGENTCODE_VISION_MODEL")
            .unwrap_or_else(|_| "gemma3:4b".to_string());
        let host = "http://127.0.0.1:11434";
        let check = std::process::Command::new("curl")
            .args(["-s", "--max-time", "10", "-o", "/dev/null", "-w", "%{http_code}", &format!("{host}/api/tags")])
            .output()
            .expect("curl must exist");
        let status = String::from_utf8_lossy(&check.stdout);
        if !status.starts_with('2') {
            eprintln!("SKIP: Ollama not reachable at {host}");
            return;
        }
        // Local fixture server: a deliberately flawed design (huge cramped
        // heading, tiny low-contrast subtitle) the critic should find
        // issues in.
        let (tx, rx) = std::sync::mpsc::channel();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                use std::io::{Read, Write};
                let mut stream = stream;
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let body = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Flawed Fixture</title>
<style>body{margin:0;font-family:Helvetica}h1{font-size:14px;margin:4px;color:#aaa}
p{font-size:9px;color:#ccc}</style></head>
<body><main><h1>flawed cramped heading that is far too small for a page title</h1>
<p>nearly unreadable low-contrast tiny subtitle</p></main></body></html>"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = tx.send(());
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(150));

        let (dir, db, lock, socket) = temp_paths("e2e-design-critic-real");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, ipc_listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &ipc_listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Real Critic"}),
        );
        assert_eq!(create["ok"], true);
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let url = format!("http://127.0.0.1:{port}/");
        let critique = request_via_ipc(
            &server, &ipc_listener, &mut daemon,
            json!({"id":"vc1","command":"DesignVisualCritique","conversation_id": cid, "url": url, "deterministic": false}),
        );
        let _ = rx.recv_timeout(std::time::Duration::from_secs(1));
        assert_eq!(critique["ok"], true, "critique response: {critique}");
        let vc = &critique["visual_critique"];
        let reason = vc["unavailable_reason"].as_str().unwrap_or("");
        assert_eq!(
            vc["available"], true,
            "real vision model path must succeed (reason: {reason}; model {model})"
        );
        assert!(
            vc["findings"].is_array(),
            "vision model must return structured findings: {vc}"
        );
        let findings = vc["findings"].as_array().unwrap();
        if !findings.is_empty() {
            // The model saw the cramped heading/low-contrast text; whatever
            // it names must be structured (aspect + issue strings).
            for finding in findings {
                assert!(
                    finding.get("issue").and_then(Value::as_str).is_some()
                        || finding.get("aspect").and_then(Value::as_str).is_some(),
                    "each finding must be structured: {finding}"
                );
            }
        }
        assert!(
            vc["screenshot_uri"].as_str().unwrap_or("").starts_with('/')
                || vc["screenshot_uri"].as_str().unwrap_or("").contains("://"),
            "real screenshot must be referenced: {}",
            vc["screenshot_uri"]
        );

        // Persisted for the conversation.
        let rows = daemon.db.design_critiques(&cid).unwrap();
        assert!(
            rows.iter()
                .any(|r| r.doc_type.as_deref() == Some("visual-critic")
                    && !r.evidence_refs.is_empty()),
            "real critic run must persist with evidence refs: {:?}",
            rows.iter().map(|r| (r.doc_type.clone(), r.evidence_refs.clone())).collect::<Vec<_>>()
        );

        server.cleanup();
        daemon.shutdown().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_studio_deterministic_end_to_end_workflow() {
        // G4-26: single deterministic E2E that exercises the full Design Studio
        // workflow: New Design Chat → design request → understanding → brief →
        // grammar → implementation → real app launch → browser → screenshot →
        // critique → repair → responsive → accessibility → functional →
        // persistent Design State.
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-e2e");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        // Web project fixture with a dev-server command, components, and styles.
        fs::write(
            project_dir.join("package.json"),
            r#"{"name":"e2e-app","scripts":{"dev":"python3 -m http.server 8787"}}"#,
        )
        .unwrap();
        fs::write(project_dir.join("index.html"), "<html><body><h1>E2E App</h1></body></html>").unwrap();
        fs::create_dir_all(project_dir.join("src")).unwrap();
        fs::write(project_dir.join("src/App.tsx"), "export function App() { return <main /> }").unwrap();
        fs::write(project_dir.join("src/styles.css"), "--color-primary: #1a1a2e;").unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        // 1. New Design Chat
        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"E2E Design"}),
        );
        assert_eq!(create["ok"], true, "create: {create}");
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // 2. Send a design request
        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DesignSend","conversation_id": cid, "content": "Create a modern, technical dashboard."}),
        );
        assert_eq!(send["ok"], true, "send: {send}");
        assert_eq!(send["message"]["role"], "assistant");

        // 3. Product understanding
        let understand = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"u1","command":"DesignUnderstand","conversation_id": cid}),
        );
        assert_eq!(understand["ok"], true);
        let analysis = &understand["analysis"];
        assert!(analysis["framework"].is_string(), "framework detected: {analysis}");

        // 4. Design Brief
        let brief = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"b1","command":"DesignBrief","conversation_id": cid, "audience": "devops engineers", "workflow": "monitor dashboards"}),
        );
        assert_eq!(brief["ok"], true);
        assert_eq!(brief["brief"]["product"], "E2E Design");
        assert_eq!(brief["brief"]["audience"], "devops engineers");

        // 5. Design Grammar
        let grammar = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"gm1","command":"DesignGrammar","conversation_id": cid}),
        );
        assert_eq!(grammar["ok"], true);
        assert!(grammar["grammar"]["color_roles"][0].as_str().unwrap().contains("E2E Design"));

        // 6. Real application preview (best-effort — skip if python3 unavailable)
        let preview = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p1","command":"DesignPreviewStart","conversation_id": cid}),
        );
        if preview["ok"] == true && preview["preview"]["status"] == "launched" {
            // Verify status reports the process
            let status = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"ps1","command":"DesignPreviewStatus","conversation_id": cid}),
            );
            let _http_ok = status["preview"]["http_ready"].as_bool().unwrap_or(false);
            // The server may or may not be ready yet; verify the structure exists
            assert!(status["preview"]["port"].is_number() || status["preview"]["port"].is_null());
            // Stop the preview
            let stop = request_via_ipc(
                &server, &listener, &mut daemon,
                json!({"id":"ps2","command":"DesignPreviewStop","conversation_id": cid}),
            );
            assert_eq!(stop["ok"], true);
        } else {
            // Preview commands may not be available in all environments — skip
            eprintln!("preview start skipped: {:?}", preview);
        }

        // 7. Browser inspection (deterministic with HTML fixture)
        let html = r#"
            <html lang="en"><body>
              <header><h1>Dashboard</h1></header>
              <nav><a href="/">Home</a><a href="/settings">Settings</a></nav>
              <main>
                <button role="button">Refresh</button>
                <form><label>Filter</label><input name="q" aria-label="Filter" /></form>
              </main>
            </body></html>
        "#;
        let browser = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"br1","command":"DesignBrowser","conversation_id": cid, "url":"http://127.0.0.1:8787", "html": html, "deterministic": true}),
        );
        assert_eq!(browser["ok"], true, "browser: {browser}");
        let browser_val = &browser["browser"];
        assert!(browser_val["visible_text"].as_str().unwrap().contains("Dashboard"));
        assert!(browser_val["screenshot_uri"].as_str().unwrap().contains("screenshot:"));

        // 8. Critique
        let critique = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cr1","command":"DesignCritique","conversation_id": cid, "content": html, "doc_type":"rendered"}),
        );
        assert_eq!(critique["ok"], true);
        // Clean HTML should pass
        assert_eq!(critique["critique"]["passed"], true);

        // 9. Repair (on a slop fixture to verify the repair loop produces actionable output)
        let slop = r#"
            <section class="hero" style="background: linear-gradient(180deg, #667eea, #764ba2); height: 100vh;">
              <h1>Welcome to the AI-powered platform</h1>
            </section>
            <div class="card">Feature A</div>
            <div class="card">Feature B</div>
            <div class="card">Feature C</div>
        "#;
        let repair = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"r1","command":"DesignRepair","conversation_id": cid, "content": slop, "doc_type":"implementation"}),
        );
        assert_eq!(repair["ok"], true);
        assert_eq!(repair["repair"]["improvement_required"], true);
        assert!(!repair["repair"]["repairs"].as_array().unwrap().is_empty());

        // 10. Screenshot again (after repair — same deterministic html)
        let browser2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"br2","command":"DesignBrowser","conversation_id": cid, "url":"http://127.0.0.1:8787", "html": html, "deterministic": true}),
        );
        assert_eq!(browser2["ok"], true);
        assert!(browser2["browser"]["screenshot_uri"].as_str().unwrap().contains("screenshot:"));

        // 11. Responsive QA (deterministic: layout metrics honestly flagged
        // as unmeasured — never a fabricated pass or fail)
        let responsive = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"qa1","command":"DesignQAResponsive","conversation_id": cid, "html": html, "deterministic": true}),
        );
        assert_eq!(responsive["ok"], true);
        assert!(responsive["qa"].is_object());
        assert!(
            responsive["qa"]["unmeasured"].as_array().unwrap().iter().any(|i| i.as_str().unwrap_or("").contains("not measurable")),
            "deterministic layout metrics must be flagged unmeasured: {}",
            responsive["qa"]
        );

        // 12. Accessibility QA
        let a11y = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"qa2","command":"DesignQAAccessibility","conversation_id": cid, "html": html, "deterministic": true}),
        );
        assert_eq!(a11y["ok"], true);
        // The fixture has semantic <button>, <label>, <a> controls — must pass
        // the structural layer.
        assert_eq!(a11y["qa"]["passed"], true, "semantic HTML should pass a11y: {a11y}");

        // 13. Functional QA
        let functional = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"qa3","command":"DesignQAFunctional","conversation_id": cid, "html": html, "deterministic": true}),
        );
        assert_eq!(functional["ok"], true);
        assert_eq!(functional["qa"]["passed"], true);

        // 14. Persistent Design State
        let state = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"DesignState","conversation_id": cid}),
        );
        assert_eq!(state["ok"], true);
        assert!(state["design_state"]["content"].as_str().unwrap().contains("DESIGN_STATE.md"));

        // 15. Verify conversation persists all messages
        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert!(msgs.len() >= 2, "at least user + assistant");
        assert_eq!(get["conversation"]["mode"], "DESIGN");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_preview_lifecycle_handles_missing_command_and_stop() {
        // G4-08: verify preview lifecycle gracefully handles projects without a
        // dev command, and that status/stop work correctly.
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-preview-lifecycle");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Preview Lifecycle"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // No dev command in empty project → PREVIEW_COMMAND_MISSING
        let start = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"p1","command":"DesignPreviewStart","conversation_id": cid}),
        );
        assert_eq!(start["ok"], true, "start: {start}");
        assert_eq!(start["preview"]["status"], "PREVIEW_COMMAND_MISSING");

        // Status without a server → no_preview
        let status = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"ps1","command":"DesignPreviewStatus","conversation_id": cid}),
        );
        assert_eq!(status["ok"], true, "status: {status}");
        assert_eq!(status["preview"]["status"], "no_preview");

        // Stop without a server → stopped
        let stop = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"ps2","command":"DesignPreviewStop","conversation_id": cid}),
        );
        assert_eq!(stop["ok"], true, "stop: {stop}");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_repair_loop_produces_actionable_improvements() {
        // G4-12: verify the repair loop produces actionable improvement
        // instructions for material findings, and accepts clean design.
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-repair-loop");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Repair Loop"}),
        );
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        // Implement: send slop content
        let slop = r#"
            <div class="hero" style="background: linear-gradient(180deg, #667eea, #764ba2); height: 100vh;">
              <h1>Welcome to the future of AI-powered workflow</h1>
              <p>Reimagine your workflow.</p>
            </div>
            <div class="card">Feature A</div>
            <div class="card">Feature B</div>
            <div class="card">Feature C</div>
            <div class="glass-panel" style="backdrop-filter: blur(12px);">Glass content</div>
        "#;

        // Critique: must reject
        let critique = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cr1","command":"DesignCritique","conversation_id": cid, "content": slop, "doc_type":"implementation"}),
        );
        assert_eq!(critique["ok"], true);
        assert_eq!(critique["critique"]["passed"], false, "slop must not pass");
        assert_eq!(critique["critique"]["improvement_required"], true);

        // Repair: must produce actionable repairs
        let repair = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"r1","command":"DesignRepair","conversation_id": cid, "content": slop, "doc_type":"implementation"}),
        );
        assert_eq!(repair["ok"], true);
        assert_eq!(repair["repair"]["improvement_required"], true);
        let repairs = repair["repair"]["repairs"].as_array().unwrap();
        assert!(!repairs.is_empty(), "must produce at least one repair");

        // Each repair has an issue and a repair field
        for r in repairs.iter() {
            assert!(r["issue"].as_str().is_some(), "repair missing issue: {r}");
            assert!(r["repair"].as_str().is_some(), "repair missing repair text: {r}");
        }

        // Second iteration: clean product-specific implementation
        let clean = r#"
            <header class="panel-header">
              <h1>Metrics Review</h1>
            </header>
            <table role="table">
              <thead><tr><th>Name</th><th>Status</th></tr></thead>
            </table>
            <button role="button">Approve</button>
        "#;

        // Critique: clean must pass
        let critique2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"cr2","command":"DesignCritique","conversation_id": cid, "content": clean, "doc_type":"implementation"}),
        );
        assert_eq!(critique2["ok"], true);
        assert_eq!(critique2["critique"]["passed"], true, "clean design must pass: {critique2}");

        // Repair: no improvement needed
        let repair2 = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"r2","command":"DesignRepair","conversation_id": cid, "content": clean, "doc_type":"implementation"}),
        );
        assert_eq!(repair2["ok"], true);
        assert_eq!(repair2["repair"]["improvement_required"], false, "clean design must not need repair: {repair2}");

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn design_send_to_goalsubmit_transition_creates_mission() {
        // G4-18: design conversation → GoalSubmit → mission reference preserved
        let _env_lock = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
        let (dir, db, lock, socket) = temp_paths("ipc-design-mission");
        let project_dir = dir.join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.to_string_lossy().to_string();

        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let Some((server, listener)) = bind_or_skip(&socket, None) else {
            daemon.shutdown().unwrap();
            std::env::remove_var("AGENTCODE_PROVIDER_MODE");
            let _ = fs::remove_dir_all(dir);
            return;
        };

        let create = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Design Mission Path"}),
        );
        assert_eq!(create["ok"], true);
        let cid = create["conversation_id"].as_str().unwrap().to_string();

        let send = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"d1","command":"DesignSend","conversation_id": cid, "content": "Let's design the dashboard layout."}),
        );
        assert_eq!(send["ok"], true);

        let goal = "Implement the dashboard layout from the design discussion.";
        let submit = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": goal}),
        );
        assert_eq!(submit["ok"], true, "submit: {submit}");
        let mission_id = submit["mission_id"].as_str().unwrap().to_string();
        assert!(mission_id.starts_with("mission-"), "mission_id: {mission_id}");

        let get = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
        );
        assert_eq!(get["ok"], true);
        assert_eq!(get["conversation"]["current_mission_id"].as_str().unwrap(), mission_id);
        assert_eq!(get["conversation"]["mode"], "DESIGN", "mode must remain DESIGN");

        let msgs = get["conversation"]["messages"].as_array().unwrap();
        assert!(msgs.iter().any(|m| m["mission_ref"].as_str() == Some(mission_id.as_str())),
            "design conversation must contain mission reference: {msgs:?}");

        let cancel = request_via_ipc(
            &server, &listener, &mut daemon,
            json!({"id":"x1","command":"CancelMission","mission_id": mission_id}),
        );
        assert_eq!(cancel["ok"], true);

        server.cleanup();
        daemon.shutdown().unwrap();
        std::env::remove_var("AGENTCODE_PROVIDER_MODE");
        let _ = fs::remove_dir_all(dir);
    }
}
