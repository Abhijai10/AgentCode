# EXTRACTION-0601 — Codex Git / Worktree Layer

**Campaign:** F — Git & Worktrees (Doc 07 §126)
**Donor:** Codex (OpenAI)
**Pinned SHA:** `77e688960196dbc82bbeb00c844d2555a61925aa` (matches reference catalog)
**License:** Apache-2.0 (LICENSE at repo root; NOTICE file present)
**Classification (overall):** `ADAPT HEAVILY` (Doc 07 §31) — study git interaction, task lifecycle, recovery
**Extraction date:** 20 August 2026

---

## 1. Source Layout

The Git/worktree capability lives in one dedicated crate:

```
codex-rs/git-utils/src/
  lib.rs          module map + public exports
  operations.rs   low-level `git` CLI wrappers
  git_process.rs  process-group/timeout wrapper for git subprocesses
  info.rs         repository metadata, repo-root/worktree resolution, diff-to-remote
  status.rs       dirty-state probe (git status --porcelain)
  branch.rs       merge-base-with-head
  baseline.rs     internal git-baseline repo (diff + destructive reset)
  apply.rs        git apply --3way with preflight and structured parse
  errors.rs       GitToolingError
```

Consumers: `core` (turn metadata enrichment), `memories/write` (baseline diffs), `cloud-tasks-client`, `chatgpt/apply_command`, `app-server`. Entry point is the library's public API surface in `git-utils/src/lib.rs:16-50`.

## 2. Key Symbols (file:line)

### 2.1 Process / command hardening
- `SAFE_BARE_REPOSITORY_CONFIG = "safe.bareRepository=explicit"` — `git-utils/src/lib.rs:14`.
- `run_git(dir, args, env)` — `git-utils/src/operations.rs:94`; always injects `-c safe.bareRepository=explicit` and `-c core.hooksPath=/dev/null` (`NUL` on Windows) before every internal git call (`operations.rs:106-112`).
- `run_git_for_stdout` / `run_git_for_status` — `operations.rs:76` / `operations.rs:63`.
- `run_git_command_with_timeout_output` — `git-utils/src/git_process.rs:96`; process-group kill on drop (`git_process.rs:14-30`), `kill_on_drop(true)`, stdin nulled.
- `GIT_COMMAND_TIMEOUT = 5s` — `git-utils/src/info.rs:45`; `GIT_OPTIONAL_LOCKS=0` set in `info.rs:418`.

### 2.2 Metadata / dirty state
- `collect_git_info(cwd)` — `git-utils/src/info.rs:71`; parallel `rev-parse HEAD`, `rev-parse --abbrev-ref HEAD`, `remote get-url origin`; `GitInfo{commit_hash, branch, repository_url}` at `info.rs:49`.
- `get_has_changes_in_repo(cwd, repo_root)` — `git-utils/src/status.rs:29`; `git status --porcelain`; per-repo deduplicated shared futures (`status.rs:23-79`).
- `current_branch_name` — `info.rs:874` (`git branch --show-current`).
- `default_branch_name` — `info.rs:503` (remote `refs/remotes/<r>/HEAD` → `git remote show` → local `main`/`master`).
- `local_git_branches` — `info.rs:844` (`for-each-ref --format=%(refname:short) refs/heads`).
- `recent_commits` — `info.rs:307` (`git log -n --pretty=format:%H%x1f%ct%x1f%s`).
- `git_diff_to_remote` — `info.rs:354`; closest remote-tracking sha via `find_closest_sha` (`info.rs:681`) + `git diff --no-textconv --no-ext-diff <sha>`; untracked appended per-file via `git diff --no-index --binary -- /dev/null <file>` (`info.rs:723-766`).

### 2.3 Repo-root / worktree resolution (read-only)
- `get_git_repo_root` — `info.rs:35`; walks up for `.git` file/dir. Doc note at `info.rs:31-34`: this does NOT detect `git worktree add` checkouts outside the main repo directory.
- `resolve_root_git_project_for_trust` — `info.rs:775`; resolves a linked worktree back to the main repository root by reading the `.git` file (`gitdir:` → `.git/worktrees/<name>` → common dir), `info.rs:804-821`.

### 2.4 Apply / stage / revert patches
- `apply_git_patch(req)` — `git-utils/src/apply.rs:42`; writes the unified diff to a temp file, runs `git apply --3way` (plus `-R` when reverting), optional `--check` preflight. `ApplyGitRequest{cwd, diff, revert, preflight}` at `apply.rs:19`; `ApplyGitResult{exit_code, applied_paths, skipped_paths, conflicted_paths, stdout, stderr, cmd_for_log}` at `apply.rs:27`.
- `stage_paths` — `apply.rs:326`; `git add --` only paths that exist on disk; best-effort (never hard-fails).
- `extract_paths_from_patch` — `apply.rs:200`; parses `diff --git` headers incl. quoted/C-escaped paths.
- `parse_git_apply_output` — `apply.rs:355`; regex classifier (ported from VS Code) splitting applied/skipped/conflicted.

### 2.5 Baseline (internal diff store) — destructive reset location
- `reset_git_repository(root)` — `git-utils/src/baseline.rs:69`; deletes `.git` metadata (`remove_git_metadata`, `baseline.rs:122`) then re-inits via `gix` and commits the whole current tree as one fresh baseline (`reset_git_repository_sync`, `baseline.rs:94-102`).
- `ensure_git_baseline_repository(root)` — `baseline.rs:78`; preserves an existing usable `.git`, otherwise resets.
- `diff_since_latest_init(root)` — `baseline.rs:105`; gix-based HEAD-tree vs working-tree diff → `GitBaselineChange{Added, Modified, Deleted}` (`baseline.rs:22`) + unified diff.
- Doc comment at `baseline.rs:65-68`:

  > This is intentionally destructive for `root/.git`. It is meant for internal directories where git is used only as a baseline/diff implementation detail, not for user repositories.

- Production callers: `memories/write/src/workspace.rs` — `prepare_memory_workspace` (:15), `memory_workspace_diff` (:28), `reset_memory_workspace_baseline` (:45). Destructive reset is used ONLY on Codex's own memory workspace, never on user repositories.

### 2.6 Branch / merge-base
- `merge_base_with_head(repo_path, branch)` — `git-utils/src/branch.rs:15`; prefers upstream ref when remote is ahead (`resolve_upstream_if_remote_ahead`, `branch.rs:68`).

### 2.7 Worktrees
- Codex does NOT create task worktrees. It runs in-place in the user repo. Worktree support is detection/resolution only (see §2.3) plus tests that run Codex inside a user-created linked worktree:
  - `core/tests/suite/git_enrichment.rs:322` — `concurrent_turns_keep_distinct_worktree_and_repository_metadata` (creates `git worktree add -b feature/...`).
  - `app-server/tests/suite/v2/hooks_list.rs:1022` — linked-worktree hooks resolution.
- `git reset --hard` appears only as an approval-policy test case: `core/tests/suite/approvals.rs:1742-1760` — under `AskForApproval::UnlessTrusted`, the command `git reset --hard` requires approval and is denied in the test. Codex gates the command through the approval policy rather than running it internally.

## 3. Control Flow

### 3.1 Dirty/workspace metadata (per turn)
`core/src/turn_metadata.rs:453-466` — `fetch_workspace_git_metadata` runs `get_head_commit_hash`, `get_git_remote_urls_assume_git_repo`, `get_has_changes_in_repo` in parallel (`tokio::join!`) and stores `WorkspaceGitMetadata{associated_remote_urls, latest_git_commit_hash, has_changes}`.

### 3.2 Apply → revert cycle (edit application)
`apply.rs:42`: preflight (`git apply --check`, dry) → real apply (`git apply --3way`) → parse outcome. Revert path (`apply.rs:50-53`) stages paths first (`stage_paths`) to avoid index mismatch, then `git apply -R`.

### 3.3 Baseline lifecycle (memory workspace only)
`workspace.rs:15` prepare (ensure layout, remove stale diff artifact, ensure baseline repo) → `memory_workspace_diff` (`diff_since_latest_init`) → write bounded diff artifact (`workspace.rs:34`, truncated at `MAX_BYTES`) → `reset_memory_workspace_baseline` deletes the diff artifact and calls `reset_git_repository` (destroys `.git`, re-inits, single baseline commit).

## 4. Lifecycle / State

- No worktree/branch/checkpoint state machine — Codex is a stateless-per-invocation CLI agent with in-place edits.
- Persistent state related to git: memory workspace `.git` baseline (single commit, re-created on each reset); turn metadata enrichment per turn.

## 5. Failure Behavior

- Git command failure → `GitToolingError::GitCommand{command, status, stderr}` (`errors.rs:11`).
- Not a repo → `NotAGitRepository` (`errors.rs:23`); repo detection and metadata functions return `None`/empty rather than erroring (`info.rs:71-79`).
- 5s timeout → command killed via process group; caller sees `None`/empty from the metadata helpers.
- `git apply` conflicts → structured `applied_paths` / `skipped_paths` / `conflicted_paths` in `ApplyGitResult` (`apply.rs:27`); preflight blocks partial application (`apply.rs:815` test).
- Baseline reset failure on a memory directory is a hard error (`anyhow`), because it owns that directory.

## 6. Upstream Tests

- `git-utils/src/status_tests.rs` — sibling-dir status coalescing, symlink alias coalescing.
- `git-utils/src/baseline.rs:528-756` — baseline reset/diff/exec-bit/status-scan tests (incl. `reset_drops_previous_history` at :692).
- `git-utils/src/branch.rs:119-256` — merge-base incl. remote-ahead case.
- `git-utils/src/apply.rs:603-855` — apply/add/conflict/preflight/revert tests.
- `git-utils/src/info.rs:885-1163` — repo detection, remote canonicalization, branch listing, fsmonitor isolation.
- `core/tests/suite/git_enrichment.rs` — turn-metadata workspace enrichment incl. linked worktree and dirty tree.
- `core/tests/suite/approvals.rs:1742-1760` — `git reset --hard` approval gate.
- `core/src/git_info_tests.rs` — dirty-state probe.

## 7. Platform Assumptions

- Unix-first; Windows handled in `git_process.rs` (JobObject) and `operations.rs` (`NUL` hooks path).
- Requires system `git` binary; `gix` (gitoxide) crate used for baseline diff internals.
- fsmonitor: probes and disables fsmonitor helpers for internal metadata queries (`info.rs:402-427`, `fsmonitor.rs`).

## 8. Licensing Implications

- Apache-2.0. Direct source copying requires Apache-2.0 notice preservation (NOTICE file at repo root).
- Recommended use: **pattern study** for Git Engine process hardening, apply/stage, diff-base selection, and dirty probing. Implement independently in AgentCode; do not copy files verbatim (prefer conceptual adaptation, Doc 07 §92).

## 9. Classification

| Mechanism | Class | Reason |
|---|---|---|
| `git` invocation hardening (`safe.bareRepository=explicit`, hooks isolation, 5s timeout, process-group kill, `GIT_OPTIONAL_LOCKS=0`) | ADAPT | Strong operational pattern; adopt into AgentCode Git Engine process runner |
| `git apply --3way` + `--check` preflight + structured applied/skipped/conflicted parse | TAKE (concept) / ADAPT | Directly reusable as ChangeSet application engine pattern |
| `git_diff_to_remote` closest-remote-sha base + untracked no-index diff | ADAPT | Diff-base selection for AgentCode diff service |
| `get_has_changes_in_repo` dirty probe with dedup | ADAPT | Dirty-state signal for worktree status |
| `reset_git_repository` / baseline (destructive `.git` reset) | REJECT for user repos / STUDY for internal store | Destructive `.git` deletion is forbidden by AgentCode policy (Doc 04 §38); only its scoped use on internal dirs is a study input |
| Worktree creation | IGNORE | Codex does not create task worktrees; AgentCode worktree lifecycle comes from OpenHands pattern (EXTRACTION-0602) + Doc 04 §35 |

## 10. AgentCode Destination / Owner

- **Destination:** Git/worktree service (Git Engine) — Doc 04 §33 (Git Engine owns `how`; Kernel owns `when`).
- **Owner boundary:** Kernel-adjacent Git/worktree architecture (locked, AGENTS.md §2).
- `git reset --hard` / `git clean` are approval-gated in Codex and never invoked internally; AgentCode goes further per Doc 04 §38: these are never generic recovery primitives (see MUST-NOT-inherit below).

## 11. MUST-NOT-INHERIT

1. **Destructive baseline reset** (`reset_git_repository`, `baseline.rs:69`) — deleting `.git` metadata and re-initializing to "make the tree clean" is exactly the class of shortcut AgentCode policy forbids (AGENTS.md §8; Doc 04 §38: `git reset --hard`, `git clean -fd/-fdx`, `git checkout -- .` are never generic recovery primitives). AgentCode baselines/checkpoints must never delete `.git` metadata on any repository it does not fully own as a disposable internal store — and even there, only with explicit policy.
2. **Prompt-level `git reset --hard` reliance** — Codex's `/approvals` gate merely *asks the user* about `git reset --hard` (`approvals.rs:1747`). AgentCode must not treat an approval gate as license to use hard reset; the command remains policy-forbidden as a recovery shortcut.
3. **Stateless in-place editing** — Codex commits nothing itself and does not isolate tasks in worktrees; AgentCode requires task worktrees + checkpoints + recovery anchors (Doc 11 H9, Doc 04 §35-37). Do not adopt Codex's "operate directly in the user tree with no durable task state".

## 12. Prototype Required

- Git Engine process-runner prototype: hardened `git` invocation (config injection, hooks isolation, timeout, process-group kill, env scrubbing) — required to prove the operational baseline for all other git features.
- ChangeSet apply prototype: `git apply --3way` + preflight + structured conflict parse — required for edit/rollback correctness (Doc 11 §30 transactional ChangeSet).

## 13. Evidence

- Pinned SHA verified at clone: `77e688960196dbc82bbeb00c844d2555a61925aa` (`git rev-parse HEAD`, 20 Aug 2026).
- Doc 07 campaign context: §27 (Aider), §31 (Codex role), §126 (Campaign F).