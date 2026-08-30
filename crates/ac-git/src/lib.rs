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
    Missing,
    Cleaned,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeRecord {
    pub id: StableId,
    pub repository_id: StableId,
    pub owner_mission_id: StableId,
    pub owner_worker_id: StableId,
    pub lease_epoch: u64,
    pub path: PathBuf,
    pub branch: String,
    pub base_commit: String,
    pub current_commit: String,
    pub status: WorktreeStatus,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointRecord {
    pub id: StableId,
    pub worktree_id: StableId,
    pub commit_ref: String,
    pub reason: String,
    pub task_attempt_id: Option<StableId>,
    pub test_summary: Option<String>,
    pub context_ref: Option<StableId>,
    pub blocker: Option<String>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeReview {
    pub id: StableId,
    pub worktree_id: StableId,
    pub branch: String,
    pub base_commit: String,
    pub merge_commit: Option<String>,
    pub rollback_commit: Option<String>,
    pub conflict_detected: bool,
    pub approved_by_kernel: bool,
    pub cleaned_up: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeConflict {
    pub files: Vec<String>,
    pub base: String,
    pub ours: String,
    pub theirs: String,
    pub related_tasks: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrationBranch {
    pub mission_id: StableId,
    pub repository_id: StableId,
    pub branch: String,
    pub base_commit: String,
}

#[derive(Default)]
pub struct GitCoordinator {
    repositories: BTreeMap<StableId, RepositoryRecord>,
    worktrees: BTreeMap<StableId, WorktreeRecord>,
    checkpoints: BTreeMap<StableId, CheckpointRecord>,
    integrations: BTreeMap<StableId, IntegrationBranch>,
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
                lease_epoch: 1,
                path,
                branch,
                current_commit: base_commit.clone(),
                base_commit,
                status: WorktreeStatus::Active,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn register_existing_worktree(
        &mut self,
        source_root: PathBuf,
        worktree: WorktreeRecord,
    ) -> AcResult<StableId> {
        if !worktree.path.exists() || !worktree.path.is_dir() {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_MISSING",
                "expected interrupted worktree directory is missing",
            ));
        }
        let expected_path = canonical_path(&worktree.path)?;
        let actual_root = canonical_path(Path::new(&git_output(
            &worktree.path,
            ["rev-parse", "--show-toplevel"],
        )?))?;
        if actual_root != expected_path {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_PATH",
                "existing directory is not the expected git worktree",
            ));
        }
        let source_common = canonical_path(&source_root.join(".git"))?;
        let actual_common = canonical_path(&worktree.path.join(git_output(
            &worktree.path,
            ["rev-parse", "--git-common-dir"],
        )?))?;
        if actual_common != source_common {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_REPOSITORY",
                "existing worktree belongs to a different source repository",
            ));
        }
        let branch = git_output(&worktree.path, ["branch", "--show-current"])?;
        if branch != worktree.branch {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_BRANCH",
                "existing worktree branch does not match the persisted branch",
            ));
        }
        let base_present = git_output(&worktree.path, ["cat-file", "-t", &worktree.base_commit])?;
        if base_present != "commit" {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_BASE",
                "persisted base commit is not present in the worktree repository",
            ));
        }
        let default_branch = git_output(&source_root, ["branch", "--show-current"])
            .unwrap_or_else(|_| "main".to_string());
        self.repositories.insert(
            worktree.repository_id.clone(),
            RepositoryRecord {
                id: worktree.repository_id.clone(),
                root: source_root,
                head: worktree.base_commit.clone(),
                default_branch,
            },
        );
        let id = worktree.id.clone();
        self.worktrees.insert(id.clone(), worktree);
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
                task_attempt_id: None,
                test_summary: None,
                context_ref: None,
                blocker: None,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint_with_metadata(
        &mut self,
        worktree_id: &StableId,
        commit_ref: impl Into<String>,
        reason: impl Into<String>,
        task_attempt_id: Option<StableId>,
        test_summary: Option<String>,
        context_ref: Option<StableId>,
        blocker: Option<String>,
    ) -> AcResult<StableId> {
        let id = self.checkpoint(worktree_id, commit_ref, reason)?;
        let checkpoint = self
            .checkpoints
            .get_mut(&id)
            .expect("created checkpoint exists");
        checkpoint.task_attempt_id = task_attempt_id;
        checkpoint.test_summary = test_summary;
        checkpoint.context_ref = context_ref;
        checkpoint.blocker = blocker;
        Ok(id)
    }

    pub fn transfer_worktree_lease(
        &mut self,
        worktree_id: &StableId,
        expected_epoch: u64,
        replacement_worker_id: StableId,
    ) -> AcResult<u64> {
        let worktree = self
            .worktrees
            .get_mut(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        if worktree.lease_epoch != expected_epoch {
            return Err(AcError::conflict(
                "GIT-STALE_WORKTREE_LEASE",
                "worktree lease is stale",
            ));
        }
        worktree.owner_worker_id = replacement_worker_id;
        worktree.lease_epoch += 1;
        Ok(worktree.lease_epoch)
    }

    fn require_lease(
        &self,
        worktree_id: &StableId,
        worker_id: &StableId,
        epoch: u64,
    ) -> AcResult<()> {
        let worktree = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        if &worktree.owner_worker_id != worker_id || worktree.lease_epoch != epoch {
            return Err(AcError::conflict(
                "GIT-STALE_WORKTREE_LEASE",
                "worker no longer owns this worktree",
            ));
        }
        Ok(())
    }

    pub fn checkpoint_current(
        &mut self,
        worktree_id: &StableId,
        reason: impl Into<String>,
    ) -> AcResult<StableId> {
        let reason = reason.into();
        let worktree_path = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?
            .path
            .clone();
        git_output(&worktree_path, ["add", "."])?;
        git_output(
            &worktree_path,
            [
                "-c",
                "user.name=AgentCode",
                "-c",
                "user.email=agentcode@example.test",
                "commit",
                "--allow-empty",
                "-m",
                reason.as_str(),
            ],
        )?;
        let commit = git_output(&worktree_path, ["rev-parse", "HEAD"])?;
        if let Some(worktree) = self.worktrees.get_mut(worktree_id) {
            worktree.current_commit = commit.clone();
        }
        self.checkpoint(worktree_id, commit, reason)
    }

    pub fn checkpoint_current_owned(
        &mut self,
        worktree_id: &StableId,
        worker_id: &StableId,
        lease_epoch: u64,
        reason: impl Into<String>,
    ) -> AcResult<StableId> {
        self.require_lease(worktree_id, worker_id, lease_epoch)?;
        self.checkpoint_current(worktree_id, reason)
    }

    pub fn recover_worktree(&mut self, checkpoint_id: &StableId) -> AcResult<StableId> {
        let checkpoint = self
            .checkpoints
            .get(checkpoint_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_CHECKPOINT", "checkpoint not found"))?
            .clone();
        let worktree = self
            .worktrees
            .get_mut(&checkpoint.worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        git_output(
            &worktree.path,
            ["reset", "--hard", checkpoint.commit_ref.as_str()],
        )?;
        worktree.current_commit = checkpoint.commit_ref;
        worktree.status = WorktreeStatus::Active;
        Ok(worktree.id.clone())
    }

    pub fn restore_to_base(&mut self, worktree_id: &StableId) -> AcResult<StableId> {
        let worktree = self
            .worktrees
            .get_mut(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        git_output(
            &worktree.path,
            ["reset", "--hard", worktree.base_commit.as_str()],
        )?;
        worktree.current_commit = worktree.base_commit.clone();
        worktree.status = WorktreeStatus::Active;
        Ok(worktree.id.clone())
    }

    pub fn worktree(&self, id: &StableId) -> Option<&WorktreeRecord> {
        self.worktrees.get(id)
    }

    pub fn checkpoint_record(&self, id: &StableId) -> Option<&CheckpointRecord> {
        self.checkpoints.get(id)
    }

    pub fn create_task_workspace(
        &mut self,
        source_root: PathBuf,
        worktree_root: PathBuf,
        owner_mission_id: StableId,
        owner_worker_id: StableId,
    ) -> AcResult<StableId> {
        if !dirty_status_excluding_managed_worktrees(&source_root)?.is_empty() {
            return Err(AcError::conflict(
                "GIT-DIRTY_BASE",
                "source repository has uncommitted user changes",
            ));
        }
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

    /// Create a task worktree at the deterministic session path, or safely
    /// reattach to an existing one left behind by an interrupted daemon.
    ///
    /// After a daemon crash the deterministic worktree directory may already
    /// exist with partial completed work.  Recreating it with `git worktree
    /// add` would fail because the destination already exists.  Instead this
    /// method verifies the existing directory is genuinely the expected
    /// AgentCode worktree of the expected source repository, discovers the
    /// branch/HEAD from git itself, and reconstructs the in-memory worktree
    /// record without creating a second worktree and without touching the
    /// source repository.  A mismatched or tampered directory fails closed.
    pub fn recover_or_create_worktree(
        &mut self,
        source_root: PathBuf,
        worktree_root: PathBuf,
        owner_mission_id: StableId,
        owner_worker_id: StableId,
    ) -> AcResult<StableId> {
        if !worktree_root.exists() {
            return self.create_task_workspace(
                source_root,
                worktree_root,
                owner_mission_id,
                owner_worker_id,
            );
        }
        if !worktree_root.is_dir() {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_NOT_DIR",
                "expected worktree path exists but is not a directory",
            ));
        }
        // The existing directory must be a git worktree whose top level is
        // exactly the deterministic session path.
        let expected_path = canonical_path(&worktree_root)?;
        let actual_root = canonical_path(Path::new(
            &git_output(&worktree_root, ["rev-parse", "--show-toplevel"]).map_err(|_error| {
                AcError::conflict(
                    "GIT-WORKTREE_REATTACH_PATH",
                    "existing directory is not a git worktree",
                )
            })?,
        ))?;
        if actual_root != expected_path {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_PATH",
                "existing directory is not the expected git worktree",
            ));
        }
        // Repository identity: the worktree must belong to the same source
        // repository the mission is bound to.
        let source_common = canonical_path(&source_root.join(".git"))?;
        let actual_common = canonical_path(&worktree_root.join(git_output(
            &worktree_root,
            ["rev-parse", "--git-common-dir"],
        )?))?;
        if actual_common != source_common {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_REPOSITORY",
                "existing worktree belongs to a different source repository",
            ));
        }
        // Ownership: only an AgentCode task branch may be resumed.
        let branch = git_output(&worktree_root, ["branch", "--show-current"])?;
        validate_branch(&branch)?;
        if !branch.starts_with("agent/task-") {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_OWNERSHIP",
                "existing worktree branch is not an AgentCode task branch",
            ));
        }
        let current_commit = git_output(&worktree_root, ["rev-parse", "HEAD"])?;
        let default_branch = git_output(&source_root, ["branch", "--show-current"])
            .unwrap_or_else(|_| "main".to_string());
        // Reconstruct the fork point as the base commit.  This is where the
        // interrupted branch diverged from the source default branch; it is a
        // valid object in the shared object database.
        let base_commit = git_output(
            &source_root,
            ["merge-base", branch.as_str(), default_branch.as_str()],
        )
        .or_else(|_| git_output(&worktree_root, ["rev-list", "--max-parents=0", "HEAD"]))?;
        let base_present = git_output(&worktree_root, ["cat-file", "-t", base_commit.as_str()])?;
        if base_present != "commit" {
            return Err(AcError::conflict(
                "GIT-WORKTREE_REATTACH_BASE",
                "reconstructed base commit is not present in the worktree",
            ));
        }
        let repository_id =
            self.register_repository(source_root, current_commit.clone(), default_branch)?;
        let id = StableId::new("wt");
        self.worktrees.insert(
            id.clone(),
            WorktreeRecord {
                id: id.clone(),
                repository_id,
                owner_mission_id,
                owner_worker_id,
                lease_epoch: 1,
                path: worktree_root,
                branch,
                current_commit: current_commit.clone(),
                base_commit,
                status: WorktreeStatus::Active,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
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
        if !worktree.path.exists() {
            worktree.status = WorktreeStatus::Missing;
            return Err(AcError::conflict(
                "GIT-WORKTREE_MISSING",
                "worktree directory is missing",
            ));
        }
        if !git_output(&worktree.path, ["status", "--porcelain"])?.is_empty() {
            return Err(AcError::conflict(
                "GIT-WORKTREE_DIRTY_CLEANUP",
                "refusing to delete uncommitted work",
            ));
        }
        git_output(
            &repository.root,
            [
                "worktree",
                "remove",
                worktree.path.to_str().ok_or_else(|| {
                    AcError::validation("GIT-WORKTREE_PATH_UTF8", "worktree path must be UTF-8")
                })?,
            ],
        )?;
        worktree.status = WorktreeStatus::Cleaned;
        Ok(())
    }

    pub fn reconcile_worktree(&mut self, worktree_id: &StableId) -> AcResult<WorktreeStatus> {
        let worktree = self
            .worktrees
            .get_mut(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        if !worktree.path.exists() {
            worktree.status = WorktreeStatus::Missing;
        } else if worktree.status == WorktreeStatus::Missing {
            worktree.status = WorktreeStatus::Active;
        }
        Ok(worktree.status)
    }

    pub fn create_integration_branch(
        &mut self,
        repository_id: &StableId,
        mission_id: StableId,
    ) -> AcResult<IntegrationBranch> {
        let repository = self
            .repositories
            .get(repository_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_REPOSITORY", "repository not found"))?
            .clone();
        let branch = format!("agentcode/mission-{mission_id}/integration");
        validate_branch(&branch)?;
        git_output(
            &repository.root,
            ["branch", branch.as_str(), repository.head.as_str()],
        )?;
        let integration = IntegrationBranch {
            mission_id: mission_id.clone(),
            repository_id: repository_id.clone(),
            branch,
            base_commit: repository.head,
        };
        self.integrations.insert(mission_id, integration.clone());
        Ok(integration)
    }

    pub fn integrate_worktree(
        &mut self,
        worktree_id: &StableId,
        mission_id: &StableId,
        approved_by_kernel: bool,
    ) -> AcResult<Option<MergeConflict>> {
        if !approved_by_kernel {
            return Err(AcError::policy_denied(
                "GIT-MERGE_REQUIRES_KERNEL_APPROVAL",
                "integration requires Kernel approval",
            ));
        }
        let worktree = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?
            .clone();
        let integration = self
            .integrations
            .get(mission_id)
            .ok_or_else(|| {
                AcError::validation("GIT-UNKNOWN_INTEGRATION", "integration branch not found")
            })?
            .clone();
        let repository = self
            .repositories
            .get(&worktree.repository_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_REPOSITORY", "repository not found"))?
            .clone();
        let previous = git_output(&repository.root, ["rev-parse", "HEAD"])?;
        let previous_branch = git_output(&repository.root, ["branch", "--show-current"])?;
        git_output(&repository.root, ["checkout", integration.branch.as_str()])?;
        let merge = Command::new("git")
            .args([
                "-c",
                "user.name=AgentCode",
                "-c",
                "user.email=agentcode@example.test",
                "merge",
                "--no-ff",
                "--no-edit",
                worktree.branch.as_str(),
            ])
            .current_dir(&repository.root)
            .output()
            .map_err(|e| AcError::validation("GIT-COMMAND_FAILED", e.to_string()))?;
        if merge.status.success() {
            return Ok(None);
        }
        let files = git_output(&repository.root, ["diff", "--name-only", "--diff-filter=U"])
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect();
        let base = git_output(
            &repository.root,
            ["merge-base", "HEAD", worktree.branch.as_str()],
        )
        .unwrap_or_else(|_| worktree.base_commit.clone());
        let _ = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(&repository.root)
            .output();
        let _ = git_output(&repository.root, ["checkout", previous_branch.as_str()]);
        let _ = git_output(&repository.root, ["reset", "--hard", previous.as_str()]);
        Ok(Some(MergeConflict {
            files,
            base,
            ours: previous,
            theirs: worktree.current_commit,
            related_tasks: vec![worktree.owner_mission_id],
        }))
    }

    pub fn prepare_merge_review(
        &self,
        worktree_id: &StableId,
        approved_by_kernel: bool,
    ) -> AcResult<MergeReview> {
        if !approved_by_kernel {
            return Err(AcError::policy_denied(
                "GIT-MERGE_REQUIRES_KERNEL_APPROVAL",
                "local merge review requires Kernel approval",
            ));
        }
        let worktree = self
            .worktrees
            .get(worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?;
        Ok(MergeReview {
            id: StableId::new("merge"),
            worktree_id: worktree_id.clone(),
            branch: worktree.branch.clone(),
            base_commit: worktree.base_commit.clone(),
            merge_commit: None,
            rollback_commit: None,
            conflict_detected: false,
            approved_by_kernel,
            cleaned_up: false,
        })
    }

    pub fn complete_merge_review(&mut self, review: &mut MergeReview) -> AcResult<()> {
        if !review.approved_by_kernel {
            return Err(AcError::policy_denied(
                "GIT-MERGE_REQUIRES_KERNEL_APPROVAL",
                "local merge review requires Kernel approval",
            ));
        }
        let worktree = self
            .worktrees
            .get(&review.worktree_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_WORKTREE", "worktree not found"))?
            .clone();
        let repository = self
            .repositories
            .get(&worktree.repository_id)
            .ok_or_else(|| AcError::validation("GIT-UNKNOWN_REPOSITORY", "repository not found"))?
            .clone();
        let rollback = git_output(&repository.root, ["rev-parse", "HEAD"])?;
        review.rollback_commit = Some(rollback.clone());
        let merge = Command::new("git")
            .args([
                "-c",
                "user.name=AgentCode",
                "-c",
                "user.email=agentcode@example.test",
                "merge",
                "--no-ff",
                "--no-edit",
                worktree.branch.as_str(),
            ])
            .current_dir(&repository.root)
            .output()
            .map_err(|err| AcError::validation("GIT-COMMAND_FAILED", err.to_string()))?;
        if !merge.status.success() {
            review.conflict_detected = true;
            let _ = Command::new("git")
                .args(["merge", "--abort"])
                .current_dir(&repository.root)
                .output();
            let _ = git_output(&repository.root, ["reset", "--hard", rollback.as_str()]);
            return Err(AcError::conflict(
                "GIT-MERGE_CONFLICT",
                String::from_utf8_lossy(&merge.stderr).trim().to_string(),
            ));
        }
        review.merge_commit = Some(git_output(&repository.root, ["rev-parse", "HEAD"])?);
        self.cleanup_worktree(&review.worktree_id)?;
        review.cleaned_up = true;
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

fn dirty_status_excluding_managed_worktrees(source_root: &Path) -> AcResult<Vec<String>> {
    Ok(git_output(source_root, ["status", "--porcelain"])?
        .lines()
        .filter(|line| !is_managed_worktree_status(line))
        .map(ToString::to_string)
        .collect())
}

fn is_managed_worktree_status(line: &str) -> bool {
    line.get(3..)
        .is_some_and(|path| path.starts_with(".agentcode-worktrees/"))
}

fn canonical_path(path: &Path) -> AcResult<PathBuf> {
    path.canonicalize().map_err(|error| {
        AcError::validation(
            "GIT-CANONICALIZE_PATH",
            format!("{}: {error}", path.display()),
        )
    })
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
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
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
        let checkpoint = git
            .checkpoint_current(&worktree_id, "agent checkpoint")
            .unwrap();
        assert!(git.checkpoint_record(&checkpoint).is_some());
        fs::write(worktree.join("src/lib.rs"), "pub fn broken() {}\n").unwrap();
        git.recover_worktree(&checkpoint).unwrap();
        assert_eq!(
            fs::read_to_string(worktree.join("src/lib.rs")).unwrap(),
            "pub fn new() {}\n"
        );
        assert_eq!(
            fs::read_to_string(source.join("src/lib.rs")).unwrap(),
            "pub fn old() {}\n"
        );
        assert!(git.worktree(&worktree_id).is_some());
        git.cleanup_worktree(&worktree_id).unwrap();
        assert_eq!(
            git.worktree(&worktree_id).unwrap().status,
            WorktreeStatus::Cleaned
        );
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn merge_review_requires_approval_and_cleans_worktree() {
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree = std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("README.md"), "old\n").unwrap();
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
        fs::write(worktree.join("README.md"), "new\n").unwrap();
        git.checkpoint_current(&worktree_id, "ready").unwrap();
        assert_eq!(
            git.prepare_merge_review(&worktree_id, false)
                .unwrap_err()
                .code(),
            "GIT-MERGE_REQUIRES_KERNEL_APPROVAL"
        );
        let mut review = git.prepare_merge_review(&worktree_id, true).unwrap();
        git.complete_merge_review(&mut review).unwrap();
        assert!(review.cleaned_up);
        assert_eq!(
            fs::read_to_string(source.join("README.md")).unwrap(),
            "new\n"
        );
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn merge_conflict_rolls_back_source_repository() {
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree = std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("README.md"), "base\n").unwrap();
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
        fs::write(worktree.join("README.md"), "worktree change\n").unwrap();
        git.checkpoint_current(&worktree_id, "worktree change")
            .unwrap();

        fs::write(source.join("README.md"), "source change\n").unwrap();
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
                "source change",
            ],
        );
        let source_head = git_output(&source, ["rev-parse", "HEAD"]).unwrap();
        let mut review = git.prepare_merge_review(&worktree_id, true).unwrap();
        let error = git.complete_merge_review(&mut review).unwrap_err();
        assert_eq!(error.code(), "GIT-MERGE_CONFLICT");
        assert!(review.conflict_detected);
        assert_eq!(
            git_output(&source, ["rev-parse", "HEAD"]).unwrap(),
            source_head
        );
        assert_eq!(
            fs::read_to_string(source.join("README.md")).unwrap(),
            "source change\n"
        );
        git.cleanup_worktree(&worktree_id).unwrap();
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn phase_seven_isolates_workers_replaces_owner_and_detects_missing_worktree() {
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let one = std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let two = std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&one);
        let _ = fs::remove_dir_all(&two);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "base\n").unwrap();
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
                "base",
            ],
        );
        let mission = StableId::new("mission");
        let worker_a = StableId::new("worker");
        let worker_b = StableId::new("worker");
        let mut git = GitCoordinator::new();
        let first = git
            .create_task_workspace(
                source.clone(),
                one.clone(),
                mission.clone(),
                worker_a.clone(),
            )
            .unwrap();
        let second = git
            .create_task_workspace(
                source.clone(),
                two.clone(),
                mission.clone(),
                worker_b.clone(),
            )
            .unwrap();
        fs::write(one.join("a.txt"), "one\n").unwrap();
        assert!(!git.worktree_status(&second).unwrap().contains("a.txt"));
        let checkpoint = git
            .checkpoint_current_owned(&first, &worker_a, 1, "worker a checkpoint")
            .unwrap();
        let epoch = git
            .transfer_worktree_lease(&first, 1, worker_b.clone())
            .unwrap();
        assert_eq!(
            git.checkpoint_current_owned(&first, &worker_a, 1, "zombie")
                .unwrap_err()
                .code(),
            "GIT-STALE_WORKTREE_LEASE"
        );
        git.recover_worktree(&checkpoint).unwrap();
        assert_eq!(git.worktree(&first).unwrap().lease_epoch, epoch);
        fs::remove_dir_all(&two).unwrap();
        assert_eq!(
            git.reconcile_worktree(&second).unwrap(),
            WorktreeStatus::Missing
        );
        git.cleanup_worktree(&first).unwrap();
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn dirty_base_is_preserved_and_integration_conflicts_are_structured() {
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree = std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "base\n").unwrap();
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
                "base",
            ],
        );
        fs::write(source.join("a.txt"), "manual\n").unwrap();
        let mut git = GitCoordinator::new();
        assert_eq!(
            git.create_task_workspace(
                source.clone(),
                worktree.clone(),
                StableId::new("mission"),
                StableId::new("worker")
            )
            .unwrap_err()
            .code(),
            "GIT-DIRTY_BASE"
        );
        assert_eq!(
            fs::read_to_string(source.join("a.txt")).unwrap(),
            "manual\n"
        );
        run_git(&source, ["restore", "a.txt"]);
        let mission = StableId::new("mission");
        let id = git
            .create_task_workspace(
                source.clone(),
                worktree.clone(),
                mission.clone(),
                StableId::new("worker"),
            )
            .unwrap();
        let repo = git.worktree(&id).unwrap().repository_id.clone();
        git.create_integration_branch(&repo, mission.clone())
            .unwrap();
        let other_path =
            std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&other_path);
        let other = git
            .create_task_workspace(
                source.clone(),
                other_path.clone(),
                mission.clone(),
                StableId::new("worker"),
            )
            .unwrap();
        fs::write(other_path.join("a.txt"), "other\n").unwrap();
        git.checkpoint_current(&other, "other").unwrap();
        assert!(git
            .integrate_worktree(&other, &mission, true)
            .unwrap()
            .is_none());
        fs::write(worktree.join("a.txt"), "task\n").unwrap();
        git.checkpoint_current(&id, "task").unwrap();
        let conflict = git
            .integrate_worktree(&id, &mission, true)
            .unwrap()
            .unwrap();
        assert_eq!(conflict.files, vec!["a.txt"]);
        assert_eq!(fs::read_to_string(source.join("a.txt")).unwrap(), "other\n");
        git.cleanup_worktree(&id).unwrap();
        git.cleanup_worktree(&other).unwrap();
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn managed_worktree_directory_does_not_make_source_base_dirty() {
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree = std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("README.md"), "base\n").unwrap();
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
        fs::create_dir_all(source.join(".agentcode-worktrees/session/src")).unwrap();
        fs::write(
            source.join(".agentcode-worktrees/session/src/lib.rs"),
            "pub fn generated() {}\n",
        )
        .unwrap();
        let mut git = GitCoordinator::new();
        git.create_task_workspace(
            source.clone(),
            worktree.clone(),
            StableId::new("mission"),
            StableId::new("worker"),
        )
        .unwrap();
        fs::write(source.join("user-notes.txt"), "untracked user work\n").unwrap();
        let err = git
            .create_task_workspace(
                source.clone(),
                worktree.join("second"),
                StableId::new("mission"),
                StableId::new("worker"),
            )
            .unwrap_err();
        assert_eq!(err.code(), "GIT-DIRTY_BASE");
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
    }

    // ── BF-02: interrupted-worktree recovery ────────────────────────────────

    /// Build a committed source repository plus a task worktree whose branch
    /// contains a real committed checkpoint of partial completed work.
    fn setup_interrupted_worktree() -> (PathBuf, PathBuf, StableId, StableId, String, String) {
        let source =
            std::env::temp_dir().join(format!("agentcode-rec-src-{}", StableId::new("tmp")));
        let worktree =
            std::env::temp_dir().join(format!("agentcode-rec-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "base\n").unwrap();
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
                "base",
            ],
        );
        let mission = StableId::new("mission");
        let worker = StableId::new("worker");
        let mut git = GitCoordinator::new();
        let worktree_id = git
            .create_task_workspace(
                source.clone(),
                worktree.clone(),
                mission.clone(),
                worker.clone(),
            )
            .unwrap();
        // Simulate completed partial work preserved in the worktree.
        fs::write(worktree.join("a.txt"), "partial completed work\n").unwrap();
        git.checkpoint_current(&worktree_id, "partial progress")
            .unwrap();
        let branch = git_output(&worktree, ["branch", "--show-current"]).unwrap();
        let head = git_output(&worktree, ["rev-parse", "HEAD"]).unwrap();
        (source, worktree, mission, worker, branch, head)
    }

    #[test]
    fn interrupted_worktree_reattaches_after_restart_preserving_work() {
        let (source, worktree, mission, worker, branch, _head) = setup_interrupted_worktree();
        // Simulate a daemon restart: a fresh GitCoordinator has no in-memory
        // state and must recover from the deterministic session path alone.
        let mut restarted = GitCoordinator::new();
        let id = restarted
            .recover_or_create_worktree(
                source.clone(),
                worktree.clone(),
                mission.clone(),
                worker.clone(),
            )
            .unwrap();
        let record = restarted.worktree(&id).unwrap().clone();
        assert_eq!(record.path, worktree);
        assert_eq!(record.branch, branch);
        assert_eq!(record.owner_mission_id, mission);
        assert_eq!(record.owner_worker_id, worker);
        // Partial completed work must survive the restart.
        assert_eq!(
            fs::read_to_string(worktree.join("a.txt")).unwrap(),
            "partial completed work\n"
        );
        // No second worktree may be created for the mission.
        let list = git_output(&source, ["worktree", "list", "--porcelain"]).unwrap();
        let count = list
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count();
        assert_eq!(count, 2, "source repo + one mission worktree only");
        // Source repository must remain untouched.
        assert_eq!(fs::read_to_string(source.join("a.txt")).unwrap(), "base\n");
        assert_eq!(git_output(&source, ["status", "--porcelain"]).unwrap(), "");
        restarted.cleanup_worktree(&id).unwrap();
        let _ = fs::remove_dir_all(&source);
    }

    #[test]
    fn interrupted_worktree_restart_reuses_same_git_worktree() {
        let (source, worktree, mission, worker, _branch, head) = setup_interrupted_worktree();
        let list_before = git_output(&source, ["worktree", "list", "--porcelain"]).unwrap();
        let worktree_entries_before = list_before
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count();
        let mut restarted = GitCoordinator::new();
        let id = restarted
            .recover_or_create_worktree(
                source.clone(),
                worktree.clone(),
                mission.clone(),
                worker.clone(),
            )
            .unwrap();
        assert_eq!(restarted.worktree(&id).unwrap().current_commit, head);
        // The recovered record must describe the existing git worktree, and no
        // duplicate worktree path may appear.
        let list_after = git_output(&source, ["worktree", "list", "--porcelain"]).unwrap();
        let worktree_entries_after = list_after
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count();
        assert_eq!(worktree_entries_before, worktree_entries_after);
        let expected = format!(
            "worktree {}",
            worktree
                .canonicalize()
                .unwrap_or_else(|_| worktree.clone())
                .display()
        );
        assert_eq!(
            list_after
                .lines()
                .filter(|line| *line == expected.as_str())
                .count(),
            1,
            "exactly one worktree entry for the deterministic path"
        );
        restarted.cleanup_worktree(&id).unwrap();
        let _ = fs::remove_dir_all(&source);
    }

    #[test]
    fn interrupted_worktree_rejects_mismatched_repository() {
        let (source, worktree, mission, worker, _branch, _head) = setup_interrupted_worktree();
        // A different repository must never be resumed against this worktree.
        let other =
            std::env::temp_dir().join(format!("agentcode-rec-other-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&other);
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("x.txt"), "other\n").unwrap();
        run_git(&other, ["init"]);
        run_git(&other, ["add", "."]);
        run_git(
            &other,
            [
                "-c",
                "user.name=AgentCode Test",
                "-c",
                "user.email=agentcode@example.test",
                "commit",
                "-m",
                "other",
            ],
        );
        let mut restarted = GitCoordinator::new();
        let err = restarted
            .recover_or_create_worktree(
                other.clone(),
                worktree.clone(),
                mission.clone(),
                worker.clone(),
            )
            .unwrap_err();
        assert_eq!(err.code(), "GIT-WORKTREE_REATTACH_REPOSITORY");
        // Mismatched recovery must leave the worktree untouched.
        assert_eq!(
            fs::read_to_string(worktree.join("a.txt")).unwrap(),
            "partial completed work\n"
        );
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&other);
    }

    #[test]
    fn interrupted_worktree_rejects_unexpected_directory() {
        let source =
            std::env::temp_dir().join(format!("agentcode-rec-src2-{}", StableId::new("tmp")));
        let worktree =
            std::env::temp_dir().join(format!("agentcode-rec-wt2-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "base\n").unwrap();
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
                "base",
            ],
        );
        // An unexpected directory at the deterministic path (not a git
        // worktree) must fail closed and must never be deleted.
        fs::create_dir_all(worktree.join("user-files")).unwrap();
        fs::write(worktree.join("user-files/precious.txt"), "user data\n").unwrap();
        let mut git = GitCoordinator::new();
        let err = git
            .recover_or_create_worktree(
                source.clone(),
                worktree.clone(),
                StableId::new("mission"),
                StableId::new("worker"),
            )
            .unwrap_err();
        assert_eq!(err.code(), "GIT-WORKTREE_REATTACH_PATH");
        assert_eq!(
            fs::read_to_string(worktree.join("user-files/precious.txt")).unwrap(),
            "user data\n"
        );
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
    }

    #[test]
    fn interrupted_worktree_rejects_non_directory_path() {
        let source =
            std::env::temp_dir().join(format!("agentcode-rec-src3-{}", StableId::new("tmp")));
        let worktree =
            std::env::temp_dir().join(format!("agentcode-rec-wt3-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "base\n").unwrap();
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
                "base",
            ],
        );
        // A regular file where the worktree directory is expected.
        fs::write(&worktree, "not a directory\n").unwrap();
        let mut git = GitCoordinator::new();
        let err = git
            .recover_or_create_worktree(
                source.clone(),
                worktree.clone(),
                StableId::new("mission"),
                StableId::new("worker"),
            )
            .unwrap_err();
        assert_eq!(err.code(), "GIT-WORKTREE_REATTACH_NOT_DIR");
        assert_eq!(fs::read_to_string(&worktree).unwrap(), "not a directory\n");
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
    }

    #[test]
    fn recover_or_create_worktree_creates_freshly_when_path_is_absent() {
        let source =
            std::env::temp_dir().join(format!("agentcode-rec-src4-{}", StableId::new("tmp")));
        let worktree =
            std::env::temp_dir().join(format!("agentcode-rec-wt4-{}", StableId::new("tmp")));
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree);
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), "base\n").unwrap();
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
                "base",
            ],
        );
        let mut git = GitCoordinator::new();
        let id = git
            .recover_or_create_worktree(
                source.clone(),
                worktree.clone(),
                StableId::new("mission"),
                StableId::new("worker"),
            )
            .unwrap();
        assert!(git.worktree(&id).is_some());
        assert!(worktree.join("a.txt").exists());
        git.cleanup_worktree(&id).unwrap();
        let _ = fs::remove_dir_all(&source);
    }
}
