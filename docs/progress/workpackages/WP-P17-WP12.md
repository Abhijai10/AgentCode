# WP-P17-WP12 — Remediation Tasks

Status: ACCEPTED

Objective: Convert confirmed findings into repair tasks without bypassing Kernel task ownership.

Implementation summary: `create_repair_task` emits a typed security repair task only for confirmed findings, leaving actual execution to the existing Kernel/worker pipeline.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: None.

Tests: `phase17_dedup_triage_repair_regression_and_reports_work`.

Acceptance status: ACCEPTED; P17-G11 covered.
