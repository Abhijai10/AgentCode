# WP-P12-WP01 — Requirement Extraction

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Convert immutable mission goals into structured requirements while preserving the original goal.
- **Implementation summary:** Added `MissionContract::extract` with typed requirement IDs, kind, priority, source, verification strategy and blocking flag.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0007_full_autonomy_kernel.sql`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`; `phase12_autonomy_state_survives_reopen`
- **Migration impact:** `mission_contract_revisions`, `requirement_matrix_entries`
- **Acceptance status:** ACCEPTED; P12-G1 PASS.
