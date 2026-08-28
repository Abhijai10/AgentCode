impl ControlPlaneDb {
    pub fn save_worktree(&self, worktree: &WorktreeRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO worktrees (
                    id, repository_id, owner_mission_id, owner_worker_id, lease_epoch, path, branch,
                    base_commit, current_commit, status, created_at_ms
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    owner_worker_id = excluded.owner_worker_id,
                    lease_epoch = excluded.lease_epoch,
                    current_commit = excluded.current_commit,
                    status = excluded.status",
                params![
                    worktree.id.as_str(),
                    worktree.repository_id.as_str(),
                    worktree.owner_mission_id.as_str(),
                    worktree.owner_worker_id.as_str(),
                    worktree.lease_epoch,
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
                "SELECT id, repository_id, owner_mission_id, owner_worker_id, lease_epoch, path, branch,
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
                lease_epoch: row.get(4).map_err(db_error)?,
                path: row.get(5).map_err(db_error)?,
                branch: row.get(6).map_err(db_error)?,
                base_commit: row.get(7).map_err(db_error)?,
                current_commit: row.get(8).map_err(db_error)?,
                status: row.get(9).map_err(db_error)?,
                created_at_ms: row.get(10).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn worktree_for_mission(
        &self,
        mission_id: &StableId,
    ) -> AcResult<Option<PersistedWorktree>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, repository_id, owner_mission_id, owner_worker_id, lease_epoch, path, branch,
                        base_commit, current_commit, status, created_at_ms
                 FROM worktrees
                 WHERE owner_mission_id = ?1
                 ORDER BY created_at_ms DESC
                 LIMIT 1",
            )
            .map_err(db_error)?;
        let mut rows = stmt
            .query(params![mission_id.as_str()])
            .map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedWorktree {
                id: row.get(0).map_err(db_error)?,
                repository_id: row.get(1).map_err(db_error)?,
                owner_mission_id: row.get(2).map_err(db_error)?,
                owner_worker_id: row.get(3).map_err(db_error)?,
                lease_epoch: row.get(4).map_err(db_error)?,
                path: row.get(5).map_err(db_error)?,
                branch: row.get(6).map_err(db_error)?,
                base_commit: row.get(7).map_err(db_error)?,
                current_commit: row.get(8).map_err(db_error)?,
                status: row.get(9).map_err(db_error)?,
                created_at_ms: row.get(10).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn save_git_checkpoint(&self, checkpoint: &CheckpointRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO worktree_checkpoints (id, worktree_id, commit_ref, reason, task_attempt_id, test_summary, context_ref, blocker, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    checkpoint.id.as_str(),
                    checkpoint.worktree_id.as_str(),
                    checkpoint.commit_ref.as_str(),
                    checkpoint.reason.as_str(),
                    checkpoint.task_attempt_id.as_ref().map(StableId::as_str),
                    checkpoint.test_summary.as_deref(),
                    checkpoint.context_ref.as_ref().map(StableId::as_str),
                    checkpoint.blocker.as_deref(),
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
                "SELECT id, worktree_id, commit_ref, reason, task_attempt_id, test_summary, context_ref, blocker, created_at_ms
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
                    task_attempt_id: row.get(4)?,
                    test_summary: row.get(5)?,
                    context_ref: row.get(6)?,
                    blocker: row.get(7)?,
                    created_at_ms: row.get(8)?,
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
