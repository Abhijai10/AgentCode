# WP-P10-WP01 — Knowledge Schema

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Persist typed knowledge facts without turning free-form prose into permanent truth.
- **Implementation details:** Added `FactInput`, `MemoryFact`, `FactType`, `FactSource`, `MemoryScope`, `MemoryClass`, and `FreshnessState` in `ac-context`; added SQLite `memory_facts`.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `crates/ac-migrations/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** `memory_requires_evidence_and_supersedes_without_overwrite`; `persistent_memory_survives_reopen_and_source_changes_stale_facts`
- **Limitations:** Structured facts are stored directly; richer retrieval/ranking remains later context-engine scope.

## Acceptance

ACCEPTED. P10-G1 PASS: structured facts persist with typed fields and reopen evidence.
