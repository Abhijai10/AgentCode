# P01-WP06 Implementation Packet — Git & Worktrees

## Future Interfaces Required

```text
GitCoordinator
  discoverRepository(path) -> RepositoryRecord | NotGit | Degraded
  createWorktree(mission_id, worker_id, base_ref, policy) -> WorktreeRecord
  listWorktrees(repository_id) -> WorktreeInventory
  checkpoint(worktree_id, reason, dirty_policy) -> CheckpointRecord
  diff(worktree_id, base_ref | checkpoint_id) -> DiffRecord
  integrate(worktree_id, target_ref, strategy) -> IntegrationAttempt
  rollback(worktree_id, checkpoint_id) -> RollbackAttempt
  cleanup(worktree_id, cleanup_policy) -> CleanupAttempt
```

## Ownership Boundaries

- Kernel: repository identity, worktree records, branch names, base commit, checkpoint refs, integration decisions, rollback decisions, cleanup lifecycle.
- Worker/AgentSession: temporary cwd, requested git capability, produced diff/ChangeSet, observed command output.
- Tool Broker/Process Manager: executes git with policy, timeout, environment filtering, process cleanup, stdout/stderr capture.
- UI: displays git state and requests approval; never mutates git directly.

## State Ownership

Persist in Kernel SQLite V1:

- `repository_id`, canonical root, current HEAD, remote metadata, default branch.
- `worktree_id`, directory, branch, detached flag, owner mission/worker, base commit, created_at, status.
- `checkpoint_id`, commit/ref, dirty-state summary, untracked snapshot marker, reason, invalidation keys.
- `integration_attempt_id`, source/target refs, merge base, conflict files, result.
- `cleanup_attempt_id`, preconditions, operations requested, result, residual paths.

## Security Constraints

- Git commands are controlled tool invocations.
- Destructive commands require ownership, precondition, and rollback evidence.
- User dirty work must be preserved by default.
- Worktree path containment must canonicalize without following unsafe symlink escapes.
- `GIT_INDEX_FILE` and other git env overrides must be explicit and scrubbed after use.
- Commit messages and branch names are untrusted strings; validate and normalize.

## Required Tests for Implementation Phase

- Parallel worktrees do not share dirty state.
- Manual user edit conflict is preserved and reported.
- Primary checkout reset/removal is rejected.
- Missing worktree directory reconciles to degraded state.
- Worker replacement resumes from Kernel checkpoint.
- Destructive git command denied without Kernel approval.
- Untracked checkpoint capture does not mutate index or stash list.

## Unresolved Questions

- Final branch naming convention and maximum path length policy.
- Whether worktree roots live project-local, global, or per-mission under AgentCode data.
- How much submodule support is required for V1.
- Whether remote fetch is allowed automatically or requires network approval.

