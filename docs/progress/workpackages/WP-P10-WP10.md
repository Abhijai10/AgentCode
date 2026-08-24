# WP-P10-WP10 — Replacement-Agent Handoff

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Let a replacement model/session reconstruct state from structured memory, decisions and snapshots rather than transcript replay.
- **Implementation details:** Added `HandoffContext` and `export_handoff`, which generates `CONTEXT.md`, archives a snapshot and returns fact/decision/task-memory refs.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** `context_snapshot_and_handoff_are_generated_from_structured_state`; DB snapshot/decision/task-memory reopen test.
- **Limitations:** Provider/model-specific rendering remains Phase 11 context-pack work.

## Acceptance

ACCEPTED. P10-G8/P10-G10/P10-G11 PASS: replacement handoff is generated from structured state with retrievable history.
