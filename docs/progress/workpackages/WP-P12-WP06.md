# WP-P12-WP06 — Worker Registry

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Maintain durable, role-aware worker records.
- **Implementation summary:** Added `RegisteredWorker`, `WorkerRole` and role registration in `AutonomyKernel`.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`; existing `durable_task_graph_orders_retries_and_recovers_after_reopen`
- **Migration impact:** Existing `workers` table remains authoritative for persisted worker rows; Phase 12 lease/mailbox records reference workers.
- **Acceptance status:** ACCEPTED; P12-G5/P12-G13 support PASS.
