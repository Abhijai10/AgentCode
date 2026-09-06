# WP-P17-WP06 — Trivy

Status: ACCEPTED

Objective: Provide dependency/IaC baseline coverage without duplicate execution where OSV/Checkov already apply.

Implementation summary: The orchestrator records Trivy as a baseline adapter and avoids duplicate dependency/IaC findings by routing normalization through shared fingerprints.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_scan_reports.adapters_run`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`.

Acceptance status: ACCEPTED; P17-G5 and P17-G7 covered through shared baseline adapter selection.
