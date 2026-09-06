# WP-P09-WP02 — Server Lifecycle

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`, `migrations/0004_semantic_repository_graph.sql`
- **Architecture refs:** Doc 09 Phase 9.1/9.4; Doc 10 P9-G1/P9-G5; Doc 11 P09-WP02
- **Acceptance gates:** P9-G1/P9-G5 PASS via lifecycle session creation, crash marking, bounded restart count, and structural fallback readiness.

## Evidence

Implemented durable lifecycle fields: server type, workspace root, PID, state, restart count, last activity, degraded reason. `mark_lsp_crashed` restarts twice, then marks degraded while `readiness()` remains `StructuralReady`.

Validation: `phase9_semantics_survive_lsp_degradation`; `semantic_graph_evidence_survives_reopen`; full workspace check/test/clippy.

## Handoff

Runtime/resource governors can later replace PID-less embedded sessions with brokered process sessions without changing the persisted contract.
