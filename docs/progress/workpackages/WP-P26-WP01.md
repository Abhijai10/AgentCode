# P26-WP01 — Final AgentCode threat model

Status: ACCEPTED
Phase: P26
Starting commit: `1f37b15b27e28840215a614d412ee9eeca45b72a`
Risk: REQUIRED_V1

## Objective
Implement Final AgentCode threat model as production-integrated release-preparation functionality without creating a parallel subsystem.

## Implementation Summary
Security policy and hardening report cover source repos, credentials, Tool Broker, sandbox, MCP, skills/hooks, browser, managed binaries, updates, logs and evidence.

## Affected Files
- `crates/ac-security/src/hardening.rs`
- `docs/security/SECURITY.md`
- `docs/legal/THIRD_PARTY_NOTICES.md`
- `release/SBOM.md`
- `migrations/0017_security_release.sql`

## Database Changes
- Migration: `migrations/0017_security_release.sql`
- Schema/user version: `17`
- Durable records survive reopen through `ac-db` save/query APIs.

## Tests
- `phase26_security_hardening_tracks_dependencies_redacts_secrets_and_blocks_boundaries`
- `phase26_secret_leak_or_unknown_license_blocks_release`
- `phase26_27_security_and_release_state_survive_reopen`
- `security_audit_release_validation_and_artifact_approval_flow`

## Acceptance Status
ACCEPTED. Production-facing typed APIs, persistence, tests and release evidence are present. Final validation is recorded in the Phase 26/27 completion package.
