use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeDiff {
    pub worktree_id: StableId,
    pub branch: String,
    pub base_commit: String,
    pub head_commit: String,
    pub status: String,
    pub diff: String,
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

    pub fn create_task_workspace(
        &mut self,
        source_root: PathBuf,
        worktree_root: PathBuf,
        owner_mission_id: StableId,
        owner_worker_id: StableId,
    ) -> AcResult<StableId> {
        let base_commit = git_output(&source_root, ["rev-parse", "HEAD"])?;
        let default_branch = git_output(&source_root, ["branch", "--show-current"])
            .unwrap_or_else(|_| "main".to_string());
        let branch = format!("agent/task-{}", StableId::new("branch"));
        validate_branch(&branch)?;
        git_output(
            &source_root,
            [
                "worktree",
                "add",
                "-b",
                branch.as_str(),
                worktree_root.to_str().ok_or_else(|| {
                    AcError::validation("GIT-WORKTREE_PATH_UTF8", "worktree path must be UTF-8")
                })?,
                base_commit.as_str(),
            ],
        )?;
        let repository_id =
            self.register_repository(source_root, base_commit.clone(), default_branch)?;
        self.create_worktree(
            &repository_id,
            owner_mission_id,
            owner_worker_id,
            worktree_root,
            branch,
            base_commit,
        )
    }

    pub fn worktree_status(&self, worktree_id: &StableId) -> AcResult<String> {
        let worktree = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        git_output(&worktree.path, ["status", "--short"])
    }

    pub fn worktree_branch(&self, worktree_id: &StableId) -> AcResult<String> {
        let worktree = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        git_output(&worktree.path, ["branch", "--show-current"])
    }

    pub fn worktree_head(&self, worktree_id: &StableId) -> AcResult<String> {
        let worktree = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        git_output(&worktree.path, ["rev-parse", "HEAD"])
    }

    pub fn worktree_diff(&self, worktree_id: &StableId) -> AcResult<WorktreeDiff> {
        let worktree = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        let status = git_output(&worktree.path, ["status", "--short"])?;
        let diff = git_output(&worktree.path, ["diff", "--", "."])?;
        let head_commit = git_output(&worktree.path, ["rev-parse", "HEAD"])?;
        Ok(WorktreeDiff {
            worktree_id: worktree_id.clone(),
            branch: worktree.branch.clone(),
            base_commit: worktree.base_commit.clone(),
            head_commit,
            status,
            diff,
        })
    }

    pub fn cleanup_worktree(&mut self, worktree_id: &StableId) -> AcResult<()> {
        let worktree = self
            .worktrees
            .get_mut(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        let repository = self
            .repositories
            .get(&worktree.repository_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_REPOSITORY", "repository not found"))?;
        git_output(
            &repository.root,
            [
                "worktree",
                "remove",
                "--force",
                worktree.path.to_str().ok_or_else(|| {
                    AcError::validation("GIT-WORKTREE_PATH_UTF8", "worktree path must be UTF-8")
                })?,
            ],
        )?;
        worktree.status = WorktreeStatus::Cleaned;
        Ok(())
    }
}

fn git_output<const N: usize>(cwd: &Path, args: [&str; N]) -> AcResult<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| AcError::validation("GIT-COMMAND_FAILED", err.to_string()))?;
    if !output.status.success() {
        return Err(AcError::validation(
            "GIT-COMMAND_FAILED",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
    use std::fs;

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

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

    #[test]
    fn task_workspace_isolated_from_source_directory() {
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree = std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        fs::create_dir_all(source.join("src")).unwrap();
        fs::write(source.join("src/lib.rs"), "pub fn old() {}\n").unwrap();
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
        let mut git = GitCoordinator::new();
        let worktree_id = git
            .create_task_workspace(
                source.clone(),
                worktree.clone(),
                StableId::new("mission"),
                StableId::new("worker"),
            )
            .unwrap();
        fs::write(worktree.join("src/lib.rs"), "pub fn new() {}\n").unwrap();
        let diff = git.worktree_diff(&worktree_id).unwrap();
        assert!(diff.status.contains("src/lib.rs"));
        assert!(diff.diff.contains("pub fn new"));
        assert_eq!(
            fs::read_to_string(source.join("src/lib.rs")).unwrap(),
            "pub fn old() {}\n"
        );
        assert!(git.worktree(&worktree_id).is_some());
        git.cleanup_worktree(&worktree_id).unwrap();
        let _ = fs::remove_dir_all(source);
    }
}
