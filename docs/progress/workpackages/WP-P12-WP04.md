# WP-P12-WP04 — DAG Validation

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Reject missing dependencies, self-dependencies and cycles before scheduling.
- **Implementation summary:** Added `validate_plan_dag` with recursive cycle detection over `RuntimePlan` tasks.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`
- **Migration impact:** None.
- **Acceptance status:** ACCEPTED; P12-G3 PASS.
