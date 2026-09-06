# WP-P12-WP13 — Recovery Engine

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Classify failures into recovery actions.
- **Implementation summary:** Added `FailureClass`, `RecoveryAction`, `recovery_action` and expired-lease recovery messages.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`; `phase12_autonomy_state_survives_reopen`
- **Migration impact:** `autonomy_records`, `autonomy_mailbox_messages`
- **Acceptance status:** ACCEPTED; P12-G7/P12-G10/P12-G11 PASS.
