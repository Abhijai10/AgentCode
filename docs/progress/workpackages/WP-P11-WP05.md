# WP-P11-WP05 — Context Profiles

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Provide dynamic context-size profiles rather than rigid fragment counts.
- **Required implementation:** Added `ContextProfile` values `Tiny`, `Normal`, `Deep`, `Audit`, `Extreme`, each producing a `TokenBudget`.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0006_context_engine.sql`
- **Completion criteria:** Profile selection is part of the request and persisted in manifests.
- **Tests:** `context_builder_applies_phase_11_gates`; `context_engine_receipts_survive_reopen`
- **Migrations:** `0006_context_engine`
- **Limitations:** Profile values are deterministic defaults; provider/model-specific tuning remains future work.

## Acceptance

ACCEPTED. P11-G1/P11-G10 PASS: profile and role are explicit in pack generation and persisted evidence.
