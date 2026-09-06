# WP-P00-WP01 — Core Document Registry

- **Phase:** P00
- **Status:** ACCEPTED (re-validated 2026-08-20 recovery run)
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd (initial docs commit)
- **Accepted commit:** (recorded in phase-00-completion.md and phase-00-completion.json)
- **Owner modules:** docs/, docs/registry/
- **Architecture refs:** DOC-10 P0-G1; DOC-11 §16/H-P00
- **Acceptance gates:** P0-G1, P0-G6, P0-G7

## Objective

Canonical registry of Docs 01–11 with stable IDs, paths, statuses, domain authority,
dependencies and superseded-alias fields.

## Inputs / Outputs

- Inputs: docs/coredocs/*.md (existing 11 documents)
- Outputs: docs/coredocs/README.md, docs/registry/documents.json,
  scripts/validate-document-registry.mjs

## Implementation

- Read the 11 core docs (titles/domains/authority) and recorded them in the registry.
- Wrote docs/coredocs/README.md (index + rules) and docs/registry/documents.json
  (schema_version 1, validation rules).
- Wrote registry validator script (duplicate-ID, missing-file, sha256, domain-authority
  checks).

## Tests / Evidence

- `node scripts/validate-document-registry.mjs` → PASS (11 documents).
- Duplicate-ID failure case exercised during development (two entries with same ID
  rejected).

## Failure Cases Covered

- Rename keeps one authority: registry updated in the same commit as renames (rule).
- Missing source is never silently verified: validator flags missing files.

## Rollback / Cleanup / Handoff

- Rollback: revert registry commit; files are additive.
- Handoff: P00-WP02..WP07 may reference registry IDs; Phase 1 packet refs use
  registry IDs.

## 2026-08-20 Recovery Validation

- Registry titles/revisions/dates re-read from the actual document headers (DOC-02..DOC-08 revision corrected 1 -> 2).
- sha256 recorded for all 11 documents; `"verified": true` is now checksum-backed.
- `node scripts/validate-document-registry.mjs` -> PASS (11 documents, 0 errors, 0 warnings).

## Evidence (2026-08-20)

- P0-G1: validator run recorded above; docs/coredocs/README.md index matches documents.json.
- P0-G6: tracker exists (PHASE_STATUS.md, roadmap_state.json).
- P0-G7: source-of-truth hierarchy in AGENTS.md (verified in phase-00-governance-validation.md).

## Handoff (2026-08-20)

- Registry is authoritative for Doc IDs/titles/revisions; Phase 1 packets and ADRs reference these IDs.
