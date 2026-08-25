use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use serde_json::{json, Value};

const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const IPC_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnixIpcServer {
    path: PathBuf,
}

impl UnixIpcServer {
    pub fn bind(path: impl Into<PathBuf>) -> AcResult<(Self, UnixListener)> {
        let path = path.into();
        if path.exists() {
            std::fs::remove_file(&path).map_err(|error| {
                AcError::conflict("DAEMON-IPC_SOCKET_BUSY", error.to_string())
            })?;
        }
        let listener = UnixListener::bind(&path)
            .map_err(|error| AcError::validation("DAEMON-IPC_BIND", error.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| AcError::validation("DAEMON-IPC_CONFIG", error.to_string()))?;
        Ok((Self { path }, listener))
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn serve_once(&self, listener: &UnixListener, daemon: &mut DaemonService) -> AcResult<bool> {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .map_err(|error| AcError::validation("DAEMON-IPC_CLIENT_CONFIG", error.to_string()))?;
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .map_err(|error| AcError::validation("DAEMON-IPC_CLIENT_CONFIG", error.to_string()))?;
                let should_shutdown = serve_client(&mut stream, daemon)?;
                Ok(should_shutdown)
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(false),
            Err(error) => Err(AcError::validation("DAEMON-IPC_ACCEPT", error.to_string())),
        }
    }

    pub fn cleanup(&self) { let _ = std::fs::remove_file(&self.path); }
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

fn serve_client(stream: &mut UnixStream, daemon: &mut DaemonService) -> AcResult<bool> {
    let request = match read_frame(stream) {
        Ok(request) => request,
        Err(error) => { let _ = write_frame(stream, &error_response("unknown", error.code(), error.to_string())); return Ok(false); }
    };
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
                Ok(tasks) => json!({"id": correlation_id, "ok": true, "mission_id": mission_id, "tasks": tasks.into_iter().map(|(task_id, state)| json!({"task_id": task_id, "state": state})).collect::<Vec<_>>() }),
                Err(error) => error_response(correlation_id, error.code(), error.to_string()),
            },
            None => error_response(correlation_id, "DAEMON-IPC_INVALID", "mission_id is required".to_string()),
        },
        "GetRecentEvents" | "GetBlockedReason" | "PauseMission" | "ResumeMission" | "CancelMission" => error_response(correlation_id, "DAEMON-IPC_UNSUPPORTED", "command is not available in macOS V1 runtime".to_string()),
        "ShutdownDaemon" => { let result = daemon.handle(DaemonCommand::Stop); match result { Ok(_) => json!({"id": correlation_id, "ok": true}), Err(error) => error_response(correlation_id, error.code(), error.to_string()) } }
        _ => error_response(correlation_id, "DAEMON-IPC_UNKNOWN_COMMAND", "unknown IPC command".to_string()),
    };
    write_frame(stream, &response)?;
    Ok(command == "ShutdownDaemon" && response.get("ok") == Some(&Value::Bool(true)))
}

fn error_response(id: &str, code: &str, message: String) -> Value { json!({"id": id, "ok": false, "error": {"code": code, "message": message}}) }
fn write_frame(stream: &mut UnixStream, value: &Value) -> AcResult<()> { let body = serde_json::to_vec(value).map_err(|error| AcError::validation("DAEMON-IPC_SERIALIZE", error.to_string()))?; if body.len() > MAX_FRAME_BYTES { return Err(AcError::validation("DAEMON-IPC_FRAME_TOO_LARGE", "IPC message exceeds limit")); } stream.write_all(&(body.len() as u32).to_be_bytes()).and_then(|_| stream.write_all(&body)).map_err(|error| AcError::validation("DAEMON-IPC_WRITE", error.to_string())) }
fn read_frame(stream: &mut UnixStream) -> AcResult<Value> { let mut size = [0; 4]; stream.read_exact(&mut size).map_err(|error| AcError::validation("DAEMON-IPC_FRAME", error.to_string()))?; let size = u32::from_be_bytes(size) as usize; if size == 0 || size > MAX_FRAME_BYTES { return Err(AcError::validation("DAEMON-IPC_FRAME_TOO_LARGE", "invalid IPC frame size")); } let mut body = vec![0; size]; stream.read_exact(&mut body).map_err(|error| AcError::validation("DAEMON-IPC_FRAME", error.to_string()))?; serde_json::from_slice(&body).map_err(|error| AcError::validation("DAEMON-IPC_JSON", error.to_string())) }
