# WP-P12-WP23 — Dynamic Task Discovery

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Add newly discovered work as a versioned replan rather than mutating history.
- **Implementation summary:** `replan` creates a new contract revision with a dynamic follow-up requirement and superseding plan.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** New contract revisions and requirement rows persist through Phase 12 tables.
- **Acceptance status:** ACCEPTED; P12-G15 PASS.
