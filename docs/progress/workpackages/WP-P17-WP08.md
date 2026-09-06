# WP-P17-WP08 — Checkov

Status: ACCEPTED

Objective: Detect IaC public-exposure issues when infrastructure fixtures are present.

Implementation summary: The Checkov-compatible adapter runs when `include_iac` is true and detects public CIDR/bucket exposure markers.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_finding_instances`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`.

Acceptance status: ACCEPTED; P17-G7 covered.
