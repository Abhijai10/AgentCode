# WP-P12-WP11 — Loop Detection

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Detect repeated tool/error/diff fingerprints within a window.
- **Implementation summary:** Added progress fingerprints and `repeated_loop_detected`.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Loop events can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G9 PASS.
