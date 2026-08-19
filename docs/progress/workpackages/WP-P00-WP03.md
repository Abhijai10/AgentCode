# WP-P00-WP03 — Dependency Admission Policy

- **Phase:** P00
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd
- **Accepted commit:** (recorded in phase-00-completion.md)
- **Owner modules:** docs/policy/
- **Architecture refs:** DOC-10 P0-G4; DOC-11 §18/H-P00, H12; DOC-07 §89–98
- **Acceptance gates:** P0-G4

## Objective

Written dependency-admission process covering need, alternatives, license, security,
maintenance, cost, install behavior; no curl|sh; no global arbitrary installs.

## Inputs / Outputs

- Inputs: Docs 07, 09, 11 dependency/license sections
- Outputs: docs/policy/DEPENDENCY_ADMISSION.md, ADR-0012

## Implementation

- Admission schema (DEP-ADM-<NNN>) with all required fields.
- License gate table; prohibited patterns (curl|sh, reference-library runtime paths).
- Process steps including `make dependency-check` enforcement.

## Tests / Evidence

- Reviewed against P0-G4 gate text: process exists and is concrete.
- ADR-0012 records the decision class POLICY.

## Rollback / Cleanup / Handoff

- Handoff: Phase 1/2 dependency additions (rusqlite, tracing, pino, tauri...) recorded
  via this process; admission records: docs/policy/dependency-admissions/ (Phase 2
  batch records in phase-02 evidence).