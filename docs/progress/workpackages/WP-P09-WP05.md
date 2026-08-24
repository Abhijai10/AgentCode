# WP-P09-WP05 — Rust Adapter

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`
- **Architecture refs:** Doc 09 Phase 9.2/9.3; Doc 10 P9-G1..P9-G4; Doc 11 P09-WP05
- **Acceptance gates:** P9-G1..P9-G4 PASS for Rust fixture content.

## Evidence

Rust files create Rust server sessions and definitions for public/private functions and type declarations. References are normalized into graph edges with source, confidence, freshness, commit and worktree provenance.

Validation: `phase9_semantics_survive_lsp_degradation`; full workspace check/test/clippy.

## Handoff

Rust analyzer can later be admitted as an external process while preserving the current normalized edge API.
