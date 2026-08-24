# WP-P13-WP07 — transaction journal

Status: ACCEPTED

Objective: Record per-file transaction state before mutation and after each applied file.

Implementation summary: `TransactionJournal` records path, state, before hash, after hash, and timestamp for each prepared edit. The database migration persists journal entries and indexes recovery states.

Affected files: `crates/ac-changeset/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0008_advanced_edit_engine.sql`.

Database changes: `edit_journal_entries` plus recovery index.

Tests: `phase13_edit_transactions_survive_reopen_and_expose_recovery_rows`, `phase13_failure_on_second_file_rolls_back_first_file`.

Acceptance status: ACCEPTED.
