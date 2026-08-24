# WP-P13-WP03 — search-replace adapter

Status: ACCEPTED

Objective: Add a strict search/replace adapter with match-count and file-state validation.

Implementation summary: `EditStrategy::SearchReplace` requires non-empty search text, exact expected match count, and validated preconditions before producing a prepared edit.

Affected files: `crates/ac-changeset/src/lib.rs`.

Database changes: strategy name and hashes persist through Phase 13 edit tables.

Tests: `phase13_search_replace_multifile_transaction_applies`, `phase13_stale_hash_and_concurrent_mutation_block_overwrite`.

Acceptance status: ACCEPTED; P13-G1 covered.
