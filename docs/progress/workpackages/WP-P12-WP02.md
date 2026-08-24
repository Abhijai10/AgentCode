# WP-P12-WP02 — Requirement Matrix

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Track requirement implementation, verification, evidence and status independently from task status.
- **Implementation summary:** Added `MissionRequirement` fields for implementation/verification status, evidence refs and linked task IDs, with durable matrix rows.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0007_full_autonomy_kernel.sql`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`; `phase12_autonomy_state_survives_reopen`
- **Migration impact:** `requirement_matrix_entries`
- **Acceptance status:** ACCEPTED; P12-G1 PASS.
