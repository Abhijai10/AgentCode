use std::path::Path;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::EvidenceRecord;
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
                "INSERT INTO kernel_events (id, decision_kind, subject_id, evidence_refs, created_at_ms)
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
    use ac_kernel::{AllowAllPolicy, Kernel};

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
}
