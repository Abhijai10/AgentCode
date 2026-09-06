# Phase 5 Completion — Native Tool Runtime Foundation

## Status

COMPLETE. P05-WP01 through P05-WP11 are ACCEPTED at the completion commit.

## Implemented Capabilities

- Typed tool registry and Broker-only execution path used by the autonomous agent.
- Canonical workspace path guard with traversal and symlink escape rejection.
- Brokered filesystem list/read/create/write/delete/search operations.
- Argv-first sandboxed process execution with sanitized environment, timeout, cancellation, stdout/stderr/exit capture, and background lifecycle control.
- Capability, role, risk, approval, network, and secret-reference enforcement.
- Append-only raw evidence with redacted bounded summaries and SQLite execution-record recovery.
- Existing Agent planning, ChangeSet preparation, verification, repair, and benchmark continue to use the Broker path.

## Gates

P5-G1..P5-G12 PASS through deterministic crate and workspace tests: read/search, workspace writes, traversal/symlink denial, timeout, background control, structured results, raw evidence persistence, routine/high-risk policy, role restrictions, and secret redaction.

## Evidence

- `crates/ac-tool/src/lib.rs`
- `crates/ac-sandbox/src/lib.rs`
- `crates/ac-security/src/lib.rs`
- `crates/ac-evidence/src/lib.rs`
- `crates/ac-db/src/lib.rs`
- `migrations/0001_kernel_schema.sql`
- `docs/progress/workpackages/WP-P05-WP01.md` through `WP-P05-WP11.md`

## Known Limitations

- Process-tree termination is limited to the directly owned child on this no-extra-dependency V1 runtime; process-group enforcement is deferred to a platform-specific hardening phase.
- Exact search is first-party literal search; optional ripgrep acceleration and regex/glob query controls remain deferred.
- Secret storage is in-memory environment-reference plumbing; OS keychain backing is deferred.
