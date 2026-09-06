# WP-P09-WP08 — Diagnostics

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`
- **Architecture refs:** Doc 09 Phase 9.3/9.4; Doc 10 P9-G4/P9-G5; Doc 11 P09-WP08
- **Acceptance gates:** P9-G4/P9-G5 PASS.

## Evidence

Added `SemanticDiagnostic` and durable `semantic_diagnostics` rows with provenance/confidence/freshness. Malformed/conflict-like source markers become warnings without breaking structural search/indexing.

Validation: full workspace check/test/clippy; DB reopen test for semantic diagnostics path.

## Handoff

Diagnostics are derived evidence and should not be presented as task truth or completion proof.
