# P01-WP06 — Git & Worktrees Adoption Decision

## Decision

AgentCode will build a Kernel-owned Git/Worktree Coordinator. Donor git helpers are evidence sources and implementation references, not authority. Workers receive isolated worktrees and return ChangeSets/evidence; Kernel owns branch creation, checkpoint refs, integration, rollback, and cleanup.

## Mechanism Classifications

| Mechanism | Donor evidence | Classification | Rationale |
|---|---|---:|---|
| Typed worktree lifecycle service | OpenCode `packages/opencode/src/worktree/index.ts` | TAKE | Clear create/list/remove/reset contract with typed errors maps well to AgentCode capability contracts. |
| Global worktree storage root | OpenCode | ADAPT | Use Kernel-managed path layout with repository/worktree ids; do not derive authority from directory placement alone. |
| Slugged branch naming | OpenCode; Munder `agentBranchFor`; Superpowers skill | ADAPT | Use deterministic `codex/<mission>/<worker>` or ADR-approved prefix; include collision handling. |
| Porcelain worktree parsing | OpenCode; Cline `git-worktree.ts` | TAKE | Git porcelain output is stable enough for typed parsing and audit. |
| Primary checkout hidden from agent worktree list | OpenCode | TAKE | Prevents accidental primary workspace reset/removal. |
| Forced branch deletion after worktree removal | OpenCode | REJECT | Destructive and must be Kernel-approved with stale/dirty/ownership preconditions. |
| Reset with status verification | OpenCode | ADAPT | Good recovery path, but only for Kernel-owned disposable worktrees. Never reset primary/user work. |
| Failed-clean pruning under canonicalized root | OpenCode | ADAPT | Useful for cleanup recovery; require symlink-aware root containment and evidence log. |
| `.worktreeinclude` copy | Cline | IGNORE for V1 default | Useful convenience, but implicit copy can leak secrets or config. Revisit as explicit project policy. |
| Worktree lock/detached/bare fields | Cline | TAKE | Needed for lifecycle reconciliation and safe cleanup. |
| Checkpoint stash-compatible commit with untracked third parent | Cline SDK | TAKE | Preserves dirty/untracked user state without mutating real index/stash list. |
| Checkpoints stored in session metadata | Cline SDK | ADAPT | Store in Kernel SQLite control plane; transcript/session metadata must not be source of truth. |
| Dirty-file collection | Aider | TAKE | Distinguishing staged/unstaged/untracked is required before edits, integration, and cleanup. |
| AI commit attribution | Aider; Codex git attribution tests | ADAPT | Attribution is policy-controlled metadata; Kernel applies exactly once when committing. |
| Single committer pattern | Munder Difflin docs | TAKE | Workers cannot rewrite history or push; Kernel integrates. |
| Native tool before raw git fallback | Superpowers skill | ADAPT | In AgentCode, native coordinator is mandatory; raw git is only a Tool Broker implementation detail. |
| Direct git shell helpers | Aider, Cline, Munder | WRAP | Useful command sequences but must run through Process Manager and permission policy. |

## Rejections

- No Worker-owned commits as completion truth.
- No destructive `git reset --hard`, `git clean -ffdx`, `branch -D`, or worktree deletion outside Kernel-approved recovery/cleanup paths.
- No hidden git memory in transcripts; git evidence is persisted as typed state and artifact references.
- No force-merge/ours/theirs shortcut for conflicts.

## Architecture Decision

TAKE OpenCode's worktree lifecycle shape and Cline's checkpoint snapshot technique. ADAPT both under Kernel ownership and Tool Broker execution. WRAP donor command sequences with AgentCode permissions/provenance. REJECT direct donor cleanup authority.

