# WP-P13-WP10 — format integration

Status: ACCEPTED

Objective: Detect a project-native formatter for affected scope.

Implementation summary: `FormatPlan` records affected paths and chooses `cargo fmt --all` for Rust-only edits, no-op for text/markdown, and a generic project-native formatter marker for mixed scopes.

Affected files: `crates/ac-changeset/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: formatter and degraded reason persist on `edit_transactions`.

Tests: `phase13_structured_ast_lsp_format_and_metrics_are_available`.

Acceptance status: ACCEPTED; P13-G7 covered by detection plus final `cargo fmt --all`.
