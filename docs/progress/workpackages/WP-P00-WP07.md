# WP-P00-WP07 — Branch/Integration Policy

- **Phase:** P00
- **Status:** ACCEPTED (re-validated 2026-08-20 recovery run)
- **Risk:** NORMAL | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd
- **Accepted commit:** (recorded in phase-00-completion.md and phase-00-completion.json)
- **Owner modules:** docs/policy/, .github/workflows/ (CI integration)
- **Architecture refs:** DOC-11 §22/H-P00, H9–H11; ADR-0013
- **Acceptance gates:** (Phase 0 validation per Doc 11 §23)

## Objective

Simple branch/worktree policy for independent WPs, dogfooding, user-work preservation,
review/integration, phase checkpoints.

## Inputs / Outputs

- Outputs: docs/policy/BRANCH_POLICY.md

## Implementation

- main = integration; batch/<name>; wp/<wp-id>; normal merges only; no force/reset
  cleanup; integration checklist.

## Tests / Evidence

- This batch runs on batch/phase-0-2-foundation and merges into main with a normal
  merge, exercising the policy.

## Rollback / Cleanup / Handoff

- Handoff: CI and all future batches follow this policy.

## 2026-08-20 Recovery Validation

- Branch policy exists; this run operates on batch/phase-0-2-foundation.
- Integration checklist: merge to main NOT performed in this run (no CI exists yet; merge deferred to a later validated batch per policy rule 5).

## Evidence (2026-08-20)

- P00 validation per Doc 11 §23: policy text present and consistent with ADR-0013 and AGENTS.md.

## Handoff (2026-08-20)

- CI and future batches follow this policy; first merge to main requires `make validate` + CI green.
