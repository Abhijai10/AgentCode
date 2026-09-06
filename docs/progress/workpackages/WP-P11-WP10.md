# WP-P11-WP10 — RTK Integration

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Provide deterministic compression for selected command outputs while retaining raw evidence references.
- **Required implementation:** Added `compress_tool_output()` with an RTK-style fallback compressor for tests/build/git/log/listing output and `CompressionReceipt` persistence.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0006_context_engine.sql`
- **Completion criteria:** Compressed output is smaller or equal by estimate, retains important failure lines, and points back to raw evidence.
- **Tests:** `compression_cache_and_benchmark_are_reported`; `context_engine_receipts_survive_reopen`
- **Migrations:** `0006_context_engine`
- **Limitations:** The local `rtk` binary is unavailable in this environment, so AgentCode records `agentcode-rtk-fallback-v1` rather than shelling out.

## Acceptance

ACCEPTED. P11-G8/P11-G9 PASS: raw tool output remains referenced and deterministic compression works.
