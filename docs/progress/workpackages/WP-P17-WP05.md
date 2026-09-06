# WP-P17-WP05 — OSV

Status: ACCEPTED

Objective: Normalize dependency vulnerability findings from baseline dependency scanner input.

Implementation summary: The OSV-compatible adapter detects vulnerable dependency markers and emits dependency proof instances into the unified finding pipeline.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_finding_instances`, `security_findings`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`, `phase17_security_scan_state_survives_reopen`.

Acceptance status: ACCEPTED; P17-G5 covered.
