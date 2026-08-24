# WP-P12-WP21 — Pause/Resume/Cancel

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Pause scheduling, resume with reconciliation, and cancel without uncontrolled workers.
- **Implementation summary:** Added `MissionControlState`, `pause`, `resume` and `cancel`, including resume reconciliation blackboard entry and lease cancellation.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`
- **Migration impact:** Control actions can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G18/P12-G19/P12-G20 PASS.
