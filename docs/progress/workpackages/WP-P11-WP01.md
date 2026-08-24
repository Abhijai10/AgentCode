# WP-P11-WP01 — Context Fragment Model

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Represent model-visible context as typed fragments with source, range, reason, freshness, score, token estimate and sensitivity.
- **Required implementation:** Added `ContextFragment`, `ContextFragmentType`, `SensitivityClass`, `ContextManifest` and build receipts in `ac-context`; persisted manifest provenance in SQLite.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `crates/ac-migrations/src/lib.rs`, `migrations/0006_context_engine.sql`
- **Completion criteria:** Fragment provenance is explicit, manifests record selected/omitted/raw/cache refs, and restart persistence exists.
- **Tests:** `context_builder_applies_phase_11_gates`; `context_engine_receipts_survive_reopen`
- **Migrations:** `0006_context_engine`
- **Limitations:** Token estimates use deterministic fallback arithmetic until provider tokenizer admission.

## Acceptance

ACCEPTED. P11-G10 PASS: context manifest/provenance is persisted and retrievable after reopen.
