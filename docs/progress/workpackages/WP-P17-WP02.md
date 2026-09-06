# WP-P17-WP02 — Security Orchestrator

Status: ACCEPTED

Objective: Select and run relevant baseline security checks from repository inputs.

Implementation summary: `BaselineSecurityOrchestrator::run` builds a threat model, selects baseline adapters, runs deterministic local scanner fallbacks, normalizes findings, and records adapter health honestly.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `security_scan_reports`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`.

Acceptance status: ACCEPTED; P17-G1 and P17-G2 covered.
