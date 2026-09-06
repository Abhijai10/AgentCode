# WP-P12-WP18 — Concurrency

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Admit independent implementation workers conservatively.
- **Implementation summary:** Scheduler admits READY tasks only when resource and conflict checks allow, with a default cap of two implementation workers under normal pressure.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`
- **Migration impact:** Scheduler/admission decisions can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G16 PASS.
