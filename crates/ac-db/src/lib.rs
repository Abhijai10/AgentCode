use std::path::Path;

use ac_changeset::ChangeSet;
use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::EvidenceRecord;
use ac_git::{CheckpointRecord, WorktreeRecord};
use ac_kernel::{KernelDecisionKind, KernelEvent, Mission, MissionState};
use rusqlite::{params, Connection};

pub struct ControlPlaneDb {
    connection: Connection,
}

impl ControlPlaneDb {
    pub fn open(path: impl AsRef<Path>) -> AcResult<Self> {
        let connection = Connection::open(path).map_err(db_error)?;
        let db = Self { connection };
        db.configure()?;
        Ok(db)
    }

    pub fn open_memory() -> AcResult<Self> {
        let connection = Connection::open_in_memory().map_err(db_error)?;
        let db = Self { connection };
        db.configure()?;
        Ok(db)
    }

    pub fn migrate(&mut self) -> AcResult<()> {
        let current_version = self.user_version()?;
        if current_version > 1 {
            return Err(AcError::conflict(
                "DB-FUTURE_VERSION",
                format!(
                    "database user_version {current_version} is newer than supported version 1"
                ),
            ));
        }
        let tx = self.connection.transaction().map_err(db_error)?;
        tx.execute_batch(include_str!("../../../migrations/0001_kernel_schema.sql"))
            .map_err(db_error)?;
        tx.pragma_update(None, "user_version", 1)
            .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(())
    }

    pub fn user_version(&self) -> AcResult<u32> {
        self.connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(db_error)
    }

    pub fn put_mission(&self, mission: &Mission) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO missions (id, original_goal, state, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at_ms = excluded.updated_at_ms",
                params![
                    mission.id.as_str(),
                    mission.original_goal,
                    mission_state(mission.state),
                    millis(mission.created_at),
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn get_mission(&self, id: &StableId) -> AcResult<Option<PersistedMission>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, original_goal, state, created_at_ms FROM missions WHERE id = ?1")
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedMission {
                id: row.get(0).map_err(db_error)?,
                original_goal: row.get(1).map_err(db_error)?,
                state: row.get(2).map_err(db_error)?,
                created_at_ms: row.get(3).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn append_kernel_event(&self, event: &KernelEvent) -> AcResult<()> {
        let evidence_refs = event
            .evidence_refs
            .iter()
            .map(StableId::to_string)
            .collect::<Vec<_>>()
            .join(",");
        self.connection
            .execute(
                "INSERT OR IGNORE INTO kernel_events (id, decision_kind, subject_id, evidence_refs, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event.id.as_str(),
                    decision_kind(event.decision),
                    event.subject.as_str(),
                    evidence_refs,
                    millis(event.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn kernel_event_count(&self) -> AcResult<u64> {
        self.connection
            .query_row("SELECT COUNT(*) FROM kernel_events", [], |row| row.get(0))
            .map_err(db_error)
    }

    pub fn append_evidence(&self, evidence: &EvidenceRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO evidence_records (id, kind, provenance_json, artifact_uri, content_hash, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    evidence.id.as_str(),
                    format!("{:?}", evidence.kind),
                    format!("{:?}", evidence.provenance),
                    evidence.artifact_uri,
                    evidence.content_hash,
                    millis(evidence.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    fn configure(&self) -> AcResult<()> {
        self.connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_error)?;
        self.connection
            .busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(db_error)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedMission {
    pub id: String,
    pub original_goal: String,
    pub state: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedSession {
    pub id: String,
    pub mission_id: String,
    pub state: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedCheckpoint {
    pub id: String,
    pub session_id: String,
    pub next_step: u32,
    pub state: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedWorktree {
    pub id: String,
    pub repository_id: String,
    pub owner_mission_id: String,
    pub owner_worker_id: String,
    pub path: String,
    pub branch: String,
    pub base_commit: String,
    pub current_commit: String,
    pub status: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedChangeSet {
    pub id: String,
    pub state: String,
    pub operations_json: String,
    pub metadata_json: Option<String>,
    pub rollback_json: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedGitCheckpoint {
    pub id: String,
    pub worktree_id: String,
    pub commit_ref: String,
    pub reason: String,
    pub created_at_ms: i64,
}

impl ControlPlaneDb {
    pub fn save_session(
        &self,
        session_id: &StableId,
        mission_id: &StableId,
        state: &str,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO agent_sessions (id, mission_id, state, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at_ms = excluded.updated_at_ms",
                params![
                    session_id.as_str(),
                    mission_id.as_str(),
                    state,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_checkpoint(
        &self,
        id: &StableId,
        session_id: &StableId,
        next_step: u32,
        state: &str,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO agent_checkpoints (id, session_id, next_step, state, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id.as_str(),
                    session_id.as_str(),
                    next_step,
                    state,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn update_session_state(&self, session_id: &StableId, state: &str) -> AcResult<()> {
        let changed = self
            .connection
            .execute(
                "UPDATE agent_sessions
                 SET state = ?2, updated_at_ms = ?3
                 WHERE id = ?1",
                params![session_id.as_str(), state, millis(TimestampMillis::now())],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err(AcError::conflict(
                "DB-SESSION_NOT_FOUND",
                format!("agent session {} was not found", session_id),
            ));
        }
        Ok(())
    }

    pub fn interrupted_sessions(&self) -> AcResult<Vec<PersistedSession>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, state, updated_at_ms
                 FROM agent_sessions
                 WHERE state NOT IN ('completed', 'cancelled', 'failed')",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PersistedSession {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    state: row.get(2)?,
                    updated_at_ms: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn get_session(&self, session_id: &StableId) -> AcResult<Option<PersistedSession>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, state, updated_at_ms FROM agent_sessions WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![session_id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedSession {
                id: row.get(0).map_err(db_error)?,
                mission_id: row.get(1).map_err(db_error)?,
                state: row.get(2).map_err(db_error)?,
                updated_at_ms: row.get(3).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn checkpoints_for_session(
        &self,
        session_id: &StableId,
    ) -> AcResult<Vec<PersistedCheckpoint>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, session_id, next_step, state, created_at_ms
                 FROM agent_checkpoints
                 WHERE session_id = ?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![session_id.as_str()], |row| {
                Ok(PersistedCheckpoint {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    next_step: row.get(2)?,
                    state: row.get(3)?,
                    created_at_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_worktree(&self, worktree: &WorktreeRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO worktrees (
                    id, repository_id, owner_mission_id, owner_worker_id, path, branch,
                    base_commit, current_commit, status, created_at_ms
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    current_commit = excluded.current_commit,
                    status = excluded.status",
                params![
                    worktree.id.as_str(),
                    worktree.repository_id.as_str(),
                    worktree.owner_mission_id.as_str(),
                    worktree.owner_worker_id.as_str(),
                    worktree.path.display().to_string(),
                    worktree.branch.as_str(),
                    worktree.base_commit.as_str(),
                    worktree.current_commit.as_str(),
                    format!("{:?}", worktree.status),
                    millis(worktree.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn get_worktree(&self, id: &StableId) -> AcResult<Option<PersistedWorktree>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, repository_id, owner_mission_id, owner_worker_id, path, branch,
                        base_commit, current_commit, status, created_at_ms
                 FROM worktrees
                 WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedWorktree {
                id: row.get(0).map_err(db_error)?,
                repository_id: row.get(1).map_err(db_error)?,
                owner_mission_id: row.get(2).map_err(db_error)?,
                owner_worker_id: row.get(3).map_err(db_error)?,
                path: row.get(4).map_err(db_error)?,
                branch: row.get(5).map_err(db_error)?,
                base_commit: row.get(6).map_err(db_error)?,
                current_commit: row.get(7).map_err(db_error)?,
                status: row.get(8).map_err(db_error)?,
                created_at_ms: row.get(9).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn save_git_checkpoint(&self, checkpoint: &CheckpointRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO worktree_checkpoints (id, worktree_id, commit_ref, reason, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    checkpoint.id.as_str(),
                    checkpoint.worktree_id.as_str(),
                    checkpoint.commit_ref.as_str(),
                    checkpoint.reason.as_str(),
                    millis(checkpoint.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn git_checkpoints_for_worktree(
        &self,
        worktree_id: &StableId,
    ) -> AcResult<Vec<PersistedGitCheckpoint>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, worktree_id, commit_ref, reason, created_at_ms
                 FROM worktree_checkpoints
                 WHERE worktree_id = ?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![worktree_id.as_str()], |row| {
                Ok(PersistedGitCheckpoint {
                    id: row.get(0)?,
                    worktree_id: row.get(1)?,
                    commit_ref: row.get(2)?,
                    reason: row.get(3)?,
                    created_at_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_changeset(&self, changeset: &ChangeSet) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO changesets (id, state, operations_json, metadata_json, rollback_json, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    state = excluded.state,
                    operations_json = excluded.operations_json,
                    metadata_json = excluded.metadata_json,
                    rollback_json = excluded.rollback_json",
                params![
                    changeset.id.as_str(),
                    format!("{:?}", changeset.state),
                    format!("{:?}", changeset.operations),
                    changeset.metadata.as_ref().map(|metadata| format!("{:?}", metadata)),
                    changeset.rollback.as_ref().map(|rollback| format!("{:?}", rollback)),
                    millis(changeset.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn get_changeset(&self, id: &StableId) -> AcResult<Option<PersistedChangeSet>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, state, operations_json, metadata_json, rollback_json, created_at_ms
                 FROM changesets
                 WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedChangeSet {
                id: row.get(0).map_err(db_error)?,
                state: row.get(1).map_err(db_error)?,
                operations_json: row.get(2).map_err(db_error)?,
                metadata_json: row.get(3).map_err(db_error)?,
                rollback_json: row.get(4).map_err(db_error)?,
                created_at_ms: row.get(5).map_err(db_error)?,
            }));
        }
        Ok(None)
    }
}

fn millis(ts: TimestampMillis) -> i64 {
    ts.as_millis().min(i64::MAX as u128) as i64
}

fn mission_state(state: MissionState) -> &'static str {
    match state {
        MissionState::Created => "created",
        MissionState::Active => "active",
        MissionState::Completed => "completed",
        MissionState::Cancelled => "cancelled",
    }
}

fn decision_kind(kind: KernelDecisionKind) -> &'static str {
    match kind {
        KernelDecisionKind::CreateMission => "create_mission",
        KernelDecisionKind::ActivateMission => "activate_mission",
        KernelDecisionKind::CompleteMission => "complete_mission",
        KernelDecisionKind::CancelMission => "cancel_mission",
        KernelDecisionKind::ApproveChangeSet => "approve_changeset",
    }
}

fn db_error(error: rusqlite::Error) -> AcError {
    AcError::new(
        "DB-SQLITE",
        error.to_string(),
        ac_common::ErrorKind::Internal,
        ac_common::Retryability::NotRetryable,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_changeset::{ChangeOperation, ChangeSet};
    use ac_git::GitCoordinator;
    use ac_kernel::{AllowAllPolicy, Kernel};
    use std::fs;
    use std::process::Command;

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn sqlite_store_persists_kernel_state() {
        let mut db = ControlPlaneDb::open_memory().unwrap();
        db.migrate().unwrap();
        assert_eq!(db.user_version().unwrap(), 1);

        let mut kernel = Kernel::new(AllowAllPolicy);
        kernel.start().unwrap();
        let mission_id = kernel.create_mission("persist me").unwrap();
        let mission = kernel.mission(&mission_id).unwrap();
        db.put_mission(mission).unwrap();
        for event in kernel.events() {
            db.append_kernel_event(event).unwrap();
        }

        let persisted = db.get_mission(&mission_id).unwrap().unwrap();
        assert_eq!(persisted.original_goal, "persist me");
        assert_eq!(persisted.state, "created");
    }

    #[test]
    fn sqlite_file_survives_reopen_with_event_history() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let mission_id;
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let mut kernel = Kernel::new(AllowAllPolicy);
            kernel.start().unwrap();
            mission_id = kernel.create_mission("durable goal").unwrap();
            db.put_mission(kernel.mission(&mission_id).unwrap())
                .unwrap();
            for event in kernel.events() {
                db.append_kernel_event(event).unwrap();
            }
            assert_eq!(db.kernel_event_count().unwrap(), 1);
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let mission = db.get_mission(&mission_id).unwrap().unwrap();
            assert_eq!(mission.original_goal, "durable goal");
            assert_eq!(db.kernel_event_count().unwrap(), 1);
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn interrupted_sessions_are_recovered_after_reopen() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let session_id = StableId::new("session");
        let mission_id = StableId::new("mission");
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_session(&session_id, &mission_id, "executing")
                .unwrap();
            db.save_checkpoint(&StableId::new("cp"), &session_id, 2, "executing")
                .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let interrupted = db.interrupted_sessions().unwrap();
            assert_eq!(interrupted.len(), 1);
            assert_eq!(interrupted[0].id, session_id.to_string());
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn lifecycle_state_recovers_after_reopen() {
        let db_path =
            std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree_path =
            std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree_path);
        fs::create_dir_all(source.join("src")).unwrap();
        fs::write(source.join("src/lib.rs"), "pub fn answer() -> u32 { 41 }\n").unwrap();
        run_git(&source, ["init"]);
        run_git(&source, ["add", "."]);
        run_git(
            &source,
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

        let session_id = StableId::new("session");
        let mission_id = StableId::new("mission");
        let changeset_id;
        let worktree_id;
        let checkpoint_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.save_session(&session_id, &mission_id, "executing")
                .unwrap();

            let mut git = GitCoordinator::new();
            worktree_id = git
                .create_task_workspace(
                    source.clone(),
                    worktree_path.clone(),
                    mission_id.clone(),
                    session_id.clone(),
                )
                .unwrap();
            fs::write(
                worktree_path.join("src/lib.rs"),
                "pub fn answer() -> u32 { 42 }\n",
            )
            .unwrap();
            checkpoint_id = git.checkpoint_current(&worktree_id, "fix answer").unwrap();
            db.save_worktree(git.worktree(&worktree_id).unwrap())
                .unwrap();
            db.save_git_checkpoint(git.checkpoint_record(&checkpoint_id).unwrap())
                .unwrap();

            let mut changeset = ChangeSet::propose(
                vec![ChangeOperation::WriteFile {
                    path: "src/lib.rs".to_string(),
                    expected_hash: None,
                    new_hash: "len:29".to_string(),
                }],
                None,
            )
            .unwrap();
            changeset.validate().unwrap();
            changeset_id = changeset.id.clone();
            db.save_changeset(&changeset).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&db_path).unwrap();
            let session = db.get_session(&session_id).unwrap().unwrap();
            let worktree = db.get_worktree(&worktree_id).unwrap().unwrap();
            let checkpoints = db.git_checkpoints_for_worktree(&worktree_id).unwrap();
            let changeset = db.get_changeset(&changeset_id).unwrap().unwrap();

            assert_eq!(session.state, "executing");
            assert_eq!(worktree.owner_mission_id, mission_id.to_string());
            assert_eq!(checkpoints[0].commit_ref, worktree.current_commit);
            assert_eq!(checkpoints[0].reason, "fix answer");
            assert_eq!(checkpoints[0].id, checkpoint_id.to_string());
            assert_eq!(changeset.state, "Validated");
            assert!(changeset.operations_json.contains("src/lib.rs"));
        }
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(worktree_path);
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn future_schema_version_is_rejected() {
        let mut db = ControlPlaneDb::open_memory().unwrap();
        db.connection
            .pragma_update(None, "user_version", 99)
            .unwrap();
        let error = db.migrate().unwrap_err();
        assert_eq!(error.code(), "DB-FUTURE_VERSION");
    }
}
