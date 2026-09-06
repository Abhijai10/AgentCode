# WP-P00-WP02 — ADR Framework

- **Phase:** P00
- **Status:** ACCEPTED (re-validated 2026-08-20 recovery run)
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd
- **Accepted commit:** (recorded in phase-00-completion.md and phase-00-completion.json)
- **Owner modules:** docs/adr/
- **Architecture refs:** DOC-10 P0-G2, P0-G3; DOC-11 §17/H-P00; DOC-07 §2
- **Acceptance gates:** P0-G2, P0-G3

## Objective

ADR directory, template, status model (PROPOSED/ACCEPTED/SUPERSEDED/REJECTED, bounded
PENDING), and the Phase 2-required ADR set.

## Inputs / Outputs

- Inputs: Docs 01–11 (locked architecture statements)
- Outputs: docs/adr/README.md, docs/adr/ADR_TEMPLATE.md, ADR-0001..ADR-0013

## Implementation

- Defined statuses/fields and amendment path for locked architecture.
- Identified the Phase 2-required constrained decisions:
  1. language/package split → ADR-0001
  2. workspace layout → ADR-0002
  3. build/task orchestration → ADR-0003
  4. SQLite driver + migration → ADR-0004
  5. IPC direction (transport bounded-PENDING) → ADR-0005
  6. OmniRoute placement → ADR-0006
  7. desktop stack/boundary → ADR-0007
  8. external binary policy → ADR-0008
  9. structured logging → ADR-0009
  10. configuration layering → ADR-0010
  11. test/fixture organization → ADR-0011
  12. dependency admission → ADR-0012
  13. CI strategy → ADR-0013

## Tests / Evidence

- All 13 ADRs exist with template-mandated sections; registry lists them in
  docs/adr/README.md.
- Bounded PENDING item documented in ADR-0005 (IPC transport, resolution evidence +
  deadline).

## Rollback / Cleanup / Handoff

- ADRs are additive; supersession via new ADR only.
- Handoff: Phase 2 WPs cite ADR IDs; Phase 1 packets may cite ADR-0004/0006/0008.

## 2026-08-20 Recovery Validation

- All 13 ADRs audited (docs/progress/adr-audit-phase00.md): decisions doc-backed; 5 fake extraction citations removed (ADR-0001/0004/0006/0009/0010); ADR-0006 SHA claim fixed; ADR-0013 verification text made future-tense.
- Bounded PENDING item (ADR-0005 transport) retained with deadline.

## Evidence (2026-08-20)

- P0-G2: ADR directory + template + README exist.
- P0-G3: foundational unresolved choices have ADRs; open transport choice has explicit bounded-PENDING marker (ADR-0005).

## Handoff (2026-08-20)

- Phase 2 WPs cite ADR IDs; P01-WP03/04 extraction will re-confirm ADR-0001/0004/0009/0010 evidence.
