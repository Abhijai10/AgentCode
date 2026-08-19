# WP-P00-WP04 — License-Review Workflow

- **Phase:** P00
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd
- **Accepted commit:** (recorded in phase-00-completion.md)
- **Owner modules:** docs/legal/
- **Architecture refs:** DOC-10 P0-G5; DOC-11 §19/H-P00; DOC-07 §89–95, §2038
- **Acceptance gates:** P0-G5

## Objective

Real license-review workflow + canonical license artifacts (matrix, notices,
machine-readable manifest).

## Inputs / Outputs

- Inputs: Doc 07 license strategy sections
- Outputs: docs/legal/OSS_LICENSE_MATRIX.md, THIRD_PARTY_NOTICES.md,
  third_party_manifest.json, LICENSE_REVIEW.md, scripts/check-legal-registry.mjs
  (planned), docs/policy/TOOL_REGISTRY.md (ADR-0008 companion)

## Implementation

- Workflow: locate → classify (6 classes) → gate → record → attribute → review.
- Matrix and manifest created; manifest covers Phase 2 build/runtime deps
  (rusqlite, tracing, serde, pino, react, tauri, dev tools).
- Known-exceptions section exists (P1-G7 readiness); currently empty.

## Tests / Evidence

- Manifest schema consistent; classification covers every Phase 2 dependency.
- Phase 2 dependency admissions reference the matrix rows.

## Rollback / Cleanup / Handoff

- Handoff: P01-WP02 populates foundation-source rows from actual license files of
  reference repos.