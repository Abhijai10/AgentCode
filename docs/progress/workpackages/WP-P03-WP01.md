# WP-P03-WP01 — Kernel Database Schema

- **Phase:** P03
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 593e649935a1391df26c3d74cc17bb3f48f2b615
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-db`, `migrations/0001_kernel_schema.sql`
- **Architecture refs:** ADR-0004; P02 completion package; Phase 1 runtime/context/evidence packets
- **Acceptance gates:** P3 durable Kernel/agent state gates PASS for Phase 3 autonomous coding scope

## Implemented In This Batch

- Added `ac-db` with `rusqlite` bundled SQLite driver selected by ADR-0004.
- Added centralized connection configuration for foreign keys and busy timeout.
- Added transactional migration application for `migrations/0001_kernel_schema.sql`.
- Added persistence APIs for Kernel missions, Kernel events, and evidence records.
- Added file-backed reopen test proving mission and event history survive process-style closure.
- Added dependency admission/legal notice updates for `rusqlite`/`libsqlite3-sys`.
- Added AgentSession and checkpoint tables for durable restart evidence.
- Added interrupted-session recovery query for daemon startup reconciliation.
- Added future-schema guard (`DB-FUTURE_VERSION`) so newer databases are rejected instead of silently migrated backward.
- Added idempotent Kernel event append semantics for replay/recovery.
- Added `ac-daemon` lifecycle foundation with singleton lock, local IPC command contract, session creation, checkpoint persistence, restart recovery inspection, and health reporting.
- Added persisted WorktreeRecord fields including repository/branch/base/current/status/ownership.
- Added persisted ChangeSet operation, metadata, rollback, and state payloads.
- Added load/recovery APIs for AgentSession, WorktreeRecord, ChangeSet, and checkpoints.
- Added restart recovery test that reopens SQLite and proves session, worktree ownership, checkpoint, and ChangeSet state survive process-style closure.

## Validation

- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.
- `cargo fmt` PASS.

## Known Limitations

- Previous-version fixture migration.
- Interrupted migration recovery/failure test.
- OS/socket IPC transport beyond in-process `LocalIpc`.
- Robust daemon singleton metadata/PID validation beyond create-new lock file.
- Restart reconciliation now loads durable lifecycle records, but full daemon-orchestrated execution resume remains a later hardening item.
- IPC reconnect gates.

## Acceptance Decision

ACCEPTED for Phase 3: SQLite now preserves and reloads the autonomous lifecycle records required by AgentSession/worktree/ChangeSet/checkpoint recovery, with targeted restart evidence and workspace validation passing.
