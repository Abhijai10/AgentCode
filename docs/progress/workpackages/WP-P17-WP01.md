# WP-P17-WP01 — Finding Schema

Status: ACCEPTED

Objective: Define a unified security finding schema distinct from scanner-specific instances.

Implementation summary: `ac-security` now models `SecurityFindingInstance`, `NormalizedSecurityFinding`, severity, confidence, exploitability, proof level, status, remediation, and evidence refs.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0012_baseline_security.sql`.

Database changes: `security_finding_instances`, `security_findings`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`, `phase17_security_scan_state_survives_reopen`.

Acceptance status: ACCEPTED; P17-G2 covered.
