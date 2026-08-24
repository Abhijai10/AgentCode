# WP-P10-WP09 — Task Memory

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Persist task-scoped summaries with evidence so replacement workers do not repeat known work.
- **Implementation details:** Added `TaskMemory`, `record_task_memory`, handoff inclusion, and durable `task_memory`.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** handoff includes task memory; DB task memory count survives reopen.
- **Limitations:** Task memory is summary/evidence linked, not completion authority.

## Acceptance

ACCEPTED. P10-G1/P10-G2 PASS for task-scoped memory persistence and evidence linkage.
