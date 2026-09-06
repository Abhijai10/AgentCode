# WP-P12-WP15 — Verifier Role

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Make Verifier independently schedulable with its own assignment context.
- **Implementation summary:** Added `VerifierAssignment` and `schedule_verifier` requiring a registered Verifier role.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Verifier assignments can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G13 PASS.
