# WP-P14-WP13 — Final Audit

Status: ACCEPTED

Objective: Audit final mission state against original goal, requirements, evidence, limitations, and accepted risk.

Implementation summary: `final_audit` rejects missing requirements/evidence, returns repair state on blocking findings, and appends final-audit evidence.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-agent/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `final_audits`.

Tests: `phase14_final_audit_and_completion_gate_block_worker_text_bypass`, `phase14_verification_state_survives_reopen`, existing agent completion tests.

Acceptance status: ACCEPTED; P14-G12 and P14-G13 covered.
