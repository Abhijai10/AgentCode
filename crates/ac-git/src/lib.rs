use std::collections::BTreeMap;
use std::path::PathBuf;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryRecord {
    pub id: StableId,
    pub root: PathBuf,
    pub head: String,
    pub default_branch: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorktreeStatus {
    Active,
    Degraded,
    Cleaned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeRecord {
    pub id: StableId,
    pub repository_id: StableId,
    pub owner_mission_id: StableId,
    pub owner_worker_id: StableId,
    pub path: PathBuf,
    pub branch: String,
    pub base_commit: String,
    pub status: WorktreeStatus,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointRecord {
    pub id: StableId,
    pub worktree_id: StableId,
    pub commit_ref: String,
    pub reason: String,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct GitCoordinator {
    repositories: BTreeMap<StableId, RepositoryRecord>,
    worktrees: BTreeMap<StableId, WorktreeRecord>,
    checkpoints: BTreeMap<StableId, CheckpointRecord>,
}

impl GitCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_repository(
        &mut self,
        root: PathBuf,
        head: impl Into<String>,
        default_branch: impl Into<String>,
    ) -> AcResult<StableId> {
        let head = head.into();
        let default_branch = default_branch.into();
        if root.as_os_str().is_empty() || head.trim().is_empty() || default_branch.trim().is_empty()
        {
            return Err(AcError::validation(
                "GIT-INVALID_REPOSITORY",
                "repository root, head, and default branch are required",
            ));
        }
        let id = StableId::new("repo");
        self.repositories.insert(
            id.clone(),
            RepositoryRecord {
                id: id.clone(),
                root,
                head,
                default_branch,
            },
        );
        Ok(id)
    }

    pub fn create_worktree(
        &mut self,
        repository_id: &StableId,
        owner_mission_id: StableId,
        owner_worker_id: StableId,
        path: PathBuf,
        branch: impl Into<String>,
        base_commit: impl Into<String>,
    ) -> AcResult<StableId> {
        let branch = branch.into();
        let base_commit = base_commit.into();
        if !self.repositories.contains_key(repository_id) {
            return Err(AcError::validation(
                "GIT-UNKNOWN_REPOSITORY",
                "repository not found",
            ));
        }
        validate_branch(&branch)?;
        if base_commit.trim().is_empty() {
            return Err(AcError::validation(
                "GIT-MISSING_BASE",
                "base commit is required",
            ));
        }
        let id = StableId::new("wt");
        self.worktrees.insert(
            id.clone(),
            WorktreeRecord {
                id: id.clone(),
                repository_id: repository_id.clone(),
                owner_mission_id,
                owner_worker_id,
                path,
                branch,
                base_commit,
                status: WorktreeStatus::Active,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn checkpoint(
        &mut self,
        worktree_id: &StableId,
        commit_ref: impl Into<String>,
        reason: impl Into<String>,
    ) -> AcResult<StableId> {
        let commit_ref = commit_ref.into();
        let reason = reason.into();
        if !self.worktrees.contains_key(worktree_id) {
            return Err(AcError::validation(
                "GIT-UNKNOWN_WORKTREE",
                "worktree not found",
            ));
        }
        if commit_ref.trim().is_empty() || reason.trim().is_empty() {
            return Err(AcError::validation(
                "GIT-INVALID_CHECKPOINT",
                "checkpoint commit and reason are required",
            ));
        }
        let id = StableId::new("checkpoint");
        self.checkpoints.insert(
            id.clone(),
            CheckpointRecord {
                id: id.clone(),
                worktree_id: worktree_id.clone(),
                commit_ref,
                reason,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn worktree(&self, id: &StableId) -> Option<&WorktreeRecord> {
        self.worktrees.get(id)
    }
}

pub fn validate_branch(branch: &str) -> AcResult<()> {
    let invalid = branch.trim().is_empty()
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch.contains("..")
        || branch.contains('~')
        || branch.contains('^')
        || branch.contains(':')
        || branch.contains('\\')
        || branch.contains(' ')
        || branch.ends_with(".lock");
    if invalid {
        return Err(AcError::validation(
            "GIT-INVALID_BRANCH",
            "branch name failed AgentCode safety rules",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_branch_names_are_rejected() {
        assert_eq!(
            validate_branch("../main").unwrap_err().code(),
            "GIT-INVALID_BRANCH"
        );
        assert_eq!(
            validate_branch("feature ok").unwrap_err().code(),
            "GIT-INVALID_BRANCH"
        );
    }

    #[test]
    fn checkpoint_requires_known_worktree() {
        let mut git = GitCoordinator::new();
        let err = git
            .checkpoint(
                &StableId::from_existing("wt-missing").unwrap(),
                "abc",
                "save",
            )
            .unwrap_err();
        assert_eq!(err.code(), "GIT-UNKNOWN_WORKTREE");
    }
}
