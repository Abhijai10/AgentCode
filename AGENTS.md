# AgentCode — Developer Instructions

Operational rules for every implementation agent (human or automated) working in this
repository. Read before writing anything.

## 1. Source-of-Truth Hierarchy

1. Current explicit project decision (this file + ADRs)
2. Docs 01–08 (`docs/coredocs/`)
3. Approved ADRs that do not contradict Docs 01–08 (`docs/adr/`)
4. Doc 09 — Master Project Roadmap
5. Doc 10 — Success Definition & Acceptance Gates
6. Doc 11 — Phase-Wise Implementation Playbook
7. Verified Doc 07 extraction / ImplementationPackets (`docs/extraction/`)
8. Existing AgentCode implementation
9. OSS donor behavior (research input only)

If code conflicts with locked architecture and no approved amendment exists, the code
is **implementation drift** — repair it, do not redesign the architecture.

## 2. No Casual Architecture Drift

Locked (do not redesign without an approved amendment — see `docs/adr/README.md`
amendment path):

- Kernel ownership (mission/task/completion truth)
- SQLite V1 control plane (single authoritative Kernel writer; no UI/provider/
  scanner/Worker direct writes)
- OmniRoute / provider architecture
- Tool Broker ownership
- Git/worktree architecture
- Tauri 2 desktop direction
- Code Intelligence authority
- Context authority
- Verification/evidence authority

## 3. Read Before Writing

Before creating or modifying a module: read the relevant core-doc sections, the
current code, and the applicable ImplementationPacket. Never rely on memory of
previous chats; evidence lives in the repo.

## 4. Work Package Procedure

- Every WP has a record in `docs/progress/workpackages/WP-<phase>-<wp>.md`
  (canonical WorkPackageRecord fields per Doc 11 H4).
- WP statuses: PLANNED → READY → IMPLEMENTING → SELF_TESTING → REVIEW → REPAIR →
  ACCEPTED.
- Smallest coherent capability first; format, lint, typecheck, test, inspect diff,
  record evidence, mark ACCEPTED only when gates pass.
- No "batch everything then test at the end".

## 5. Doc 10 Gate Requirement

A WP/phase is ACCEPTED/COMPLETE only when the relevant Doc 10 gates pass with recorded
evidence (gate ID, run result, commit, artifacts). Evidence classes E0–E6 per Doc 10.
File existence alone is never completion. Never mark a phase COMPLETE to match a
target — evidence wins.

## 6. No Fake Implementations

Forbidden while marking work complete: TODO production paths, always-success
placeholders, fake verifier PASS, dummy provider results, mock-only production paths,
uncalled implementations, tests that test only mocks, skipped failing gates without a
recorded blocker, arbitrary donor code copied without provenance/license review.

Every accepted WP must answer: Who calls it? Where is the real production path? Where
is state persisted? What happens on failure? What test proves it? What evidence proves
the accepted commit?

## 7. Dependency and License Rules

- New foundational/runtime dependencies require an admission record
  (`docs/policy/DEPENDENCY_ADMISSION.md`, ADR-0012).
- License review before any external material enters the tree
  (`docs/legal/LICENSE_REVIEW.md`); no license inference from popularity.
- No `curl | sh`; no arbitrary global installs.
- Lockfiles committed (Cargo.lock, pnpm-lock.yaml); CI checks manifest/lock
  consistency.
- The reference-library directory (`/Volumes/T7 Shield/GitHub-Repos-dependency`) is
  **research input only** — never a runtime/build dependency. No committed path may
  reference it.

## 8. Git Safety

- Preserve user work. No `git reset --hard`, `git clean -fdx`, or force-push to make
  the repo look clean.
- Coherent commits/checkpoints with concise messages; stage only intended files;
  never commit secrets or generated garbage.
- Branch/worktree policy: see `docs/policy/BRANCH_POLICY.md` (main = integration;
  WPs on branches/worktrees; merge via normal merge, never force).
- Do not rewrite unrelated working files for stylistic preference.

## 9. Evidence / Handoff Requirements

- Record accepted commit, gate-run IDs, evidence manifests, known limitations,
  rollback/cleanup result and exact handoff for every WP (Doc 11 H28).
- Handoff must let the next agent continue without this conversation's context.

## 10. No Transcript / Hidden Chain-of-Thought Dependency

AgentCode's own architecture prohibits LLM-as-source-of-truth. This repo follows the
same rule: authoritative state lives in docs, registries, ADRs, code and evidence —
never in a chat transcript or hidden reasoning. Session state must be reconstructable
from the repository (Doc 11 H33).

## 11. Session Startup / Shutdown

Startup (Doc 11 H34):

1. `git status`, branch, HEAD — record starting state.
2. Read `docs/progress/PHASE_STATUS.md` + `roadmap_state.json` + the active WP record.
3. Read the relevant core-doc sections + packets + ADRs for the assigned WP.
4. Verify environment: `make bootstrap` prerequisites.

Shutdown (Doc 11 H35):

1. Leave the tree green: format, lint, typecheck, tests pass.
2. Update WP records + phase tracker truthfully; commit evidence.
3. Write exact handoff: what is done, what remains, next commands, known limitations.
4. No hidden state outside the repository.

## 12. Exact Phase Completion

A phase completion package lives at `docs/progress/phase-NN-completion.md` (+ `.json`)
with: WP acceptance list, gate runs, failure tests, evidence refs, review record.
Phase completion requires the independent-understanding check where Doc 10 requires it.