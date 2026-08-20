use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_db::{ControlPlaneDb, PersistedSession};
use ac_kernel::{AllowAllPolicy, Kernel, MissionState};
use ac_runtime::{AgentSession, AgentSessionState, Worker};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DaemonLifecycle {
    Created,
    Running,
    Stopping,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DaemonHealth {
    pub lifecycle: DaemonLifecycle,
    pub recovered_sessions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DaemonCommand {
    Health,
    CreateSession {
        goal: String,
    },
    CheckpointSession {
        session_id: StableId,
        next_step: u32,
    },
    Stop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DaemonResponse {
    Health(DaemonHealth),
    SessionCreated {
        mission_id: StableId,
        session_id: StableId,
    },
    CheckpointSaved {
        checkpoint_id: StableId,
    },
    Stopped,
}

pub trait IpcTransport {
    fn send(&mut self, command: DaemonCommand) -> AcResult<DaemonResponse>;
}

pub struct LocalIpc<'a> {
    daemon: &'a mut DaemonService,
}

impl<'a> LocalIpc<'a> {
    pub fn new(daemon: &'a mut DaemonService) -> Self {
        Self { daemon }
    }
}

impl IpcTransport for LocalIpc<'_> {
    fn send(&mut self, command: DaemonCommand) -> AcResult<DaemonResponse> {
        self.daemon.handle(command)
    }
}

pub struct DaemonService {
    lifecycle: DaemonLifecycle,
    db: ControlPlaneDb,
    kernel: Kernel<AllowAllPolicy>,
    lock_path: PathBuf,
    lock_file: Option<File>,
    recovered: Vec<PersistedSession>,
}

impl DaemonService {
    pub fn open(db_path: impl AsRef<Path>, lock_path: impl Into<PathBuf>) -> AcResult<Self> {
        let mut db = ControlPlaneDb::open(db_path)?;
        db.migrate()?;
        Ok(Self {
            lifecycle: DaemonLifecycle::Created,
            db,
            kernel: Kernel::new(AllowAllPolicy),
            lock_path: lock_path.into(),
            lock_file: None,
            recovered: Vec::new(),
        })
    }

    pub fn start(&mut self) -> AcResult<()> {
        if self.lifecycle == DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-ALREADY_RUNNING",
                "daemon is already running",
            ));
        }
        self.acquire_singleton()?;
        self.kernel.start()?;
        self.recovered = self.db.interrupted_sessions()?;
        self.lifecycle = DaemonLifecycle::Running;
        Ok(())
    }

    pub fn stop(&mut self) -> AcResult<()> {
        if self.lifecycle != DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-NOT_RUNNING",
                "daemon must be running before stop",
            ));
        }
        self.lifecycle = DaemonLifecycle::Stopping;
        self.kernel.stop()?;
        self.lock_file = None;
        let _ = fs::remove_file(&self.lock_path);
        self.lifecycle = DaemonLifecycle::Stopped;
        Ok(())
    }

    pub fn restart(&mut self) -> AcResult<()> {
        if self.lifecycle == DaemonLifecycle::Running {
            self.stop()?;
        }
        self.start()
    }

    pub fn health(&self) -> DaemonHealth {
        DaemonHealth {
            lifecycle: self.lifecycle,
            recovered_sessions: self.recovered.len(),
        }
    }

    pub fn recovered_sessions(&self) -> &[PersistedSession] {
        &self.recovered
    }

    pub fn handle(&mut self, command: DaemonCommand) -> AcResult<DaemonResponse> {
        match command {
            DaemonCommand::Health => Ok(DaemonResponse::Health(self.health())),
            DaemonCommand::CreateSession { goal } => {
                self.ensure_running()?;
                let mission_id = self.kernel.create_mission(goal)?;
                self.kernel
                    .transition_mission(&mission_id, MissionState::Active, Vec::new())?;
                if let Some(mission) = self.kernel.mission(&mission_id) {
                    self.db.put_mission(mission)?;
                }
                for event in self.kernel.events() {
                    let _ = self.db.append_kernel_event(event);
                }
                let session = AgentSession::new(Worker::new());
                self.db
                    .save_session(session.id(), &mission_id, session_state(session.state()))?;
                Ok(DaemonResponse::SessionCreated {
                    mission_id,
                    session_id: session.id().clone(),
                })
            }
            DaemonCommand::CheckpointSession {
                session_id,
                next_step,
            } => {
                self.ensure_running()?;
                let checkpoint_id = StableId::new("daemoncp");
                self.db
                    .save_checkpoint(&checkpoint_id, &session_id, next_step, "executing")?;
                self.db.update_session_state(&session_id, "executing")?;
                Ok(DaemonResponse::CheckpointSaved { checkpoint_id })
            }
            DaemonCommand::Stop => {
                self.stop()?;
                Ok(DaemonResponse::Stopped)
            }
        }
    }

    fn ensure_running(&self) -> AcResult<()> {
        if self.lifecycle != DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-NOT_RUNNING",
                "daemon command requires running lifecycle",
            ));
        }
        Ok(())
    }

    fn acquire_singleton(&mut self) -> AcResult<()> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.lock_path)
            .map_err(|err| {
                AcError::conflict(
                    "DAEMON-SINGLETON_LOCKED",
                    format!("daemon singleton lock unavailable: {}", err),
                )
            })?;
        self.lock_file = Some(file);
        Ok(())
    }
}

fn session_state(state: AgentSessionState) -> &'static str {
    match state {
        AgentSessionState::Created => "created",
        AgentSessionState::Running => "running",
        AgentSessionState::Cancelling => "cancelling",
        AgentSessionState::Stopped => "stopped",
    }
}

impl Drop for DaemonService {
    fn drop(&mut self) {
        self.lock_file = None;
        if self.lifecycle == DaemonLifecycle::Running {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

pub fn default_paths(base: impl AsRef<Path>) -> (PathBuf, PathBuf) {
    let base = base.as_ref();
    (
        base.join("agentcode.sqlite"),
        base.join("agentcode-daemon.lock"),
    )
}

#[allow(dead_code)]
fn _timestamp_for_observability() -> TimestampMillis {
    TimestampMillis::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_paths() -> (PathBuf, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("agentcode-daemon-{}", StableId::new("tmp")));
        fs::create_dir_all(&dir).unwrap();
        let (db, lock) = default_paths(&dir);
        (dir, db, lock)
    }

    #[test]
    fn daemon_starts_routes_ipc_and_stops() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let mut ipc = LocalIpc::new(&mut daemon);
        let response = ipc
            .send(DaemonCommand::CreateSession {
                goal: "Create README.md".to_string(),
            })
            .unwrap();
        assert!(matches!(response, DaemonResponse::SessionCreated { .. }));
        assert!(matches!(
            ipc.send(DaemonCommand::Health).unwrap(),
            DaemonResponse::Health(DaemonHealth {
                lifecycle: DaemonLifecycle::Running,
                ..
            })
        ));
        assert_eq!(
            ipc.send(DaemonCommand::Stop).unwrap(),
            DaemonResponse::Stopped
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn singleton_prevents_duplicate_launch() {
        let (dir, db, lock) = temp_paths();
        let mut first = DaemonService::open(&db, &lock).unwrap();
        first.start().unwrap();
        let mut second = DaemonService::open(&db, &lock).unwrap();
        let err = second.start().unwrap_err();
        assert_eq!(err.code(), "DAEMON-SINGLETON_LOCKED");
        first.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restart_recovers_interrupted_session() {
        let (dir, db, lock) = temp_paths();
        let session_id;
        {
            let mut daemon = DaemonService::open(&db, &lock).unwrap();
            daemon.start().unwrap();
            let mut ipc = LocalIpc::new(&mut daemon);
            let response = ipc
                .send(DaemonCommand::CreateSession {
                    goal: "Recover me".to_string(),
                })
                .unwrap();
            session_id = match response {
                DaemonResponse::SessionCreated { session_id, .. } => session_id,
                _ => panic!("expected session"),
            };
            ipc.send(DaemonCommand::CheckpointSession {
                session_id,
                next_step: 3,
            })
            .unwrap();
        }
        let mut restarted = DaemonService::open(&db, &lock).unwrap();
        restarted.start().unwrap();
        assert!(!restarted.recovered_sessions().is_empty());
        restarted.stop().unwrap();
        let _ = fs::remove_dir_all(dir);
    }
}
