# WP-P09-WP11 — Unified Graph

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`, `migrations/0004_semantic_repository_graph.sql`
- **Architecture refs:** Doc 09 Phase 9.7/9.8; Doc 10 P9-G6/P9-G7; Doc 11 P09-WP11
- **Acceptance gates:** P9-G6/P9-G7 PASS.

## Evidence

Structural imports, normalized definitions/references, test mappings, API routes and schema/config relationships are merged into one deduplicated `SemanticEdge` graph with durable persistence.

Validation: `full_stack_trace_uses_semantic_graph_edges`; `semantic_graph_evidence_survives_reopen`; full workspace check/test/clippy.

## Handoff

The graph is derived, rebuildable evidence owned by code intelligence; Kernel truth remains separate.
