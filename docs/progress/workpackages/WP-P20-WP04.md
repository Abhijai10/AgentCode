# WP-P20-WP04 - Model Routing

Status: ACCEPTED
Base commit: af129752b32c6512ad991cd143e91406be1e1e5e
Phase: P20
Risk: HIGH per playbook

## Objective

Route discussion through a discussion-specific provider profile with fallback-friendly local preference.

## Implementation Summary

TaskProfile::discuss adds repo_discussion routing semantics, structured output, local-first compatibility and cost limits.

## Affected Files

- `crates/ac-agent/src/discuss.rs`
- `crates/ac-agent/src/design.rs`
- `crates/ac-agent/src/lib.rs`
- `crates/ac-provider/src/lib.rs`
- `crates/ac-verification/src/lib.rs`
- `crates/ac-db/src/discuss_design.rs`
- `crates/ac-db/src/models.rs`
- `crates/ac-db/src/migrations.rs`
- `crates/ac-db/src/tests.rs`
- `crates/ac-migrations/src/lib.rs`
- `migrations/0014_discuss_design_modes.sql`

## Database Changes

Migration `migrations/0014_discuss_design_modes.sql` adds durable Discuss and Design Studio tables. `crates/ac-db/src/discuss_design.rs` exposes production-facing save/read APIs and reopen tests cover recovery after restart.

## Tests

- `crates/ac-agent` unit tests cover repo-grounded Discuss answers, read-only denial, decision/plan/mission promotion, design analysis, anti-slop critique, preview iteration, revision history, reference principles and DOM-source mapping.
- `crates/ac-provider` unit test covers Discuss and Design routing profiles.
- `crates/ac-verification` unit test covers evidence-backed visual/responsive/accessibility/functional design QA reports.
- `crates/ac-db` reopen test covers durable Discuss and Design state.

## Acceptance Status

ACCEPTED. Applicable gates `P20-G1..P20-G7` are represented by production-facing APIs and final validation commands recorded in the phase completion package. Optional model/research/visual dependencies degrade to deterministic local behavior with limitations recorded.
