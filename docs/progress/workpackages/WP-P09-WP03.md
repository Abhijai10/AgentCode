# WP-P09-WP03 — TypeScript Adapter

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`
- **Architecture refs:** Doc 09 Phase 9.2/9.3; Doc 10 P9-G1..P9-G4; Doc 11 P09-WP03
- **Acceptance gates:** P9-G1..P9-G4 PASS for TypeScript/JavaScript fixture content.

## Evidence

JavaScript/TypeScript files create TypeScript server sessions, definitions from exported functions, references, route diagnostics, API route edges, test mapping and env/schema relationships.

Validation: `phase9_semantics_survive_lsp_degradation`; `full_stack_trace_uses_semantic_graph_edges`; full workspace check/test/clippy.

## Handoff

The adapter emits normalized semantic evidence with confidence below deterministic structural facts when inferred from source patterns.
