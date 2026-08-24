# WP-P09-WP04 — Python Adapter

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`
- **Architecture refs:** Doc 09 Phase 9.2/9.3; Doc 10 P9-G1..P9-G4; Doc 11 P09-WP04
- **Acceptance gates:** P9-G1..P9-G4 PASS for Python fixture content.

## Evidence

Python files create Python server sessions and normalized definitions for `def`/`class` declarations. Diagnostics and references share the phase-wide semantic graph path.

Validation: `phase9_semantics_survive_lsp_degradation`; full workspace check/test/clippy.

## Handoff

Future external Python LSP integration can feed the same `SemanticEdge` and `SemanticDiagnostic` contracts.
