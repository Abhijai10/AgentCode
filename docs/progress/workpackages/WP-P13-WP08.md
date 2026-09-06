# WP-P13-WP08 — rollback

Status: ACCEPTED

Objective: Roll back partially applied edits using recorded pre-change content and hashes.

Implementation summary: `EditEngine::apply` rolls back already-applied files if a later write fails or if concurrent mutation is detected. `EditEngine::rollback` verifies restored hashes before reporting success.

Affected files: `crates/ac-changeset/src/lib.rs`.

Database changes: rollback states persist in `edit_journal_entries`.

Tests: `phase13_failure_on_second_file_rolls_back_first_file`.

Acceptance status: ACCEPTED; P13-G5 and corruption gate covered.
