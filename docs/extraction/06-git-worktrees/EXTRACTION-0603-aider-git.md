# EXTRACTION-0603 — Aider Git Integration

**Campaign:** F — Git & Worktrees (Doc 07 §126)
**Donor:** Aider (Paul Gauthier)
**Pinned SHA:** `5dc9490bb35f9729ef2c95d00a19ccd30c26339c`
**License:** Apache-2.0 (LICENSE.txt at repo root; no plain `LICENSE` file)
**Classification (overall):** `STUDY / ADAPT` (Doc 07 §27) — commit/undo semantics, dirty-tree preservation
**Extraction date:** 20 August 2026

---

## 1. Source Layout

```
aider/aider/
  repo.py      GitRepo class (GitPython-based)
  commands.py  /git undo, /git diff, /git commit, /git branch, /git reset
  coders/base_coder.py  auto-commit / dirty-commit integration
```

Entry point: `aider/repo.py` — `GitRepo` class at `aider/aider/repo.py:52`, constructed via GitPython (`git.Repo(..., odbt=git.GitDB)`, `repo.py:125`).

## 2. Key Symbols (file:line)

### 2.1 Repo detection
- `GitRepo(fnames, git_dname, ...)` — `repo.py:52`; walks provided file args → `git.Repo(fname, search_parent_directories=True)` (`repo.py:110`); multiple distinct repos → error (`repo.py:120-122`); none found → `FileNotFoundError` (`repo.py:118`).

### 2.2 Read-only inspection
- `get_dirty_files` — `repo.py:581`; `git diff --name-only --cached` + `git diff --name-only` (tracked changes only).
- `is_dirty` — `repo.py:598`; per-path GitPython `repo.is_dirty(path)`.
- `get_diffs(fnames)` — `repo.py:375`; unborn branch: `diff --cached` + working-dir diff; otherwise `git diff HEAD -- <fnames>`.
- `diff_commits` — `repo.py:419`.
- `get_tracked_files` — `repo.py:433`; HEAD tree traversal + staged entries, minus gitignored.
- `git_ignored_file` — `repo.py:523`; pathspec (`aider_ignore_file`) + `repo.ignored(path)`.
- `subtree_only` — `repo.py:543`; commit scoped to subdirectory.
- `get_head_commit` / `get_head_commit_sha` / `get_head_commit_message` — `repo.py:604` / `:610` / `:618`.

### 2.3 Commit
- `commit(fnames, message, ...)` — `repo.py:131`; only when dirty; message from LLM via `get_commit_message` (`repo.py:326`) or explicit; staged as `git add <fnames>` + `git commit -m <msg> -- <fnames>` (`repo.py:147-149`) else `git commit -a -m`; optional `--no-verify` (`repo.py:278-279`); commit attribution `GIT_COMMITTER_NAME="<user> (aider)"` / `GIT_AUTHOR_NAME` + `Co-authored-by: <user> <email>` trailer (`repo.py:291-314`).
- `set_git_env` — `repo.py:40`; env-var context manager wrapping each command.

### 2.4 Auto-commit and dirty-tree preservation
- `auto_commit(edited_files, context)` — `coders/base_coder.py:2375-2395`; after each LLM turn, commits ONLY the edited files (`repo.commit(fnames=edited, aider_edits=True)`); records the resulting hash in `aider_commit_hashes` and `last_aider_commit_hash` (`base_coder.py:2397-2403`).
- `dirty_commit` — `base_coder.py:2411-2423`; before the first edit of a message, commits the user's pre-existing dirty files (`need_commit_before_edits` set, `base_coder.py:382,2189`), so those changes are never lost and never mixed into the aider edit commit.
- `commit_before_message` — `base_coder.py:112,348,874`; head snapshot recorded per message to bound the undo window.

### 2.5 Undo (safe, non-destructive)
- `cmd_undo(args)` — `commands.py:553`; `raw_cmd_undo(args)` at `commands.py:560`.
- Preconditions (`raw_cmd_undo`, `commands.py:563-621`):
  - last commit must be an aider-session commit (hash in `aider_commit_hashes`) — otherwise refuse with "no recent aider commits to undo" (`:573-574`);
  - refuses first commit in repo (`:578-580`), refuses multi-parent commits (`:581`), refuses if a restored file is currently dirty (`:591-594`), refuses if a file did not exist in the previous commit (`:599-600`), refuses if already pushed to origin (`:615-621`).
- Mechanics: per-file `git checkout HEAD~1 -- <file>` (`:628`), then `git reset --soft HEAD~1` (`:644`).
- Explicit destructive hint kept off the automatic path — `commands.py:576-577`:

  > You could try `/git reset --hard HEAD^` but be aware that this is a destructive command!

## 3. Control Flow

Per message: `commit_before_message` (head snapshot) → first edit triggers `dirty_commit` (user files committed safely) → LLM turn edits → `auto_commit` (only edited files, aider-attributed) → hash recorded in `aider_commit_hashes`. `/git undo` pops exactly one aider commit via soft reset + per-file checkout, guarded by the preconditions in §2.5.

## 4. Lifecycle / State

- In-session, in-memory: `aider_commit_hashes` (list), `last_aider_commit_hash`, `need_commit_before_edits` (set), `commit_before_message` (hash per message).
- No worktrees, no branches, no checkpoint store — aider commits directly on the user's current branch.

## 5. Failure Behavior

- No repo → `FileNotFoundError` at startup; aider falls back to no-git mode.
- Undo refusals are silent/explained failures (see preconditions §2.5) — the command never destroys user work or rewrites history.
- Commit failure → `subprocess.CalledProcessError` propagated; the edit diff is retained for retry.

## 6. Upstream Tests

- `aider/tests/test_repo.py` — commit, diff, tracked/ignored files, dirty state.
- `aider/tests/test_undo.py` — undo preconditions incl. pushed-commit refusal, multi-parent refusal, dirty-file refusal.
- `aider/tests/test_auto_commit.py` — auto-commit after edits, dirty-commit before edits.

## 7. Platform Assumptions

- Python + GitPython (uses system `git`); POSIX/Windows cross-platform via GitPython.
- Commits run with attribution env overrides — attribution of agent commits is a deliberate feature.

## 8. Licensing Implications

- Apache-2.0 (LICENSE.txt). Apache-2.0 attribution required if code is copied; conceptual adaptation (commit/undo policy) carries no notice obligation.

## 9. Classification

| Mechanism | Class | Reason |
|---|---|---|
| Auto-commit edited files only + hash tracking per session | ADAPT | Basis for AgentCode checkpoint/commit policy; track owned hashes to bound undo |
| `dirty_commit` before edits (preserve user work) | TAKE (pattern) | Directly satisfies AGENTS.md §8 "Preserve user work" and Doc 04 §38 |
| Safe undo: soft reset + per-file `checkout HEAD~1`, ownership/precondition guards, pushed-refusal | ADAPT | AgentCode rollback model for Checkpoint/ChangeSet (Doc 04 §30, §37) |
| Commit attribution env (committer/author + Co-authored-by) | ADAPT | Provenance for agent commits |
| Commit scoping (`git commit -- <fnames>` vs `-a`) | TAKE (pattern) | ChangeSet-scoped commits |
| `/git reset --hard HEAD^` destructive hint | REJECT | Never on the automatic path; AgentCode forbids hard reset as a recovery primitive (Doc 04 §38) |
| Worktree/branch management | IGNORE | Aider has none; AgentCode worktree lifecycle comes from OpenHands (EXTRACTION-0602) |

## 10. AgentCode Destination / Owner

- **Destination:** Git Engine commit/checkpoint and undo policy (Doc 04 §30 ChangeSet, §37 checkpoints; Doc 11 H9 worktrees).
- **Owner boundary:** Kernel owns when to commit/checkpoint; Git Engine owns how.

## 11. MUST-NOT-INHERIT

1. **Destructive `git reset --hard`** — even though aider only *suggests* it to the user, the suggestion pattern conflicts with AgentCode policy: hard reset is never a recovery primitive (Doc 04 §38). AgentCode rollback = soft reset / per-file checkout / snapshot restore, never `--hard`.
2. **No isolation** — aider's in-place commit-on-user-branch model must not replace AgentCode's task-worktree requirement (Doc 11 H9, Doc 04 §35). Do not adopt "commit directly on the user's current branch" as the default.

## 12. Prototype Required

- Rollback prototype: safe-undo semantics (ownership of commit hashes, dirty/pushed refusals, per-file restore) against the worktree overlay prototype (Doc 11 §30 transactional ChangeSet). Proves the "restore to last good state without touching user work" acceptance path (Doc 04 D04-G002/G003).

## 13. Evidence

- Pinned SHA verified at clone: `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` (`git rev-parse HEAD`, 20 Aug 2026).
- Doc 07 campaign context: §27 (Aider role), §126 (Campaign F).