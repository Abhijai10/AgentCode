# WP-P09-WP09 — SCIP Spike

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`
- **Architecture refs:** Doc 09 Phase 9.5; Doc 10 conditional SCIP gate; Doc 11 P09-WP09
- **Acceptance gates:** Conditional SCIP gate recorded as optional-disabled.

## Evidence

Added `OptionalIndexDecision` with durable `optional_index_decisions`. SCIP remains disabled because no admitted external indexer demonstrates sufficient benefit over the existing structural/LSP-shaped graph in this phase.

Validation: `phase9_semantics_survive_lsp_degradation`; `semantic_graph_evidence_survives_reopen`; full workspace check/test/clippy.

## Handoff

Phase 23 can revisit SCIP with benchmark evidence without changing the decision persistence shape.
