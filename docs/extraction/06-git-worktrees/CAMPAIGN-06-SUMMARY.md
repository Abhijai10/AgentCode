# CAMPAIGN-06-SUMMARY — Git & Worktrees Extraction

**Campaign:** F — Git & Worktrees (Doc 07 §126)
**Phase:** Extraction (research only; no AgentCode production code)
**Date:** 20 August 2026
**Status:** COMPLETE (extraction records + adoption decision written; Phase 7 design/prototype pending)

---

## 1. Campaign Deliverables

| ID | File | Donor | License |
|----|------|-------|---------|
| EXTRACTION-0601 | `06-git-worktrees/EXTRACTION-0601-codex-git-worktrees.md` | Codex | Apache-2.0 |
| EXTRACTION-0602 | `06-git-worktrees/EXTRACTION-0602-openhands-git-service.md` | OpenHands (+ software-agent-sdk backend) | MIT |
| EXTRACTION-0603 | `06-git-worktrees/EXTRACTION-0603-aider-git.md` | Aider | Apache-2.0 |
| EXTRACTION-0604 | `06-git-worktrees/EXTRACTION-0604-opencode-git.md` | opencode | MIT |
| ADOPTION-0601 | `decisions/ADOPTION-0601-worktree-strategy.md` | — | — |
| SUMMARY | this file | — | — |

Pinned SHAs verified at clone time against the reference library:

- Codex `77e688960196dbc82bbeb00c844d2555a61925aa`
- OpenHands `b25f9b3969f924f37440fee908ff35309ec6eea2` (catalog REF-036)
- Aider `5dc9490bb35f9729ef2c95d00a19ccd30c26339c`
- Opencode `9b0dd36cda0b9accb429a7f9f9ad9b054a27d04a`

## 2. Conclusions (design inputs for Phase 2/7, no re-reading of donors required)

1. **Task isolation = per-task worktree.** The only production worktree lifecycle among the donors is OpenHands' conversation worktree (`conversation_service.py:191-248`): `agent/<task_id>` branch, `git worktree add -b`, start point `origin/<default>` → `main` → `master` → `HEAD` with best-effort fetch, relative-subdir preservation, guidance injected into the agent. Codex and opencode do not create worktrees; aider has no isolation at all. AgentCode adopts OpenHands' lifecycle hardened per Doc 04 §35-36.
2. **Checkpoints live in a separate git dir, not the worktree's `.git`.** Opencode's snapshot store (`snapshot/index.ts:66-347`) is the proven pattern: `--git-dir <data>/snapshot/<project>/<worktree-hash>` + `--work-tree <worktree>`, tree-hash snapshots via `write-tree`, object alternates seeding, 2MB file cap, 7-day gc. Keeps task worktree history clean and stores checkpoints centrally.
3. **Rollback is soft, owned, and scoped.** Aider's safe undo (`commands.py:560-644`) defines the guard rails: undo only commits AgentCode owns (session hash list), refuse pushed/multi-parent/dirty/first commits, per-file `checkout HEAD~1`, then `git reset --soft HEAD~1`. Opencode contributes per-file revert with batching and delete-if-absent. `git reset --hard` is never a recovery primitive (Doc 04 §38).
4. **Git invocation must be hardened.** Codex's `git-utils` shows the operational baseline: `-c safe.bareRepository=explicit`, `-c core.hooksPath=/dev/null`, `GIT_OPTIONAL_LOCKS=0`, 5s timeouts with process-group kill, argv-only invocation, `core.quotepath=false` / `autocrlf=false` / `fsmonitor=false` (opencode). Apply via `git apply --3way` with `--check` preflight and structured applied/skipped/conflicted parse (Codex `apply.rs`).
5. **User work is preserved by design.** Aider's `dirty_commit` (commit pre-existing user dirty files before edits, `base_coder.py:2411-2423`) plus ownership-gated cleanup directly implements AGENTS.md §8.
6. **Git write operations are never prompt-injected.** OpenHands' frontend delegates pull/push/PR/branch to agent prompt text (`git-tools-submenu.tsx:34-52`) — rejected for AgentCode; all writes route through Git Engine (Doc 04 §33).
7. **Degraded handling.** OpenHands' git router returns structured empty results for non-repo workspaces (not errors) and validates SHAs with `^[0-9a-fA-F]{4,64}$` against option injection — adopt both (Doc 04 D04-G010).

## 3. What AgentCode Will NOT Inherit (MUST-NOT list, per record §11)

| Practice | Donor source | Rejection reason |
|---|---|---|
| `git reset --hard` as cache-update/cleanup | OpenHands `cached_repo.py:110,215` | Doc 04 §38; AGENTS.md §8 |
| Destructive `.git` deletion + re-init baseline | Codex `baseline.rs:69` | Same policy class; also kills history |
| Unconditional `worktree remove --force` + `branch -D` + rmtree | OpenHands `conversation_service.py:214-226` | Needs fencing/dirty checks + structured BROKEN/CLEANUP_PENDING states (D04-G010) |
| Unconditional `checkout-index -a -f` restore | Opencode `snapshot/index.ts:382-406` | Only inside owned ChangeSet/checkpoint flow |
| `/git reset --hard` user hint | Aider `commands.py:576-577` | Never offered on any path |
| Prompt-injected git write commands | OpenHands `git-tools-submenu.tsx` | Tool Broker / Git Engine ownership |
| Commit-on-user-branch (no isolation) | Aider default model | Doc 11 H9 worktree requirement |

## 4. License Findings

- **All four donors are permissive; no license blockers.** Codex Apache-2.0, OpenHands MIT, Aider Apache-2.0, Opencode MIT. OpenHands backend git package lives in the sibling `software-agent-sdk` (MIT), not in the pinned OpenHands repo.
- Recommended implementation is **conceptual adaptation** (per Doc 07 §92); verbatim reuse (if any) requires THIRD_PARTY_NOTICES.md + third_party_manifest.json records (Doc 07 §§93-94) and no new runtime dependencies (ADRs README; AGENTS.md §7).

## 5. Prototype Backlog (Phase 7 / Doc 11 §30)

Per ADOPTION-0601 §8: (1) worktree overlay prototype, (2) checkpoint prototype, (3) ChangeSet apply prototype, (4) failure drills (BROKEN / CLEANUP_PENDING / degraded non-repo). Acceptance: Doc 04 D04-G001..G010 with recorded evidence per Doc 10.

## 6. Open Items / Known Limitations

- OpenHands conversation worktree config default `/tmp/conversation-worktrees` is a deployment detail; AgentCode location is a Phase 2 design decision (data-path managed, not /tmp).
- Worktree guidance text (injected system message) wording is OpenHands-specific; AgentCode guidance is defined by Doc 04 §35/§36 semantics.
- Checkpoint store migration/retention policy (7-day gc, 2MB cap) is opencode's; AgentCode retention ties to Doc 11 checkpoint lifecycle and needs Phase 2 confirmation.
- No formal ADR yet (decision record ADOPTION-0601 is extraction-phase; ADR + Doc 10 gate evidence required when implementation starts per `docs/adr/README.md`).

## 7. Handoff

Next step: Phase 2 design of the Git Engine + Worktree/Checkpoint subsystems using EXTRACTION-0601..0604 and ADOPTION-0601 as sole inputs, then Phase 7 prototypes per §5. No further donor reading required for the covered mechanisms.