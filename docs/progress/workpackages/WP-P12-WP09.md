# WP-P12-WP09 — Progress Model

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Distinguish meaningful progress from routine tool activity.
- **Implementation summary:** Added `ProgressEvent` and `ProgressKind`, with meaningful-progress filtering for stall checks.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Progress can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G8 support PASS.
