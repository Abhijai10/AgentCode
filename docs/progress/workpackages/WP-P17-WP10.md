# WP-P17-WP10 — Deduplication

Status: ACCEPTED

Objective: Group duplicate scanner instances by shared root cause.

Implementation summary: Multiple instances with the same fingerprint collapse into one `NormalizedSecurityFinding` with all affected code, evidence refs, and instance ids retained.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_findings.instance_ids`.

Tests: `phase17_dedup_triage_repair_regression_and_reports_work`.

Acceptance status: ACCEPTED; P17-G8 covered.
