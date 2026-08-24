# P27-WP09 — Crash logs and diagnostic export

Status: ACCEPTED
Phase: P27
Starting commit: `1f37b15b27e28840215a614d412ee9eeca45b72a`
Risk: REQUIRED_V1

## Objective
Implement Crash logs and diagnostic export as production-integrated release-preparation functionality without creating a parallel subsystem.

## Implementation Summary
Diagnostics report redacts canary secrets before release-visible export.

## Affected Files
- `crates/ac-daemon/src/release.rs`
- `release/packaging/manifest.md`
- `release/update/update-policy.md`
- `release/diagnostics/redaction-policy.md`
- `migrations/0017_security_release.sql`

## Database Changes
- Migration: `migrations/0017_security_release.sql`
- Schema/user version: `17`
- Durable records survive reopen through `ac-db` save/query APIs.

## Tests
- `phase27_release_engineering_verifies_artifact_update_rollback_and_diagnostics`
- `phase27_packaging_layout_keeps_state_outside_user_repository_and_reset_is_safe`
- `phase26_27_security_and_release_state_survive_reopen`
- `security_audit_release_validation_and_artifact_approval_flow`

## Acceptance Status
ACCEPTED. Production-facing typed APIs, persistence, tests and release evidence are present. Final validation is recorded in the Phase 26/27 completion package.
