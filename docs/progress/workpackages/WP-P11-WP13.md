# WP-P11-WP13 — Benchmark

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Compare broad-context baselines against AgentCode targeted packs for token reduction without success loss.
- **Required implementation:** Added `ContextBenchmarkResult` and persistence for broad/targeted token counts, success booleans, retry/latency deltas and pass/fail result.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0006_context_engine.sql`
- **Completion criteria:** Targeted benchmark passes only when targeted context succeeds, uses fewer tokens than broad baseline and does not increase retries.
- **Tests:** `compression_cache_and_benchmark_are_reported`; `context_engine_receipts_survive_reopen`
- **Migrations:** `0006_context_engine`
- **Limitations:** Benchmark task execution is deterministic in-crate evidence, not repeated live model trials.

## Acceptance

ACCEPTED. P11 benchmark gate PASS: targeted pack shows token reduction with equal success in the deterministic fixture.
