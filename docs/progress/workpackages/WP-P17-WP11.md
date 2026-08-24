# WP-P17-WP11 — LLM Triage

Status: ACCEPTED

Objective: Represent security-verifier triage outcomes, including false-positive dismissal and manual/business-logic findings.

Implementation summary: Deterministic triage marks blocking severities as confirmed, known false-positive fixture paths as false positive, and manual business-logic findings as needs validation.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_findings.status`.

Tests: `phase17_dedup_triage_repair_regression_and_reports_work`.

Acceptance status: ACCEPTED; P17-G9 and P17-G10 covered.
