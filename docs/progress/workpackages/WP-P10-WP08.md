# WP-P10-WP08 — Snapshot Archive

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Retain historical context snapshots for handoff and recovery.
- **Implementation details:** Added `ContextSnapshot`, `archive_snapshot`, snapshot lookup, and durable `context_snapshots`.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** service snapshot lookup; DB snapshot reopen retrieval.
- **Limitations:** Retention/GC policy remains later optimization work.

## Acceptance

ACCEPTED. P10-G11 PASS: historical context snapshots are retrievable.
