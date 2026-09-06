# WP-P13-WP02 — precondition hashes

Status: ACCEPTED

Objective: Require current content hash, base revision, and optional symbol fingerprint before an edit can mutate a file.

Implementation summary: `EditPrecondition` validates path, content hash, base revision, and symbol fingerprint. Stale content or base revision returns conflict instead of overwriting.

Affected files: `crates/ac-changeset/src/lib.rs`, `crates/ac-agent/src/lib.rs`.

Database changes: precondition result hashes persist in `edit_operations` and `edit_journal_entries`.

Tests: `phase13_stale_hash_and_concurrent_mutation_block_overwrite`, agent isolated-worktree hash assertion.

Acceptance status: ACCEPTED; P13-G3 and P13-G10 covered.
