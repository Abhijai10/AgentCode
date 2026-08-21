# WP-P07-WP01 — Git Worktree Execution

- **Phase:** P07
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 62439da380875cc06664b26f5cef9dcff17822c8
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-git`, `crates/ac-agent`, `crates/ac-tool`
- **Architecture refs:** P01 WP06 implementation packet; branch/worktree policy
- **Acceptance gates:** git/worktree/checkpoint/merge gates PASS

## Implemented In This Batch

- Replaced directory-copy task workspace creation with `git worktree add -b`.
- Captured repository default branch, base commit, task branch, worktree path, status, head, and diff.
- Added cleanup support through `git worktree remove --force`.
- Updated autonomous fixture to run in a real git worktree and prove the source repository file remains unchanged.
- Added targeted worktree isolation test with initialized repository and committed base.
- Added current commit tracking on WorktreeRecord.
- Added checkpoint-current, recover-worktree, and checkpoint lookup support.
- Added local merge review objects that require Kernel approval and clean up worktrees after review completion.
- Replaced merge simulation with controlled local git merge, conflict detection, rollback to pre-merge HEAD, and no agent self-approval.
- Added tests for isolated worktree checkpoint/recover/cleanup, successful approved merge, rejected merge, and conflict rollback.
- Added durable WorktreeRecord persistence through `ac-db`.

## Validation

- `cargo fmt` PASS.
- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.

## Known Limitations

- Stale worktree recovery and cleanup policy.
- Branch cleanup beyond worktree removal remains later hardening.

## Acceptance Decision

ACCEPTED for Phase 3: task work happens in real isolated git worktrees, can checkpoint/recover/cleanup, can be merged only after Kernel approval, and rolls back safely on merge conflict.
