# WP-P17-WP07 — Semgrep

Status: ACCEPTED

Objective: Detect simple SAST injection patterns through a relevant baseline rule path.

Implementation summary: The Semgrep-compatible adapter detects query/sink patterns and emits SAST proof instances into the unified finding pipeline.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_finding_instances`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`, `phase17_dedup_triage_repair_regression_and_reports_work`.

Acceptance status: ACCEPTED; P17-G6 covered.
