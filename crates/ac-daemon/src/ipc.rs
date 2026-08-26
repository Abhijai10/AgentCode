use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use serde_json::{json, Value};

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
            Some(goal) => match daemon.handle(DaemonCommand::CreateSession { goal: goal.to_string() }) {
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
                Ok(tasks) => { let status = daemon.mission_status(mission_id); json!({"id": correlation_id, "ok": true, "mission_id": mission_id, "session_id": status.as_ref().map(|status| status.session_id.to_string()), "state": status.as_ref().map(|status| status.state.clone()), "tasks": tasks.into_iter().map(|(task_id, state)| json!({"task_id": task_id, "state": state})).collect::<Vec<_>>() }) },
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "mission_id is required".to_string()),
        },
        "PauseMission" | "ResumeMission" | "CancelMission" => match request.get("mission_id").and_then(Value::as_str) {
            Some(mission_id) => {
                let result = match command { "PauseMission" => daemon.pause_mission(mission_id), "ResumeMission" => daemon.resume_mission(mission_id), _ => daemon.cancel_mission(mission_id) };
                match result { Ok(()) => json!({"id": correlation_id, "ok": true, "mission_id": mission_id, "state": daemon.mission_status(mission_id).map(|status| status.state)}), Err(error) => error_response(correlation_id, error.code(), error.to_string()) }
            }
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "mission_id is required".to_string()),
        },
        "GetRecentEvents" | "GetBlockedReason" => error_response(correlation_id, "DAEMON-IPC_UNSUPPORTED", "command is not available in macOS V1 runtime".to_string()),
        "ShutdownDaemon" => { let result = daemon.handle(DaemonCommand::Stop); match result { Ok(_) => json!({"id": correlation_id, "ok": true}), Err(error) => error_response(correlation_id, error.code(), error.to_string()) } }
        _ => error_response(correlation_id, "DAEMON-IPC_UNKNOWN_COMMAND", "unknown IPC command".to_string()),
    };
    let should_shutdown = command == "ShutdownDaemon" && response.get("ok") == Some(&Value::Bool(true));
    (response, should_shutdown)
}

fn error_response(id: &str, code: &str, message: String) -> Value { json!({"id": id, "ok": false, "error": {"code": code, "message": message}}) }
fn write_frame(stream: &mut UnixStream, value: &Value) -> AcResult<()> { let body = serde_json::to_vec(value).map_err(|error| AcError::validation("DAEMON-IPC_SERIALIZE", error.to_string()))?; if body.len() > MAX_FRAME_BYTES { return Err(AcError::validation("DAEMON-IPC_FRAME_TOO_LARGE", "IPC message exceeds limit")); } stream.write_all(&(body.len() as u32).to_be_bytes()).and_then(|_| stream.write_all(&body)).map_err(|error| AcError::validation("DAEMON-IPC_WRITE", error.to_string())) }
fn read_frame(stream: &mut UnixStream) -> AcResult<Value> { let mut size = [0; 4]; stream.read_exact(&mut size).map_err(|error| AcError::validation("DAEMON-IPC_FRAME", error.to_string()))?; let size = u32::from_be_bytes(size) as usize; if size == 0 || size > MAX_FRAME_BYTES { return Err(AcError::validation("DAEMON-IPC_FRAME_TOO_LARGE", "invalid IPC frame size")); } let mut body = vec![0; size]; stream.read_exact(&mut body).map_err(|error| AcError::validation("DAEMON-IPC_FRAME", error.to_string()))?; serde_json::from_slice(&body).map_err(|error| AcError::validation("DAEMON-IPC_JSON", error.to_string())) }

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};
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
        let start = Instant::now();
        while !response.is_finished() && start.elapsed() < Duration::from_secs(1) {
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
        while !healthy.is_finished() && start.elapsed() < Duration::from_millis(750) {
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
}
