# WP-P11-WP11 — Caching

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Cache stable context content by commit/file/symbol/parameter-style keys.
- **Required implementation:** Added cache keys in manifests, `ContextCacheEntry`, cache derivation from fragments and SQLite cache persistence with upsert.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0006_context_engine.sql`
- **Completion criteria:** Cache entries have content hash, source ref and token estimate, and survive reopen.
- **Tests:** `compression_cache_and_benchmark_are_reported`; `context_engine_receipts_survive_reopen`
- **Migrations:** `0006_context_engine`
- **Limitations:** Provider prompt-cache headers are not emitted yet; stable ordering and cache keys are ready for provider wiring.

## Acceptance

ACCEPTED. P11-G10 PASS: cache/provenance state is durable.
