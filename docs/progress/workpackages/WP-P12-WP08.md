# WP-P12-WP08 — Heartbeats

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Record lightweight worker liveness separately from progress.
- **Implementation summary:** Added `heartbeat` refresh logic that extends lease expiry and rejects non-owner heartbeats.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`; `phase12_autonomy_state_survives_reopen`
- **Migration impact:** `task_leases.heartbeat_interval_ms`, `updated_at_ms`
- **Acceptance status:** ACCEPTED; P12-G6 PASS.
