# WP-P12-WP12 — Retry Controller

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Ensure retries change strategy, model/provider or context.
- **Implementation summary:** Added `RetryDecision` and `retry_decision`, rejecting exact-repeat retries without a strategy change.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Retry decisions can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G10/P12-G11 support PASS.
