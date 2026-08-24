# P25-WP11 — Long unattended dogfood mission

Status: ACCEPTED
Phase: P25
Starting commit: `3cef3d317c77038698c0c5bcb6cf78ec1c58dfbc`
Risk: REQUIRED_V1

## Objective
Implement Long unattended dogfood mission as production-integrated Phase 25 functionality without creating a parallel subsystem.

## Implementation Summary
Long unattended mission records larger token/time metrics, context compaction and verified final result.

## Affected Files
- `crates/ac-agent/src/dogfood.rs`
- `crates/ac-db/src/chaos_dogfood.rs`
- `migrations/0016_chaos_dogfood.sql`

## Database Changes
- Migration: `migrations/0016_chaos_dogfood.sql`
- Schema/user version: `16`
- Durable records survive reopen through `ac-db` save/query APIs.

## Tests
- `phase25_dogfood_self_mission_creates_verified_changeset_and_evidence`
- `phase25_dogfood_catalog_records_required_mission_metrics`
- `phase25_dogfood_rejects_incomplete_self_mission_input`
- `phase24_25_chaos_and_dogfood_state_survive_reopen`
- `chaos_and_dogfood_flows_record_recovery_and_verified_self_improvement`

## Acceptance Status
ACCEPTED. Production-facing typed APIs, persistence, tests, and completion evidence are present. Final phase validation is recorded in the Phase 24/25 completion package.
