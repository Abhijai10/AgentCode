# P24-WP02 — Provider and model fault scenarios

Status: ACCEPTED
Phase: P24
Starting commit: `3cef3d317c77038698c0c5bcb6cf78ec1c58dfbc`
Risk: REQUIRED_V1

## Objective
Implement Provider and model fault scenarios as production-integrated Phase 24 functionality without creating a parallel subsystem.

## Implementation Summary
Provider 429, timeout, bad model output, context exhaustion and free-provider outage scenarios with route/cooldown/blocker recovery expectations.

## Affected Files
- `crates/ac-runtime/src/chaos.rs`
- `crates/ac-db/src/chaos_dogfood.rs`
- `migrations/0016_chaos_dogfood.sql`

## Database Changes
- Migration: `migrations/0016_chaos_dogfood.sql`
- Schema/user version: `16`
- Durable records survive reopen through `ac-db` save/query APIs.

## Tests
- `phase24_chaos_harness_recovers_provider_worker_db_and_verification_faults`
- `phase24_chaos_requires_repeated_runs`
- `phase24_25_chaos_and_dogfood_state_survive_reopen`
- `chaos_and_dogfood_flows_record_recovery_and_verified_self_improvement`

## Acceptance Status
ACCEPTED. Production-facing typed APIs, persistence, tests, and completion evidence are present. Final phase validation is recorded in the Phase 24/25 completion package.
