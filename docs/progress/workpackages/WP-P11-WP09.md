# WP-P11-WP09 — Progressive Retrieval

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Allow workers to request additional definitions, references, tests, broader scope, raw output or history without broad prompts.
- **Required implementation:** Added `ProgressiveNeed`, `ProgressiveRetrievalRequest` and durable retrieval records from pack expansion.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0006_context_engine.sql`
- **Completion criteria:** Retrieval requests add matching omitted fragments within remaining budget and record degraded states when no budget remains.
- **Tests:** `context_builder_applies_phase_11_gates`; `context_engine_receipts_survive_reopen`
- **Migrations:** `0006_context_engine`
- **Limitations:** Retrieval operates over supplied candidate fragments; repository search/index expansion is a caller responsibility.

## Acceptance

ACCEPTED. P11-G6 PASS: progressive retrieval works and is persisted.
