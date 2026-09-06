# WP-P13-WP09 — crash recovery

Status: ACCEPTED

Objective: Reconcile interrupted edit transactions after restart.

Implementation summary: `EditEngine::reconcile` compares actual file hashes with journal before/after hashes and returns `Noop`, `Finish`, `Rollback`, or `Blocked`. `ac-db` exposes pending edit transactions for daemon recovery.

Affected files: `crates/ac-changeset/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `pending_edit_transactions()` queries `Applying`, `RollingBack`, and `UnknownEffect` transactions.

Tests: `phase13_crash_reconciliation_detects_partial_state`, `phase13_edit_transactions_survive_reopen_and_expose_recovery_rows`.

Acceptance status: ACCEPTED; P13-G6 covered.
