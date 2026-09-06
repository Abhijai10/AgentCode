# ADOPTION-0601 — Task Worktree Strategy for AgentCode

**Campaign:** F — Git & Worktrees (Doc 07 §126, Campaign F extraction phase)
**Classification:** `ADOPT` / `TAKE` (Doc 07 §31) — worktree lifecycle, git hardening, checkpoint store
**Decision date:** 20 August 2026
**Status:** Extraction-phase decision record (research only; no production code). Formal ADR follows in Phase 2 when implementation starts, per ADR rules (`docs/adr/README.md:1-13` — note: Git/worktree architecture is locked, so this document records *how* the locked architecture is implemented, not a re-design).
**Inputs:** EXTRACTION-0601 (Codex), EXTRACTION-0602 (OpenHands), EXTRACTION-0603 (Aider), EXTRACTION-0604 (Opencode), Doc 04 §§30-38, Doc 11 H9/H29, `docs/policy/BRANCH_POLICY.md`.

---

## 1. Decision

AgentCode implements task isolation with a **conversation/task worktree per task** created from the latest approved integration state, a **git-backed checkpoint store** separate from the worktree's `.git`, and a **safe-undo rollback path** that never uses hard reset.

1. **Worktree lifecycle** — adopted and hardened from OpenHands (`EXTRACTION-0602` §2.5): each task gets branch `agent/<task_id>`, created with `git worktree add -b <branch> <path> <start_point>`; start point = `origin/<default>` → `main` → `master` → `HEAD` after a best-effort fetch (doc refs `conversation_service.py:148-188, 191-248`).
2. **Checkpoint store** — adopted concept from opencode (`EXTRACTION-0604` §2.2): separate git dir under the AgentCode data path with `--work-tree` pointing at the task worktree; tree-hash snapshots via `write-tree`; object alternates seeding; 7-day / size-limit pruning.
3. **Safe rollback** — adapted from Aider (`EXTRACTION-0603` §2.5) + opencode per-file revert: undo only commits AgentCode owns (session hash list), soft reset + per-file `git checkout HEAD~1 -- <file>`, refusal preconditions (pushed, multi-parent, dirty files, first commit).
4. **Git process hardening** — adopted from Codex (`EXTRACTION-0601` §2.1): `safe.bareRepository=explicit`, `core.hooksPath` isolation, `GIT_OPTIONAL_LOCKS=0`, timeout + process-group kill, argv-only invocation, `core.quotepath=false` / `autocrlf=false` / `fsmonitor=false`.
5. **ChangeSet application** — `git apply --3way` with `--check` preflight and structured applied/skipped/conflicted parsing (Codex `apply.rs:19-42`).

## 2. Problem / Rationale

- Doc 04 §35 mandates task worktrees with single-writer fencing; §37 mandates checkpoints; §38 forbids `git reset --hard` / `git clean -fdx` / `git checkout -- .` as generic recovery; Doc 11 H9 mandates worktree isolation with latest-integration start points; BRANCH_POLICY mandates main = integration with no force rewrites.
- Donors confirm feasibility: OpenHands runs production conversation worktrees; opencode runs production git-backed checkpoints; Aider runs production safe undo. Codex shows how to run `git` safely at scale.

## 3. Constraints

- No destructive shortcuts: hard reset, `clean -fd/-fdx`, unconditional `checkout-index -a -f`, `worktree remove --force` + `branch -D` without ownership checks are all forbidden as generic cleanup (AGENTS.md §8; Doc 04 §38; D04-G006/G007/G010).
- Kernel owns commit/checkpoint *when*; Git Engine owns *how* (Doc 04 §33).
- Reference library is research input only (AGENTS.md §7); no committed path references it.
- SQLite V1 control plane owns worktree record state (single authoritative Kernel writer, Doc 04 §35-36).

## 4. Alternatives Considered

1. **In-place editing (Codex model, no worktrees)** — REJECTED: no task isolation, no fencing, no crash-safe overlay; conflicts with Doc 04 §35 and Doc 11 H9.
2. **Checkout-directory cloning** (clone the repo per task) — REJECTED: network + disk cost, no shared object store; worktree + alternates achieves the same isolation at near-zero cost.
3. **Commit-on-user-branch with auto-commit (Aider model)** — REJECTED as primary: violates "never commit directly on user branch" and loses isolation; its *safe undo* and *dirty-preservation* semantics are adopted separately.
4. **Snapshot via `git stash` / temp commits on the task worktree** — REJECTED: pollutes task worktree history; separate git-dir store (opencode pattern) keeps the worktree's `.git` pristine and checkpoint storage centralized under AgentCode data path.
5. **Hard-reset-based recovery** — REJECTED at every level: forbidden by policy regardless of donor precedent (OpenHands cache updates, aider `/git reset --hard` hint).

## 5. Adoption Details

- **Where:** Git Engine (locked architecture — AGENTS.md §2 "Git/worktree architecture") implemented per Doc 04 §§33-38.
- **Worktree record:** SQLite-backed per Doc 04 §35, states per Doc 11 P01-WP06 (`BROKEN` handling per D04-G010).
- **Start point policy:** `origin/<default>` → `main` → `master` → `HEAD` (OpenHands `conversation_service.py:148-188`); fetch failure degrades to cached refs with a warning (Doc 04 D04-G008 retry semantics apply).
- **Cleanup:** only after fencing/ownership validation + dirty check; structured `CLEANUP_PENDING`/`BROKEN` states (Doc 04 D04-G010), never unconditional force-remove.
- **Branch naming:** `agent/<task_id>` (adapts OpenHands `openhands/<uuid>`).
- **Checkpoint anchors:** pre-turn and post-turn tree-hash snapshots in the checkpoint store (opencode `processor.ts:102, 436`).
- **Rollback scope:** ChangeSet-scoped (Doc 04 §30): per-file `git checkout <hash> -- <file>` + soft reset, guarded by ownership (Aider `aider_commit_hashes` pattern).
- **Dirty-tree preservation:** commit pre-existing user changes before task edits (Aider `dirty_commit`, `base_coder.py:2411-2423`).
- **Operation classification:** git write operations go through the Git Engine only; never prompt-injected (rejects OpenHands `git-tools-submenu` pattern).

## 6. Consequences

- **Positive:** deterministic isolation; crash-safe overlay; cheap checkpoints; policy-compliant rollback; single integration branch history.
- **Negative/cost:** worktree bookkeeping in SQLite; extra git processes per task; checkpoint store disk growth (mitigated by alternates + 7-day gc + 2MB file cap); degraded non-repo workspaces must degrade gracefully (Doc 04 D04-G010).

## 7. License Obligations

- All four donors are permissive: Codex Apache-2.0, OpenHands MIT, Aider Apache-2.0, Opencode MIT (`EXTRACTION-0601/02/03/04` §8 each).
- Implementation is conceptual adaptation, not verbatim copying; if any source-level reuse occurs, THIRD_PARTY_NOTICES.md entries + third_party_manifest.json records are required (Doc 07 §§93-94). No new runtime dependencies introduced (Doc 07 §19; AGENTS.md §7).

## 8. What Phase 7 Must Prove

Prototype backlog items from Doc 11 §30 (worktree overlays, transactional ChangeSet):

1. **Worktree overlay prototype:** create `agent/<task_id>` from latest integration state; verify Doc 04 D04-G001 isolation; run task edits; verify dirty-user-tree preservation (Aider pattern); structured cleanup. 
2. **Checkpoint prototype:** pre/post-turn tree-hash snapshots in separate git dir; restore guarded by ownership checks; per-file ChangeSet revert.
3. **ChangeSet apply prototype:** `git apply --3way` + preflight + conflict parse (Codex pattern) against the overlay.
4. **Failure drills:** missing worktree → `BROKEN` state (D04-G010); interrupted cleanup → `CLEANUP_PENDING`; degraded non-repo workspace.

## 9. References

- EXTRACTION-0601 (Codex) §9-§12; EXTRACTION-0602 (OpenHands) §2.5, §9, §11-§12; EXTRACTION-0603 (Aider) §2.4-§2.5, §11-§12; EXTRACTION-0604 (Opencode) §2.2-§2.3, §11-§12.
- Doc 04 §30, §33-38; Doc 11 H9, H29, §30; `docs/policy/BRANCH_POLICY.md`.
- Related records: none (first decision record of this campaign).