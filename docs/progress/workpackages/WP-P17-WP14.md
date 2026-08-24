# WP-P17-WP14 — Report Exporters

Status: ACCEPTED

Objective: Generate Markdown, JSON, and SARIF reports from normalized security findings.

Implementation summary: `reports` returns redacted Markdown, JSON, and SARIF bundles and `ac-db` persists them by scan id.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0012_baseline_security.sql`.

Database changes: `security_reports`.

Tests: `phase17_dedup_triage_repair_regression_and_reports_work`, `phase17_security_scan_state_survives_reopen`.

Acceptance status: ACCEPTED; P17-G13, P17-G14, and P17-G15 covered.
