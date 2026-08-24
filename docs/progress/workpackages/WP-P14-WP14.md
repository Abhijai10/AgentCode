# WP-P14-WP14 — completion gate

Status: ACCEPTED

Objective: Ensure Worker completion text cannot bypass Kernel completion authority.

Implementation summary: `completion_gate` permits completion only after a passing final audit, and `AutonomousAgent::request_completion` now calls final audit/completion gate before asking Kernel to transition mission COMPLETE.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-agent/src/lib.rs`.

Database changes: completion decision persists with `final_audits.completion_allowed`.

Tests: `phase14_final_audit_and_completion_gate_block_worker_text_bypass`, existing `simple_coding_task_produces_validated_changeset` and isolated workspace agent tests.

Acceptance status: ACCEPTED; P14-G14 covered.
