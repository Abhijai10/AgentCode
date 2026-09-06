# WP-P17-WP04 — Gitleaks

Status: ACCEPTED

Objective: Detect synthetic secret exposures and redact raw values before model/report exposure.

Implementation summary: The Gitleaks-compatible adapter path identifies secret-like lines, creates secret proof findings, and stores only redacted evidence in normalized output.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_finding_instances.redacted_evidence`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`, `phase17_dedup_triage_repair_regression_and_reports_work`.

Acceptance status: ACCEPTED; P17-G3 and P17-G4 covered.
