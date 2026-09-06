# WP-P17-WP03 — Threat Model

Status: ACCEPTED

Objective: Generate an evidence-backed V1 threat model from repository signals.

Implementation summary: Threat model generation derives entry points, auth boundaries, data stores, admin operations, cloud configuration, and sensitive assets from typed scan input.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `security_threat_models`.

Tests: `phase17_baseline_scan_threat_model_normalizes_and_redacts`, `phase17_security_scan_state_survives_reopen`.

Acceptance status: ACCEPTED; P17-G1 covered.
