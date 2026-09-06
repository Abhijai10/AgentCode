# WP-P12-WP10 — Stall Detection

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Detect workers with no meaningful progress despite activity.
- **Implementation summary:** Added `stalled_workers` based on active task ownership and meaningful progress timestamps.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Stall events can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G8 PASS.
