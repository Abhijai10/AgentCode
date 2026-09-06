# WP-P12-WP07 — Leases

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Fence task assignments with leases so only one live worker owns a task.
- **Implementation summary:** Added `TaskLease`, `acquire_lease`, lease epochs, expiry and zombie write rejection.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0007_full_autonomy_kernel.sql`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`; `phase12_autonomy_state_survives_reopen`
- **Migration impact:** `task_leases`
- **Acceptance status:** ACCEPTED; P12-G5/P12-G7 PASS.
