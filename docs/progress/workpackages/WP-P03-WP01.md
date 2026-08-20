# WP-P03-WP01 — Kernel Database Schema

- **Phase:** P03
- **Status:** IMPLEMENTING
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 593e649935a1391df26c3d74cc17bb3f48f2b615
- **Accepted commit:** not accepted yet
- **Owner modules:** `crates/ac-db`, `migrations/0001_kernel_schema.sql`
- **Architecture refs:** ADR-0004; P02 completion package; Phase 1 runtime/context/evidence packets
- **Acceptance gates:** P3-G7/P3-G8 partial; full P3-G1..P3-G10 not complete

## Implemented In This Batch

- Added `ac-db` with `rusqlite` bundled SQLite driver selected by ADR-0004.
- Added centralized connection configuration for foreign keys and busy timeout.
- Added transactional migration application for `migrations/0001_kernel_schema.sql`.
- Added persistence APIs for Kernel missions, Kernel events, and evidence records.
- Added file-backed reopen test proving mission and event history survive process-style closure.
- Added dependency admission/legal notice updates for `rusqlite`/`libsqlite3-sys`.

## Validation

- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `jq empty docs/legal/third_party_manifest.json` PASS.

## Remaining Before Acceptance

- Unknown-future-version behavior.
- Previous-version fixture migration.
- Interrupted migration recovery/failure test.
- Daemon singleton/write ownership lock.
- Restart reconciliation and IPC reconnect gates.
