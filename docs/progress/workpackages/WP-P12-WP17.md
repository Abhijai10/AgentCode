# WP-P12-WP17 — Blackboard

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Store mission-level blockers, constraints and high-risk findings with scoped retrieval.
- **Implementation summary:** Added `BlackboardEntry`, insertion and relevant-entry retrieval.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Blackboard entries can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G14/P12-G19 support PASS.
