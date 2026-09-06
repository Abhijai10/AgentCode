# WP-P22-WP07 - Progress model

Status: ACCEPTED
Base commit: 49ae6f0d0d88c964f9b67723b6ddcd957362803e
Phase: P22
Risk: HIGH per playbook

## Objective

Truthful progress without invented percentage.

## Implementation Summary

Projection exposes phase/action/waiting/no-action-required instead of synthetic percent.

## Affected Files

- `crates/ac-agent/src/desktop.rs`
- `crates/ac-agent/src/lib.rs`
- `crates/ac-daemon/src/lib.rs`
- `crates/ac-runtime/src/optimization.rs`
- `crates/ac-runtime/src/lib.rs`
- `crates/ac-runtime/src/tests.rs`
- `crates/ac-context/src/lib.rs`
- `crates/ac-provider/src/lib.rs`
- `crates/ac-db/src/desktop_optimization.rs`
- `crates/ac-db/src/models.rs`
- `crates/ac-db/src/migrations.rs`
- `crates/ac-db/src/tests.rs`
- `crates/ac-migrations/src/lib.rs`
- `migrations/0015_desktop_optimization.sql`

## Persistence

Migration `migrations/0015_desktop_optimization.sql` adds desktop sessions/projects/preferences/UI/approval records plus token usage, resource telemetry and optimization reports. `crates/ac-db/src/desktop_optimization.rs` exposes save/read APIs and reopen tests cover recovery after restart.

## Tests

- `crates/ac-agent` covers desktop open project, goal composition, mission projection, activity, changes, approvals, close/reopen and notifications.
- `crates/ac-daemon` covers window close preserving daemon activity and desktop reconnect.
- `crates/ac-runtime` covers telemetry, token accounting, resource governor, local model/LSP/index policies and optimization reports.
- `crates/ac-context` covers context trimming, relevance preservation and redundant context removal.
- `crates/ac-provider` covers cost-aware model selection.
- `crates/ac-db` covers durable desktop and optimization reopen behavior.

## Limitations

This batch implements the production-facing typed desktop projection and lifecycle state, not a pixel-perfect final Tauri/React renderer. Optional platform notification/tray/sound integrations are represented by typed records and can be bound by the shell without changing authority boundaries.

## Acceptance Status

ACCEPTED. Applicable gates `P22-G1..P22-G18` are represented by production-facing APIs and final validation commands recorded in the phase completion package.
