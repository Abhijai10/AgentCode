# WP-P09-WP06 — Go Adapter

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`
- **Architecture refs:** Doc 09 Phase 9.2/9.3; Doc 10 P9-G1..P9-G4; Doc 11 P09-WP06
- **Acceptance gates:** P9-G1..P9-G4 PASS for Go fixture content.

## Evidence

Go files create Go server sessions and normalize `func` definitions through the same semantic graph path used by other required languages.

Validation: `phase9_semantics_survive_lsp_degradation`; full workspace check/test/clippy.

## Handoff

The adapter is intentionally small and source-evidence based; future `gopls` output should map into the existing semantic contracts.
