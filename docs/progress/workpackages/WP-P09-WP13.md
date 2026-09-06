# WP-P09-WP13 — Tests Relationship

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`
- **Architecture refs:** Doc 09 Phase 9.11; Doc 10 P9-G9; Doc 11 P09-WP13
- **Acceptance gates:** P9-G9 PASS.

## Evidence

Implemented test-to-implementation edges using test file classification, imports, and naming relationships. Edges are marked `test_covers` with lower confidence than direct definitions.

Validation: `phase9_semantics_survive_lsp_degradation`; full workspace check/test/clippy.

## Handoff

Coverage/stack traces can later enrich the same edge kind with stronger provenance.
