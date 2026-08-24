# WP-P17-WP13 — Regression

Status: ACCEPTED

Objective: Verify repaired findings by rescan/regression evidence.

Implementation summary: `regression` compares confirmed root causes against a rescan report and returns structured pass/fail evidence refs.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: None.

Tests: `phase17_dedup_triage_repair_regression_and_reports_work`.

Acceptance status: ACCEPTED; P17-G12 covered.
