# WP-P13-WP04 — unified diff adapter

Status: ACCEPTED

Objective: Apply unified diff hunks only when context and removed text match exactly.

Implementation summary: `EditStrategy::UnifiedDiff` parses simple unified hunks, validates context, rejects missing/ambiguous hunk targets, and records rejected hunks as conflicts.

Affected files: `crates/ac-changeset/src/lib.rs`.

Database changes: unified-diff operations persist in `edit_operations`.

Tests: `phase13_unified_diff_rejects_bad_context`.

Acceptance status: ACCEPTED; P13-G2 covered.
