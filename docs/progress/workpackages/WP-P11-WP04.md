# WP-P11-WP04 — Sensitivity Filter

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Prevent secret values from entering provider-visible context while preserving retrievable raw evidence refs.
- **Required implementation:** Added `SensitivityClass`, provider-sensitive gating and deterministic redaction for common secret/token/password/key canaries.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`
- **Completion criteria:** Sensitive values are redacted before serialization and redaction counts appear in metrics.
- **Tests:** `context_builder_applies_phase_11_gates`
- **Migrations:** None beyond Phase 11 metrics persistence.
- **Limitations:** Redaction is deterministic pattern-based; richer policy hooks can replace it later through the same boundary.

## Acceptance

ACCEPTED. P11-G4 PASS: secret canary values are filtered from model-visible context.
