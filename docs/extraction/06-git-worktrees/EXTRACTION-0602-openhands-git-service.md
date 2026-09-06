# EXTRACTION-0602 — OpenHands Git Service & Conversation Worktrees

**Campaign:** F — Git & Worktrees (Doc 07 §126)
**Donor:** OpenHands
**Pinned SHA:** `b25f9b3969f924f37440fee908ff35309ec6eea2` (from `docs/reference/agentcode_reference_catalog.json` REF-036)
**License:** MIT (LICENSE, repo root); backend git package is in the sibling `software-agent-sdk` (MIT, `LICENSE` at its root)
**Classification (overall):** `ADAPT HEAVILY` (Doc 07 §30) — workspace, git service, event model
**Extraction date:** 20 August 2026

---

## 1. Repo Split at the Pinned SHA

At the pinned SHA the OpenHands repository is the **frontend monorepo** (React/TS). The Python git-service backend that historically lived at `openhands/core/git_service.py` now lives in the sibling reference repo `software-agent-sdk` (also MIT):

```
OpenHands/                                # frontend (this extraction's primary donor SHA)
  src/api/git-service/git-service.api.ts  # frontend GitService client
  src/components/features/controls/git-tools-submenu.tsx

software-agent-sdk/                       # backend git package (same project, separate repo)
  openhands-sdk/openhands/sdk/git/        # git changes/diff/commits/utils
  openhands-agent-server/openhands/agent_server/git_router.py
  openhands-agent-server/openhands/agent_server/conversation_service.py  # worktree creation
```

Both carry MIT license and are treated as one donor for this campaign. This record cites paths from both repos; the pinned SHA applies to the OpenHands repo, and the software-agent-sdk checkout is the matching backend at that point in time.

## 2. Key Symbols (file:line)

### 2.1 Frontend GitService (read-only display layer)
- `class GitService` — `OpenHands/src/api/git-service/git-service.api.ts:34`; cloud repo search/branches/installations (`searchGitRepositories` :35, `getRepositoryBranches` :91, `getUserInstallations` :129), `getGitChanges` (:144) and `getGitChangeDiff` (:163) via `RemoteWorkspace`.
- Git write operations are **not** programmatic: `OpenHands/src/components/features/controls/git-tools-submenu.tsx:34-52` — pull/push/PR/create-branch are implemented by injecting prompt text into the conversation (`setMessageToSend(getGitPushPrompt(...))`). The agent then runs the git command in its shell.

### 2.2 Backend git command runner
- `run_git_command(args, cwd, timeout)` — `software-agent-sdk/openhands-sdk/openhands/sdk/git/utils.py:68`; argv-only (no shell), `--no-pager`, timeout (default 30s), URL-credential redaction in logs/errors (`redact_url_credentials_in_text`), typed `GitCommandError`.
- `validate_git_repository` — `utils.py:495`; resolves path, checks dir exists, `git rev-parse --git-dir`; raises `GitRepositoryError` otherwise.
- `GIT_EMPTY_TREE_HASH` — `utils.py:20`; used as the diff base for unborn/empty repos.
- `get_valid_ref` — `utils.py:331`; base selection for `export` vs `display`; `_get_display_base_ref` (`utils.py:254`) orders candidates: `origin/<branch>` (skipped when pointing at a clean HEAD), `merge-base(HEAD, origin/<default>)`, `origin/<default>`, local `main`/`master` fork point, `HEAD`, empty tree.
- `get_git_repository_metadata` — `utils.py:48`; redacted remote URL + `rev-parse HEAD --abbrev-ref HEAD` (DETACHED sentinel for detached HEAD).

### 2.3 Changes / diff / commits
- `get_git_changes(cwd, ref)` — `software-agent-sdk/openhands-sdk/openhands/sdk/git/git_changes.py:218`; workspace plus nested git dirs; `get_changes_in_repo` (:147) uses `git diff --name-status <ref>` + `git ls-files --others --exclude-standard` for untracked; `_parse_name_status` (:37) splits renames into delete+add, copies to add; unmerged `U` mapped to UPDATED (:24-34).
- `get_git_commits` — `git_commits.py:40`; `git log --no-show-signature --format=%H%x1f%h%x1f%an%x1f%aI%x1f%s`, newline-safe unit separator.
- REST router — `openhands-agent-server/openhands/agent_server/git_router.py`: `/git/changes` (:115), `/git/diff` (:131), `/git/commits` (:156), `/git/commits/{sha}/changes` (:170). SHA validated `^[0-9a-fA-F]{4,64}$` (:40) to prevent option injection. Non-repo workspace → empty result, not an error (:43-55, :58-71).

### 2.4 Cached repository management (clone/fetch/reset)
- `class GitHelper` — `software-agent-sdk/openhands-sdk/openhands/sdk/git/cached_repo.py:30`; `clone` (:37), `fetch` (:73), `checkout` (:97), `get_current_branch` (:123), `get_default_branch` (:144), `get_head_commit` (:177).
- `reset_hard(repo_path, ref)` — `cached_repo.py:110`; runs `git reset --hard <ref>`.
- `try_cached_clone_or_update` — `cached_repo.py:197`; file-lock guarded (`FileLock` + `Timeout`, lock timeout 30s); update sequence documented at :215: `fetch origin -> checkout ref -> reset --hard origin/ref`. `_update_repository` (:328) uses a detached-HEAD optimization to skip fetch for immutable refs; `_recover_from_detached_head` (:421) checks out the default branch. Destructive reset is used only on internal cache repos (skills/plugins), never on user workspaces.

### 2.5 Conversation worktree lifecycle (primary pattern)
All in `software-agent-sdk/openhands-agent-server/openhands/agent_server/conversation_service.py`:

- `_build_worktree_guidance` — :85-101; system-message suffix injected into the agent:

  > This conversation uses a dedicated git worktree. ... Do all file and git work inside this worktree. Do your work on a new, appropriately-named branch, based off the main/master branch, and do not switch back to the original workspace.

- `_get_worktree_start_point` — :148-188; policy: fetch origin (best-effort, warning on failure :166-172) → `refs/remotes/origin/HEAD` → local `main` → local `master` → `HEAD` fallback.
- `_create_conversation_worktree` — :191-248:
  1. `validate_git_repository` + `git rev-parse --show-toplevel` (:198-204); failure → return `None` (conversation proceeds in place).
  2. Path: `<worktree_root>/<conversation_id>/<repo_name>` (:209-210); branch `openhands/<conversation_id>` (:212).
  3. Stale cleanup (:214-226): `git worktree remove --force <path>` → on failure `safe_rmtree(path)`; `git worktree prune`; if branch exists, `git branch -D <branch>`.
  4. Create: `git worktree add -b <branch> <path> <start_point>` (:228-239).
  5. Preserve relative subdir: `workspace_dir = worktree_root / relative_workspace` (:241).
- `_prepare_request_workspace` — :251-274; opt-in via `StartConversationRequest.worktree` (`openhands-sdk/openhands/sdk/conversation/request.py:109-114`); re-points the conversation workspace + injects guidance.
- Config: `conversation_worktree_root` default `/tmp/conversation-worktrees` (`config.py:258-260`, `conversation_service.py:637-639`).
- `safe_rmtree` — imported from `openhands.agent_server.utils` (`conversation_service.py:48`).

## 3. Control Flow

### 3.1 Conversation with worktree
Start request (`worktree: true`) → `_prepare_request_workspace` → `_create_conversation_worktree` (cleanup stale → prune → branch delete → `worktree add -b`) → agent starts with guidance injected into the system message; all edits and git work happen inside the worktree; the original workspace is untouched.

### 3.2 Change display
`GitService.getGitChanges` → `/git/changes` → `get_git_changes` → `get_changes_in_repo` (diff vs display base + untracked listing) → `_parse_name_status` → typed `GitChange` list → frontend status mapper.

## 4. Lifecycle / State

- Worktree lives for the lifetime of the conversation under `/tmp/conversation-worktrees/<conversation_id>/<repo_name>`. A new conversation with the same id re-creates it destructively (`worktree remove --force` + `branch -D` at :214-226).
- Branch naming `openhands/<uuid>` is globally namespaced per conversation, avoiding collisions with user branches.
- The worktree start point is the latest remote default (`origin/<default>`), i.e. latest approved integration state — matching Doc 04 §35 "start from latest approved integration state, not arbitrarily old mission-start commit".

## 5. Failure Behavior

- Non-repo workspace → no worktree created, conversation proceeds in place (`_create_conversation_worktree` returns `None`, :205-206).
- `git fetch origin` failure → warning, cached refs used (:166-172).
- Stale worktree removal failure → fallback `safe_rmtree` (directory delete); still destructive but scoped to the conversation-owned path (:214-221).
- Git router: `GitRepositoryError` → empty response (UI empty state); other `GitError` → 400 with detail; unknown SHA → 400 (`git_router.py:115-181`).

## 6. Upstream Tests

- `OpenHands/__tests__/api/git-service.test.ts`, `agent-server-git-service.test.ts`, `cloud/git-service.test.ts` — frontend service mocks.
- `software-agent-sdk/tests/sdk/git/test_git_changes.py` — incl. worktree shape (`git worktree add -b openhands/conv1 ...` at :709) and orphan-branch cases.
- `software-agent-sdk/tests/sdk/git/test_git_utils.py` — ref selection, URL redaction.
- `software-agent-sdk/tests/sdk/git/test_cached_repo.py` — clone/update/reset behavior.
- `software-agent-sdk/tests/agent_server/test_conversation_service.py` — worktree creation/cleanup (worktree root fixture at :175).
- `software-agent-sdk/tests/agent_server/test_git_router.py` — router incl. orphan HEAD.

## 7. Platform Assumptions

- POSIX-first paths; `/tmp` default worktree root; Linux/Unix agent-server deployment.
- Requires system `git` binary; subprocess `run_git_command` is argv-based (no shell), matching AgentCode's structured-argv requirement (Doc 04 D04-P001).
- Docker sandbox is the historical OpenHands execution model; the git service itself is host/agent-server-side.

## 8. Licensing Implications

- MIT (both repos). Attribution notice required if source is copied; conceptual adaptation carries no notice obligation.
- Recommended use: pattern study + adaptation; do not copy verbatim.

## 9. Classification

| Mechanism | Class | Reason |
|---|---|---|
| Conversation worktree creation (`worktree add -b`, start-point policy, relative-subdir preservation, guidance injection) | ADAPT HEAVILY | Core AgentCode task-worktree lifecycle pattern; merge with Doc 04 §35 |
| Branch naming `openhands/<uuid>` | ADAPT | AgentCode equivalent: `agent/<task_id>`-style namespaced branches |
| Start point `origin/<default>` → `main` → `master` → `HEAD` | ADAPT | Match Doc 04 §35 "latest approved integration state" |
| Stale-worktree cleanup (`worktree remove --force` + `branch -D` + `safe_rmtree`) | ADAPT (guarded) | Pattern for cleanup, but AgentCode requires ownership/dirty checks before deletion (Doc 04 D04-G010 BROKEN state); force/rmtree shortcuts are MUST-NOT-inherit as generic cleanup |
| `get_valid_ref` display/export base selection + empty-tree fallback | ADAPT | Diff-base logic for AgentCode diff service |
| SHA pattern validation + non-repo→empty structured response | TAKE (concept) | Option-injection defense and degraded-state handling |
| argv-only `run_git_command` with timeout + credential redaction | TAKE (pattern) | Aligns with Doc 04 D04-P001 |
| Cached-repo `reset --hard origin/<branch>` update cycle | REJECT as automatic cleanup / STUDY for cache-only context | Destructive reset as an automatic update path is forbidden by AgentCode policy (Doc 04 §38); if a cache needs refreshing, re-clone under explicit policy instead |
| Frontend prompt-injected git write commands (pull/push/branch/PR via `setMessageToSend`) | REJECT | AgentCode routes git through Tool Broker / Git Engine, never through agent prompt text (Doc 04 §1, §33) |

## 10. AgentCode Destination / Owner

- **Destination:** Git/worktree service (Git Engine) — Doc 04 §33; Kernel owns task assignment/integration timing.
- **Owner boundary:** Git/worktree architecture is locked (AGENTS.md §2); this extraction only feeds implementation.

## 11. MUST-NOT-INHERIT

1. **Automatic `git reset --hard`** as part of cache update or any routine path (`cached_repo.py:110, 215, 410-419`). AgentCode policy: `git reset --hard` is never a generic recovery primitive (Doc 04 §38; AGENTS.md §8).
2. **Unconditional stale-worktree destruction** — `git worktree remove --force` + `git branch -D` + `safe_rmtree` (`conversation_service.py:214-226`) without checking for uncertain user work. AgentCode requires: ownership/fencing validation, dirty-state check, structured `BROKEN`/`CLEANUP_PENDING` states, and explicit authorization before deleting any worktree (Doc 04 D04-G010; Doc 11 P01-WP06 failure cases).
3. **Prompt-injected git write commands** (`git-tools-submenu.tsx:34-52`). Agents must not be instructed to run git mutating commands by free text; all git writes go through the Git Engine with policy classification.

## 12. Prototype Required

- **Worktree overlay prototype** (Doc 11 §30): create `agent/<task_id>` worktree from latest approved integration state, run a task inside it, verify isolation (Doc 04 D04-G001), checkpoint, and structured cleanup — the Phase 7 proof target (see ADOPTION-0601).

## 13. Evidence

- Pinned SHA verified at clone: `b25f9b3969f924f37440fee908ff35309ec6eea2` (`git rev-parse HEAD`, 20 Aug 2026); catalog REF-036 `docs/reference/agentcode_reference_catalog.json:761`.
- Doc 07 campaign context: §30 (OpenHands role), §126 (Campaign F).