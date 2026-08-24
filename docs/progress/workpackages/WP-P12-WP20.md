# WP-P12-WP20 — Resource Governor

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Delay scheduling under CPU/build/browser/LSP/local-model pressure.
- **Implementation summary:** Added `ResourceSnapshot`, `MemoryPressure` and resource admission checks.
- **Affected files:** `crates/ac-runtime/src/lib.rs`
- **Tests:** `phase12_contract_planner_dag_scheduler_and_controls_work`
- **Migration impact:** Resource snapshots can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G16/P12-G17 support PASS.
