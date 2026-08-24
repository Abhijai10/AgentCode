# WP-P12-WP05 — Scheduler

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Admit only dependency-ready tasks using conservative resource/conflict checks.
- **Implementation summary:** Added `AutonomyKernel::ready_tasks` and `schedule` returning structured `SchedulerDecision` records.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`
- **Migration impact:** Scheduler decisions can be persisted through `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G4 PASS.
