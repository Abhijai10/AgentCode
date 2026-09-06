# WP-P09-WP10 — Zoekt Spike

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`
- **Architecture refs:** Doc 09 Phase 9.6; Doc 10 conditional Zoekt gate; Doc 11 P09-WP10
- **Acceptance gates:** Conditional Zoekt gate recorded as threshold-gated optional.

## Evidence

Zoekt activation policy is represented as durable `OptionalIndexDecision`: disabled below 10000 indexed files, enabled only above that threshold for future sharded search work.

Validation: `phase9_semantics_survive_lsp_degradation`; DB optional index reopen assertion; full workspace check/test/clippy.

## Handoff

Future benchmarking can tune the threshold and reason string while keeping structural/ripgrep fallback as the default.
