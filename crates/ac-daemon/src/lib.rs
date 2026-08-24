use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_db::{ControlPlaneDb, PersistedSession};
use ac_kernel::{AllowAllPolicy, Kernel, MissionState};
use ac_runtime::{AgentSession, AgentSessionState, Worker};

include!("release.rs");

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

pub trait DaemonLifecycleRuntime {
    fn start(&mut self) -> AcResult<()>;
    fn stop(&mut self) -> AcResult<()>;
    fn restart(&mut self) -> AcResult<()>;
    fn recover(&mut self) -> AcResult<usize>;
    fn heartbeat(&self) -> AcResult<DaemonHealth>;
    fn shutdown(&mut self) -> AcResult<()>;
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
    DesktopWindowClosed {
        desktop_session_id: StableId,
    },
    ReconnectDesktop {
        desktop_session_id: StableId,
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
    DesktopWindowClosed {
        daemon_active: bool,
    },
    DesktopReconnected {
        health: DaemonHealth,
        recovered_sessions: usize,
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

    pub fn recover(&mut self) -> AcResult<usize> {
        self.recovered = self.db.interrupted_sessions()?;
        Ok(self.recovered.len())
    }

    pub fn health(&self) -> DaemonHealth {
        DaemonHealth {
            lifecycle: self.lifecycle,
            recovered_sessions: self.recovered.len(),
        }
    }

    pub fn heartbeat(&self) -> AcResult<DaemonHealth> {
        if self.lifecycle != DaemonLifecycle::Running {
            return Err(AcError::conflict(
                "DAEMON-NOT_RUNNING",
                "daemon heartbeat requires running lifecycle",
            ));
        }
        Ok(self.health())
    }

    pub fn shutdown(&mut self) -> AcResult<()> {
        match self.lifecycle {
            DaemonLifecycle::Running => self.stop(),
            DaemonLifecycle::Stopping | DaemonLifecycle::Stopped | DaemonLifecycle::Created => {
                self.lock_file = None;
                let _ = fs::remove_file(&self.lock_path);
                self.lifecycle = DaemonLifecycle::Stopped;
                Ok(())
            }
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
            DaemonCommand::DesktopWindowClosed { .. } => Ok(DaemonResponse::DesktopWindowClosed {
                daemon_active: self.lifecycle == DaemonLifecycle::Running,
            }),
            DaemonCommand::ReconnectDesktop { .. } => Ok(DaemonResponse::DesktopReconnected {
                health: self.health(),
                recovered_sessions: self.recovered.len(),
            }),
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

impl DaemonLifecycleRuntime for DaemonService {
    fn start(&mut self) -> AcResult<()> {
        DaemonService::start(self)
    }

    fn stop(&mut self) -> AcResult<()> {
        DaemonService::stop(self)
    }

    fn restart(&mut self) -> AcResult<()> {
        DaemonService::restart(self)
    }

    fn recover(&mut self) -> AcResult<usize> {
        DaemonService::recover(self)
    }

    fn heartbeat(&self) -> AcResult<DaemonHealth> {
        DaemonService::heartbeat(self)
    }

    fn shutdown(&mut self) -> AcResult<()> {
        DaemonService::shutdown(self)
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
    fn desktop_close_does_not_stop_daemon_and_reopen_reconnects() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        daemon.start().unwrap();
        let mut ipc = LocalIpc::new(&mut daemon);
        let desktop_session_id = StableId::new("desktop");
        assert_eq!(
            ipc.send(DaemonCommand::DesktopWindowClosed {
                desktop_session_id: desktop_session_id.clone()
            })
            .unwrap(),
            DaemonResponse::DesktopWindowClosed {
                daemon_active: true
            }
        );
        assert!(matches!(
            ipc.send(DaemonCommand::ReconnectDesktop { desktop_session_id })
                .unwrap(),
            DaemonResponse::DesktopReconnected {
                health: DaemonHealth {
                    lifecycle: DaemonLifecycle::Running,
                    ..
                },
                ..
            }
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

    #[test]
    fn lifecycle_contract_supports_recover_heartbeat_and_shutdown() {
        let (dir, db, lock) = temp_paths();
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        assert!(daemon.heartbeat().is_err());
        DaemonLifecycleRuntime::start(&mut daemon).unwrap();
        assert_eq!(
            DaemonLifecycleRuntime::heartbeat(&daemon)
                .unwrap()
                .lifecycle,
            DaemonLifecycle::Running
        );
        assert_eq!(DaemonLifecycleRuntime::recover(&mut daemon).unwrap(), 0);
        DaemonLifecycleRuntime::shutdown(&mut daemon).unwrap();
        assert_eq!(daemon.health().lifecycle, DaemonLifecycle::Stopped);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn phase27_release_engineering_verifies_artifact_update_rollback_and_diagnostics() {
        let mut release = ReleaseEngineer::new();
        let build = release
            .capture_build("1.0.0", "commit-a", "release", "macos-arm64")
            .unwrap();
        assert!(build.reproducible);
        let version = ReleaseVersion {
            version: "1.0.1".to_string(),
            source_commit: "commit-b".to_string(),
        };
        let bytes = b"agentcode-app-bundle";
        let artifact = release
            .create_artifact(&version, "macos-arm64", "app-bundle", bytes)
            .unwrap();
        assert!(release.verify_artifact(&artifact, bytes));
        assert!(!release.verify_artifact(&artifact, b"tampered"));
        let update = release.plan_update("1.0.0", &artifact, bytes);
        assert_eq!(update.decision, UpdateDecision::Install);
        assert!(update.verified);
        let blocked = release.plan_update("1.0.0", &artifact, b"tampered");
        assert_eq!(blocked.decision, UpdateDecision::Blocked);
        let rollback = release.rollback_after_failed_update("1.0.0", "1.0.1", "rollback:commit-a");
        assert_eq!(rollback.decision, UpdateDecision::Rollback);
        let diagnostics =
            release.diagnostics_report("AC_SECRET_CANARY", "daemon ok AC_SECRET_CANARY");
        assert!(!diagnostics.contains("AC_SECRET_CANARY"));
        assert!(diagnostics.contains("[REDACTED]"));
    }

    #[test]
    fn phase27_packaging_layout_keeps_state_outside_user_repository_and_reset_is_safe() {
        let repository = PathBuf::from("/Volumes/T7 Shield/My Repo");
        let layout = PackagingLayout {
            app_bundle_id: "com.agentcode.desktop".to_string(),
            config_dir: PathBuf::from("/Users/me/Library/Application Support/AgentCode/config"),
            data_dir: PathBuf::from("/Users/me/Library/Application Support/AgentCode/data"),
            cache_dir: PathBuf::from("/Users/me/Library/Caches/AgentCode"),
            log_dir: PathBuf::from("/Users/me/Library/Logs/AgentCode"),
            managed_tools_dir: PathBuf::from(
                "/Users/me/Library/Application Support/AgentCode/tools",
            ),
            browser_profiles_dir: PathBuf::from(
                "/Users/me/Library/Application Support/AgentCode/browser",
            ),
        };
        layout.validate(&repository).unwrap();
        let release = ReleaseEngineer::new();
        assert!(release
            .reset_plan_preserves_repository(&repository)
            .iter()
            .all(|path| !path.starts_with(&repository)));
        let unsafe_layout = PackagingLayout {
            data_dir: repository.join(".agentcode"),
            ..layout
        };
        assert_eq!(
            unsafe_layout.validate(&repository).unwrap_err().code(),
            "RELEASE-APP_STATE_IN_REPOSITORY"
        );
    }

    #[test]
    fn phase28_release_candidate_gate_blocks_failed_validation_and_accepts_complete_evidence() {
        let mut release = ReleaseEngineer::new();
        let build = release
            .capture_build("1.0.0-rc.1", "commit-rc", "release", "macos-arm64")
            .unwrap();
        let mut candidate = release
            .create_candidate(
                "1.0.0-rc.1",
                "rc.1",
                &build,
                "macos-arm64",
                vec!["release/rc/scope-freeze.md".to_string()],
            )
            .unwrap();
        let migration = MigrationSafetyReport {
            fresh_install: true,
            upgrade: true,
            schema_version: 18,
            interrupted_recovery: true,
            evidence_ref: "release/rc/migration-report.md".to_string(),
        };
        let failing = ReleaseChecklist {
            security_checks: true,
            tests: false,
            artifact_verification: true,
            migration_validation: true,
        };
        let failed_validation =
            release.run_production_validation(&candidate, &migration, &failing, true, "rc/failed");
        let blocked_gate = ReleaseCandidateGate {
            tests_pass: false,
            security_pass: true,
            migrations_pass: true,
            artifacts_valid: true,
            evidence_refs: failed_validation.evidence_refs(),
        };
        assert_eq!(
            release
                .approve_candidate(&mut candidate, &failed_validation, &blocked_gate)
                .unwrap_err()
                .code(),
            "RC-APPROVAL_BLOCKED"
        );

        let passing = ReleaseChecklist {
            security_checks: true,
            tests: true,
            artifact_verification: true,
            migration_validation: true,
        };
        let validation =
            release.run_production_validation(&candidate, &migration, &passing, true, "rc/pass");
        let gate = ReleaseCandidateGate {
            tests_pass: true,
            security_pass: true,
            migrations_pass: true,
            artifacts_valid: true,
            evidence_refs: validation.evidence_refs(),
        };
        release
            .approve_candidate(&mut candidate, &validation, &gate)
            .unwrap();
        assert_eq!(
            candidate.validation_status,
            ReleaseCandidateStatus::Accepted
        );
    }

    #[test]
    fn phase29_release_manifest_decision_bundle_and_status_require_evidence() {
        let mut release = ReleaseEngineer::new();
        let version = ReleaseVersion {
            version: "1.0.0".to_string(),
            source_commit: "commit-v1".to_string(),
        };
        let artifact = release
            .create_artifact(&version, "macos-arm64", "app-bundle", b"agentcode-v1")
            .unwrap();
        let manifest = release
            .final_manifest(
                "1.0.0",
                vec![
                    "autonomous coding missions".to_string(),
                    "Discuss Mode".to_string(),
                    "Design Studio core".to_string(),
                    "Security baseline".to_string(),
                ],
                vec!["0018_release_candidate_v1.sql".to_string()],
                std::slice::from_ref(&artifact),
                vec!["signed/notarized public artifact is prerequisite-bound".to_string()],
            )
            .unwrap();
        assert!(manifest.manifest_hash.starts_with("fnv1a64:"));

        let build = release
            .capture_build("1.0.0", "commit-v1", "release", "macos-arm64")
            .unwrap();
        let candidate = release
            .create_candidate(
                "1.0.0",
                "v1-final",
                &build,
                "macos-arm64",
                vec!["release/manifest/v1.json".to_string()],
            )
            .unwrap();
        let validation = release.run_production_validation(
            &candidate,
            &MigrationSafetyReport {
                fresh_install: true,
                upgrade: true,
                schema_version: 18,
                interrupted_recovery: true,
                evidence_ref: "release/migration/final.md".to_string(),
            },
            &ReleaseChecklist {
                security_checks: true,
                tests: true,
                artifact_verification: true,
                migration_validation: true,
            },
            true,
            "release/final",
        );
        let decision = release
            .approve_release(&manifest, &validation, "security gate passed")
            .unwrap();
        assert_eq!(decision.approved_version, "1.0.0");
        let bundle = release
            .evidence_bundle(
                "1.0.0",
                "release/audit.md",
                "release/security.md",
                "release/validation.md",
                "release/artifact.md",
                "release/migration.md",
            )
            .unwrap();
        assert!(bundle.complete());

        let mut state = ReleaseStateMachine::new("1.0.0").unwrap();
        assert_eq!(
            state
                .transition(ReleaseStatus::Released, "release/evidence")
                .unwrap_err()
                .code(),
            "RELEASE-INVALID_TRANSITION"
        );
        state
            .transition(ReleaseStatus::Candidate, "release/rc-accepted")
            .unwrap();
        state
            .transition(ReleaseStatus::Approved, "release/decision")
            .unwrap();
        state
            .transition(ReleaseStatus::Released, "release/published-hash")
            .unwrap();
        assert_eq!(state.status, ReleaseStatus::Released);
    }
}
