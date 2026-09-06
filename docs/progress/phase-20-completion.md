# Phase 20 Completion - Discuss Mode

Status: COMPLETE
Base commit: af129752b32c6512ad991cd143e91406be1e1e5e

## Accepted Work Packages

- `WP-P20-WP01`
- `WP-P20-WP02`
- `WP-P20-WP03`
- `WP-P20-WP04`
- `WP-P20-WP05`
- `WP-P20-WP06`
- `WP-P20-WP07`
- `WP-P20-WP08`

## Implemented Capabilities

- Durable discussion sessions, messages, decision candidates and promoted plans.
- Repository-grounded discussion answers from Code Intelligence, Context Engine and accepted Memory decisions.
- Read-only Discuss policy that denies workspace writes and process execution by default.
- Discussion-specific provider routing profile with local fallback-friendly defaults.
- Research augmentation with explicit degraded output when optional research is unavailable.
- Accepted decision promotion into MemoryService and mission drafts that inherit decisions without transcript replay.

## Evidence

- `crates/ac-agent/src/discuss.rs`
- `crates/ac-provider/src/lib.rs`
- `crates/ac-db/src/discuss_design.rs`
- `migrations/0014_discuss_design_modes.sql`
- `docs/progress/workpackages/WP-P20-WP01.md` ... `WP-P20-WP08.md`

## Gates

P20-G1..P20-G7 are satisfied by the tests and production-facing APIs listed in WP records.

## Limitations

Optional external research/model dependencies are not required for completion; unavailable research is recorded as a degraded discussion message and local deterministic context remains available.
