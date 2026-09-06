# WP-P09-WP07 — Normalized Definitions/References

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`
- **Architecture refs:** Doc 09 Phase 9.3/9.7/9.8; Doc 10 P9-G2/P9-G3/P9-G7; Doc 11 P09-WP07
- **Acceptance gates:** P9-G2/P9-G3/P9-G7 PASS.

## Evidence

Added `SemanticLocation`, `SemanticEdge`, `SemanticEdgeKind`, and `SemanticProvenance`. Definitions and references normalize to file path, symbol, range, provenance source, confidence, freshness, commit and worktree.

Validation: `phase9_semantics_survive_lsp_degradation`; `full_stack_trace_uses_semantic_graph_edges`; full workspace check/test/clippy.

## Handoff

Consumers must treat confidence/freshness as part of the contract; inferred references do not receive deterministic confidence.
