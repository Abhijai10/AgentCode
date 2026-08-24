# WP-P17-WP09 — Normalization

Status: ACCEPTED

Objective: Convert scanner candidates into normalized findings without automatically confirming scanner output.

Implementation summary: The orchestrator groups scanner instances into normalized findings with root cause, severity, confidence, exploitability, status, evidence refs, and remediation.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `security_findings`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`.

Acceptance status: ACCEPTED; P17-G2 covered.
