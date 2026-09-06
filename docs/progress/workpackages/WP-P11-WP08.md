# WP-P11-WP08 — Deduplication

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Reduce duplicate source when the same fragment arrives through multiple channels.
- **Required implementation:** Added deterministic source/content normalization and dedupe before final budget selection, with dedupe counts in metrics.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Completion criteria:** Duplicate full source content is represented once while provenance remains in manifests/metrics.
- **Tests:** `context_builder_applies_phase_11_gates`; `context_engine_receipts_survive_reopen`
- **Migrations:** `0006_context_engine`
- **Limitations:** Dedupe is exact-normalized content matching, not semantic clone detection.

## Acceptance

ACCEPTED. P11-G5 PASS: duplicate source is reduced.
