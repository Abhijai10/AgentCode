# WP-P12-WP24 — Human Escalation

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Escalate only for credentials, irreversible decisions, production actions, budget limits or persistent blockers.
- **Implementation summary:** Added `HumanEscalation` and guarded `escalate_human` that rejects ordinary retries.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`
- **Migration impact:** Escalations can be persisted as `autonomy_records`.
- **Acceptance status:** ACCEPTED; P12-G7/P12-G8/P12-G9 support PASS.
