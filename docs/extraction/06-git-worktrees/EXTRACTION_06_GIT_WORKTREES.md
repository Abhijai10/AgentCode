# P01-WP06 — Git & Worktrees Extraction

## Scope

Extraction-only campaign for git/worktree lifecycle, branch and commit evidence, dirty-state handling, checkpoints, rollback, conflict preservation, and destructive-git prevention.

## Donors Inspected

| Donor | Catalog ref | Pinned commit | Files inspected |
|---|---:|---|---|
| Codex | REF-013 | `77e688960196dbc82bbeb00c844d2555a61925aa` | `codex-rs/git-utils/src/git_process.rs`; `codex-rs/app-server/tests/suite/v2/git_attribution.rs`; `codex-rs/cli/src/doctor/git.rs`; `codex-rs/core/tests/suite/git_enrichment.rs` |
| OpenCode | REF-034 | `9b0dd36cda0b9accb429a7f9f9ad9b054a27d04a` | `packages/opencode/src/worktree/index.ts`; `packages/opencode/test/project/worktree.test.ts`; `packages/opencode/test/project/worktree-remove.test.ts`; `packages/schema/src/worktree-event.ts` |
| Cline | REF-009 | `8a038022a439f401d78764a059e1561578848e81` | `apps/vscode/src/utils/git-worktree.ts`; `apps/vscode/src/utils/git.ts`; `sdk/packages/core/src/hooks/checkpoint-hooks.ts`; `sdk/packages/core/src/hooks/checkpoint-hooks.test.ts` |
| Aider | REF-002 | `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` | `aider/repo.py`; `tests/basic/test_repo.py`; `tests/basic/test_sanity_check_repo.py` |
| Munder Difflin | REF-029 | `5fb93721030b` | `src/main/git.ts`; `blog/src/posts/single-committer-git-pattern.md`; `blog/src/posts/claude-code-git-worktrees-vs-hive.md` |
| Superpowers | REF-055 | `b36e0829c6d0` | `skills/using-git-worktrees/SKILL.md`; `tests/claude-code/test-worktree-native-preference.sh`; `tests/claude-code/test-worktree-path-policy.sh` |

## Mechanisms

### Worktree Service Lifecycle

OpenCode `packages/opencode/src/worktree/index.ts` defines `Info`, `CreateInput`, `RemoveInput`, `ResetInput`, and typed errors `NotGitError`, `NameGenerationFailedError`, `CreateFailedError`, `StartCommandFailedError`, `RemoveFailedError`, `ResetFailedError`, `ListFailedError`. `Service` exposes `makeWorktreeInfo`, `createFromInfo`, `create`, `list`, `remove`, and `reset`.

Control flow:

1. `makeWorktreeInfo` checks `InstanceState.context.project.vcs === "git"`, creates a global worktree root under `Global.Path.data/worktree/<project.id>`, slugifies a requested name, checks path existence, and checks `refs/heads/opencode/<name>`.
2. `setup` runs `git worktree add --no-checkout -b <branch> <directory>` or detached `git worktree add --no-checkout --detach <directory> HEAD`.
3. `boot` runs `git reset --hard` in the new worktree, loads an `InstanceStore` for that directory, emits `Worktree.Event.Ready` or `Worktree.Event.Failed`, then runs project and worktree start scripts asynchronously.
4. `list` parses `git worktree list --porcelain`, canonicalizes paths, hides the primary checkout, and returns branch-stripped worktree info.
5. `remove` canonicalizes the requested directory, disposes loaded instance state, stops `fsmonitor--daemon`, runs `git worktree remove --force`, checks for stale entries, deletes the directory, and deletes the branch with `git branch -D`.
6. `reset` rejects primary workspace reset, locates the worktree, fetches default remote branch when needed, runs `reset --hard`, `clean -ffdx`, submodule update/reset/clean, verifies `status --porcelain=v1` is empty, and restarts scripts.

Failure behavior:

- Not-git projects become typed errors, not silent success.
- Bootstrap failures emit failed events but do not undo created git state automatically.
- Removal failure is rechecked against a fresh worktree list before returning error.
- `failedRemoves` parses `git clean` warnings and prunes only paths canonicalized under the target root.
- Reset refuses the primary workspace and reports residual local changes as a failure.

Tests inspected:

- `packages/opencode/test/project/worktree.test.ts` covers name generation, slugification, detached worktrees, create/remove lifecycle, ready events, list hiding primary checkout, reset behavior, and non-git error paths.
- `packages/opencode/test/project/worktree-remove.test.ts` covers stale or difficult removal behavior.

AgentCode adaptation:

- TAKE typed lifecycle state and typed error model.
- ADAPT canonical path checks, event emissions, and reset verification under Kernel-owned worktree records.
- REJECT direct `git branch -D` or forced cleanup as a Worker/UI-owned action. AgentCode must route destructive git through Kernel preconditions and Tool Broker.

### Workspace Discovery and Branch Availability

Cline `apps/vscode/src/utils/git-worktree.ts` uses `simple-git` to check git installation, repository status, top-level path, `git worktree list --porcelain`, branch refs, lock state, detached state, and branch availability. `createWorktree` supports new branch, existing branch, or detached checkout; after creation it copies `.worktreeinclude` files. `deleteWorktree` accepts a caller-supplied `force` boolean.

Failure behavior:

- Git missing, non-repo, list/create/delete failures return `{ success: false, message }` or `{ error }`.
- Detached HEAD returns `currentBranch: ""`.
- Branch availability filters branches already attached to worktrees.

Tests inspected:

- `apps/vscode/src/utils/__tests__/git.test.ts` and workspace tests cover parsing and workspace root handling.

AgentCode adaptation:

- TAKE porcelain parsing fields: path, HEAD, branch, bare, detached, locked, lockReason.
- ADAPT `.worktreeinclude` as an optional project policy, not an implicit copy.
- REJECT caller-provided force deletion without Kernel ownership/precondition checks.

### Checkpointing Without Mutating User State

Cline `sdk/packages/core/src/hooks/checkpoint-hooks.ts` implements checkpoint hooks around run/model events. Important symbols: `CheckpointEntry`, `CheckpointMetadata`, `createWorktreeStashCommit`, `createUntrackedParentCommit`, `createCheckpointHooks`, `isCheckpointStashMessage`.

Control flow:

1. `createCheckpointHooks` counts root user runs and skips child-agent/reopened-session duplicates.
2. Checkpoint creation first attempts a stash-compatible commit using `git stash create`.
3. Untracked files are captured through a temporary `GIT_INDEX_FILE`, NUL-delimited pathspec, `git write-tree`, and `git commit-tree`, so the real index and stash list are not disturbed.
4. Metadata is written through caller-provided `readSessionMetadata`/`writeSessionMetadata`.

Failure behavior:

- Malformed checkpoint metadata is ignored.
- Clean worktrees fall back to HEAD commit checkpoints.
- Temp index directories are removed in `finally`.

Tests inspected:

- `sdk/packages/core/src/hooks/checkpoint-hooks.test.ts` verifies one checkpoint per root run, third-parent untracked capture, no disturbance to worktree/index/stash list, prompt-preseed regression, and reopened-session no-op.

AgentCode adaptation:

- TAKE untracked-file snapshot technique and metadata validation.
- ADAPT checkpoint metadata into Kernel state, with checkpoint refs as evidence anchors only.
- REJECT hook-owned checkpoint authority; Worker hooks may request checkpoints, Kernel owns them.

### Commit, Dirty State, and Attribution

Aider `aider/repo.py` `GitRepo.commit`, `get_dirty_files`, `is_dirty`, `get_head_commit_sha`, `path_in_repo`, and `abs_root_path` provide compact dirty-state and attribution mechanics. `get_dirty_files` merges `git diff --name-only --cached` and unstaged `git diff --name-only`. `is_dirty(path)` returns true for paths outside the repo.

Codex `codex-rs/app-server/tests/suite/v2/git_attribution.rs` proves attribution policy is fetched from authenticated workspace settings, cached across failures, updated on account switch, and restored on rollback without duplicating trailers.

Munder Difflin `src/main/git.ts` provides read-only branch/status/log/ahead-behind/diff operations with timeouts and `safeJoin` path validation. It distinguishes staged, unstaged, and untracked data from `git status --porcelain=v1 -z`.

AgentCode adaptation:

- TAKE explicit dirty-state classes and path-in-repo false-as-dirty guard.
- TAKE Codex's principle that git attribution policy is durable session/workspace policy, not prompt text.
- ADAPT Munder's single-committer concept: Workers may produce ChangeSets; Kernel is the only committer/integrator.

### Process Discipline for Git Commands

Codex `codex-rs/git-utils/src/git_process.rs` runs git with scrubbed non-inheritable env vars, null stdin, captured stdout/stderr, timeout, process group/job containment, and process-tree kill on drop.

Failure behavior:

- Spawn failure returns `None`.
- Timeout returns `None` after process tree cleanup.
- Windows uses a job object fallback; Unix uses process groups.

AgentCode adaptation:

- TAKE timeout + process-tree cleanup.
- ADAPT to Tool Broker/Process Manager with structured exit status, stdout/stderr hashes, and command provenance.

## Limitations

- OpenCode's `remove`/`reset` paths are useful but too destructive to copy directly.
- Cline and Aider are largely local-process helpers; they do not enforce AgentCode's Kernel authority.
- Munder Difflin's security posture is documented, but code-level worktree ownership is thinner than OpenCode.
- Superpowers is a workflow skill, not a runtime; use it as policy guidance only.

## AgentCode Requirements Extracted

- Kernel owns `RepositoryRecord`, `WorktreeRecord`, branch, base commit, checkpoint, integration status, and cleanup intent.
- Workers may not treat git state as hidden memory; commits, diffs, and refs are evidence.
- All git commands must go through Tool Broker/Process Manager with allowlisted operations and preconditions.
- Destructive operations require worktree ownership, clean/stale checks, and explicit recovery evidence.
- Checkpoints are rollback anchors, not verification proof.

