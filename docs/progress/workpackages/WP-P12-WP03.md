# WP-P12-WP03 — Planner Schema

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Validate structured Planner output without letting Planner declare mission completion.
- **Implementation summary:** Added `PlannerTaskProposal` and `RuntimePlan::from_contract` with dependencies, risk, task type, acceptance criteria, skills and context profile.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`
- **Migration impact:** Durable plan evidence is captured in autonomy records/matrix rows.
- **Acceptance status:** ACCEPTED; P12-G2 PASS.
