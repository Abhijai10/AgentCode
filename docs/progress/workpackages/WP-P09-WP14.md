# WP-P09-WP14 — API/Schema Adapters

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`
- **Architecture refs:** Doc 09 Phase 9.10/9.12; Doc 10 P9-G10; Doc 11 P09-WP14
- **Acceptance gates:** P9-G10 PASS.

## Evidence

Implemented route detection for common JS/TS route forms plus schema/env relationship mapping to SQL/migration/schema files. `trace_relationship` can follow route/service/config/schema edges in a representative full-stack fixture.

Validation: `full_stack_trace_uses_semantic_graph_edges`; full workspace check/test/clippy.

## Handoff

Adapters intentionally target high-value patterns first; additional framework-specific adapters should emit the same `SemanticEdge` contract.
