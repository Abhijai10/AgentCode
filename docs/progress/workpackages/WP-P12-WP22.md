# WP-P12-WP22 — Replanning

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Revise plans without losing prior completed evidence.
- **Implementation summary:** Added `ReplanRecord` and `replan`, preserving old plan IDs and evidence refs while creating a new contract revision.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Replan records can be persisted as `autonomy_records`; contract revisions persist in `mission_contract_revisions`.
- **Acceptance status:** ACCEPTED; P12-G15 PASS.
