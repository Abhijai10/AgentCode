# WP-P07-WP01 — Git Worktree Execution

- **Phase:** P07
- **Status:** IMPLEMENTING
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 62439da380875cc06664b26f5cef9dcff17822c8
- **Accepted commit:** not accepted yet
- **Owner modules:** `crates/ac-git`, `crates/ac-agent`, `crates/ac-tool`
- **Architecture refs:** P01 WP06 implementation packet; branch/worktree policy
- **Acceptance gates:** git isolation partial; full P07 gates not complete

## Implemented In This Batch

- Replaced directory-copy task workspace creation with `git worktree add -b`.
- Captured repository default branch, base commit, task branch, worktree path, status, head, and diff.
- Added cleanup support through `git worktree remove --force`.
- Updated autonomous fixture to run in a real git worktree and prove the source repository file remains unchanged.
- Added targeted worktree isolation test with initialized repository and committed base.
- Added current commit tracking on WorktreeRecord.
- Added checkpoint-current, recover-worktree, and checkpoint lookup support.
- Added local merge review objects that require Kernel approval and clean up worktrees after review completion.

## Remaining Before Acceptance

- Stale worktree recovery and cleanup policy.
- Durable persisted worktree records beyond in-memory coordinator state.
