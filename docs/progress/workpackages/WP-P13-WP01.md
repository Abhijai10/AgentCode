# WP-P13-WP01 — ChangeSet model

Status: ACCEPTED

Objective: Implement a single typed ChangeSet transaction model for all advanced edit strategies.

Implementation summary: `ac-changeset` now exposes `ChangeSetTransaction`, prepared edits, lifecycle states `PREPARED/APPLYING/APPLIED/VALIDATING/ACCEPTED/ROLLING_BACK/ROLLED_BACK/CONFLICT/UNKNOWN_EFFECT`, and a shared `EditEngine::prepare/apply/rollback/reconcile` path.

Affected files: `crates/ac-changeset/src/lib.rs`, `crates/ac-agent/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0008_advanced_edit_engine.sql`.

Database changes: `edit_transactions`, `edit_operations`, `edit_journal_entries`, `edit_strategy_metrics`.

Tests: `phase13_search_replace_multifile_transaction_applies`, agent isolated-worktree ChangeSet assertion, `phase13_edit_transactions_survive_reopen_and_expose_recovery_rows`.

Acceptance status: ACCEPTED; P13-G4 covered.
