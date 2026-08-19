# WP-P00-WP06 — Developer Implementation Rules

- **Phase:** P00
- **Status:** ACCEPTED (re-validated 2026-08-20 recovery run)
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd
- **Accepted commit:** (recorded in phase-00-completion.md and phase-00-completion.json)
- **Owner modules:** AGENTS.md
- **Architecture refs:** DOC-10 P0-G7; DOC-11 §21/H-P00, H33–H36
- **Acceptance gates:** P0-G7

## Objective

Top-level developer instructions embedding the source-of-truth hierarchy, no-drift
rule, read-before-write, WP procedure, gate requirement, no-fake-implementations,
dependency/license rules, Git safety, evidence/handoff, no-transcript rule, exact
session startup/shutdown.

## Inputs / Outputs

- Inputs: Doc 11 H34/H35, H28, H4
- Outputs: AGENTS.md

## Implementation

- Wrote concise operational AGENTS.md (12 sections); did not paste core docs into it.

## Tests / Evidence

- Independent Phase-0 understanding check (failure/understanding test) reads only
  governance artifacts and must answer: (a) can Kernel architecture be replaced
  casually? (b) may arbitrary cloned OSS be copied? (c) how does a phase become
  COMPLETE? — recorded in docs/progress/phase-00-review.md.

## Rollback / Cleanup / Handoff

- Handoff: all agents must read AGENTS.md before writing.

## 2026-08-20 Recovery Validation

- AGENTS.md wording unified per phase-00-review.md finding 2 (approved amendment path reference).
- Governance understanding check passed and recorded in docs/progress/phase-00-governance-validation.md.

## Evidence (2026-08-20)

- P0-G7: source-of-truth hierarchy in AGENTS.md; independent understanding check answers Q1/Q2/Q3 correctly (PASS).

## Handoff (2026-08-20)

- All agents must read AGENTS.md before writing.
