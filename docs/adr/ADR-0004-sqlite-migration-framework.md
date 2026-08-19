# ADR-0004 — SQLite Driver and Migration Framework

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

Which Rust SQLite driver and migration mechanism implements the V1 control-plane store
and satisfies Doc 11 H17/H18 (concurrency/singleton, migration rules)?

## Constraints

- Doc 03 §0.31: logical SQLite control-plane schema; WAL mode or equivalent; single
  authoritative Kernel writer; short transactions.
- Doc 11 H17: centralized SQLite configuration — WAL, busy timeout, foreign keys,
  transaction isolation, single daemon ownership lock, migration lock/backup, read
  concurrency.
- Doc 11 H18: migration ID/version, forward migration, fixture from previous version,
  interruption behavior, backup/recovery, fresh-database test, unknown-future-version
  behavior.
- Doc 09 HC-P02: `migrations/` canonical directory; migration validation CI job.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| sqlx (compile-time checked) | Popular async/compile-time SQL | Async runtime not needed in daemon skeleton; heavier proc-macro surface; offline mode complexity for migrations |
| diesel | ORM | Schema-first ORM conflicts with direct control-plane SQL control and evidence auditability |
| rusqlite (bundled) + custom migration runner | Direct SQLite control, no async, vendored SQLite | Chosen |
| refinery | Dedicated migration crate | Extra dependency with its own CLI/runtime semantics; custom runner gives exact H18 interruption/future-version control |
| SQLite in JS (better-sqlite3) | TS-side store | Control plane must be Kernel/Rust-owned (Doc 03); rejected |

## Evidence

- Doc 03 §0.31, Doc 11 H17/H18; extraction P01-WP04 (codex-rs uses rusqlite-style
  direct SQLite with embedded migrations), P01-WP08 (letta-core migrations via custom
  runner) — direct-driver + small migration runner is the proven donor pattern.

## Decision

- Driver: `rusqlite` with `bundled` feature (SQLite compiled in; no system dependency,
  deterministic across machines).
- Centralized connection config in `crates/ac-db`: `PRAGMA foreign_keys=ON`,
  `PRAGMA journal_mode=WAL`, `busy_timeout` (default 5000ms), single-writer ownership
  enforced by the daemon singleton (Phase 3), migration applied inside
  `BEGIN IMMEDIATE` transactions.
- Migration mechanism: versioned SQL files in `migrations/` (`NNNN_description.sql`),
  embedded via `include_str!` at compile time, applied in order by `ac-db`'s runner
  using `PRAGMA user_version` as the version marker.
  - unknown future version (DB newer than runner) → explicit `Error::FutureVersion`
  - interrupted migration → transaction rollback leaves previous version; re-run
    succeeds
  - duplicate migration number → hard error at validation time
  - fresh DB → full chain to current
  - migration backup: DB file copy before upgrade when upgrading a non-empty DB
    (Phase 3 wiring; harness exposes the hook now)
- Phase 2 ships only the harness + substrate migrations (v1 seed, v2 seed proving
  forward migration); the Kernel schema arrives in Phase 3.

## Consequences

- Positive: exact control over H17/H18 semantics; no async; vendored SQLite removes
  platform variance.
- Negative: rusqlite is not async; acceptable for a single-writer daemon.
- Affected: Doc 03 §0.31, Doc 09 H11, Doc 11 H17/H18; Phase 3 Kernel schema work.

## Verification

- P2-G5 migration framework works: fresh DB, v1→v2, interrupted migration,
  unknown-future version, temp-db isolation (tests in `crates/ac-db`).