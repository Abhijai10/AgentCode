# Phase 0 Governance Validation (2026-08-20 recovery run)

Deterministic validation of the Phase 0 governance artifacts. This run performed the
validation directly (no subagent), by reading the artifacts and checking the rules
against them. The independent understanding check (docs/progress/phase-00-review.md)
already returned PASS on the three questions; this file records the repair of its two
findings and the re-validation of the questions.

## Review findings repaired

1. **Finding 1 — COMPLETE claims referenced nonexistent evidence.**
   `docs/progress/PHASE_STATUS.md` and `roadmap_state.json` claimed P00 and P02
   COMPLETE with evidence files that did not exist. Repairs:
   - The premature P00=COMPLETE and P02=COMPLETE claims were retracted; P02 is now
     NOT_STARTED (no implementation exists — verified: no Cargo.toml, no
     package.json, no lockfiles, no source code in the tree).
   - Phase 0 completion package created: `docs/progress/phase-00-completion.md` +
     `.json` (this run), gates P0-G1..P0-G7 recorded with run results.
   - P00 status set to COMPLETE only in the same commit as the completion package,
     per the tracker's own Phase Completion Rule.
2. **Finding 2 — minor wording tension (AGENTS.md vs docs/adr/README.md).**
   AGENTS.md §2 now says "an approved amendment — see `docs/adr/README.md`
   amendment path" (unified wording); the amendment path description itself
   (proposal ADR → dependent-doc reconciliation → recorded approval + registry bump
   → Doc 10 gate re-runs) is unchanged and consistent in both documents.

## Q1 — Can Kernel/SQLite ownership be casually replaced?

**Expected: NO; architecture amendment/review required.**

Deterministic check (all artifact reads):

- `AGENTS.md` §2 lists "Kernel ownership (mission/task/completion truth)" and
  "SQLite V1 control plane (single authoritative Kernel writer; no UI/provider/
  scanner/Worker direct writes)" under "Locked (do not redesign without an approved
  amendment)".
- `AGENTS.md` §1: code conflicting with locked architecture without an approved
  amendment is "implementation drift — repair it, do not redesign the architecture".
- `docs/adr/README.md` Rule 1: locked architecture requires "an approved amendment
  path, not an ADR"; the amendment path is 4 steps (proposal ADR; reconciliation of
  dependent core docs; approval recorded + registry revision bump; Doc 10 gate
  re-runs).
- `docs/registry/documents.json` DOC-03 authority: "Kernel ownership of
  mission/task/completion truth, SQLite V1 control plane, task leases, single
  authoritative Kernel writer, daemon process, event/state model" — status FINAL
  (locked baseline).
- Doc 10 §34 failure test applied below.

**Verdict: NO — casual replacement is impossible under the recorded rules.**

### Doc 10 failure test (P0 §34)

Question asked of the governance artifacts: *"What should I do if I want to replace
SQLite with Redis?"*

Expected behavior: agent recognizes architecture decision and requires
ADR/review, rather than silently changing it.

Test execution: the SQLite V1 control plane is a locked item in AGENTS.md §2, the
ADR README Rule 1, and documents.json DOC-03 authority. The recorded path for the
change is the locked-architecture amendment path (proposal ADR →
dependent-document reconciliation → approval + registry bump → Doc 10 gate re-runs),
which is an explicit review/ADR requirement.

**Result: PASS (no silent replacement path exists).**

## Q2 — Can arbitrary cloned OSS be copied?

**Expected: NO; extraction + classification + license/provenance/admission rules
apply.**

Deterministic check (all artifact reads):

- `AGENTS.md` §7: the reference-library directory is "research input only — never a
  runtime/build dependency. No committed path may reference it."
- `AGENTS.md` §6: "arbitrary donor code copied without provenance/license review"
  is forbidden while marking work complete.
- `docs/policy/DEPENDENCY_ADMISSION.md`: admission record (DEP-ADM-<NNN>) required;
  vendored source forbidden except by explicit ADR + license review; no
  runtime/build path touching the reference library.
- `docs/legal/LICENSE_REVIEW.md`: every external material passes a recorded license
  review before use; actual license files are inspected (never inferred from
  popularity); classification per matrix; gate table (strong copyleft/unlicensed/
  source-available non-OSS BLOCKED for V1 runtime without approved amendment).
- `docs/legal/OSS_LICENSE_MATRIX.md` Known Exceptions now records daytona (license
  UNKNOWN, BLOCKED_FOR_COPY) and GPL repos (cloudsploit, ScoutSuite, ctags) with
  their justification (P1-G7 readiness).
- `docs/reference/AGENTCODE_REFERENCE_CATALOG.md` rules: every repo has a recorded
  SHA before extraction; dirty repositories are read-only.

**Verdict: NO — copying is admissible only via extraction + classification +
license inspection + admission record + attribution; reference library never a
runtime/build path.**

## Q3 — How does a phase become COMPLETE?

**Expected: required WPs accepted + Doc 10 phase/global gates pass + evidence
current + completion report exists + handoff valid.**

Deterministic check (all artifact reads):

- `docs/progress/PHASE_STATUS.md` Phase Completion Rule: (1) all WPs ACCEPTED,
  (2) Doc 10 phase gates pass with recorded evidence, (3) completion package exists
  (phase-NN-completion.md + .json), (4) failure/understanding check passes where
  Doc 10 requires it. "Evidence wins; file existence alone never marks a phase
  complete."
- `AGENTS.md` §5: WP/phase ACCEPTED/COMPLETE only when Doc 10 gates pass with
  recorded evidence (gate ID, run result, commit, artifacts); evidence classes
  E0–E6; "Never mark a phase COMPLETE to match a target".
- `AGENTS.md` §12: completion package contents = WP acceptance list, gate runs,
  failure tests, evidence refs, review record; independent-understanding check where
  Doc 10 requires it.
- `docs/policy/BRANCH_POLICY.md` rule 6: completion package committed on `main`
  with the phase's final commit.
- Applied to this run: P00's completion package contains WP statuses, P0-G1..G7
  results with commands/artifacts, global-gate applicability, review-finding repair
  record, and the P01 handoff — the four elements above.

**Verdict: PASS — the recorded completion rule is exactly the four-element rule.**

## Overall

- Independent understanding check: PASS (phase-00-review.md).
- Governance re-validation (this run): Q1 PASS, Q2 PASS, Q3 PASS.
- Doc 10 P0 failure test (SQLite→Redis): PASS.
- Findings 1 and 2: repaired (see above).
- P0-G1..P0-G7 gate results with run evidence: `docs/progress/phase-00-completion.md`.
