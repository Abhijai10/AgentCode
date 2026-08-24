# WP-P13-WP12 — LSP workspace edits

Status: ACCEPTED

Objective: Support LSP-style workspace rename edits while preserving transaction preconditions and rollback.

Implementation summary: `EditStrategy::LspRename` performs token-aware identifier rename as the production fallback when a language server is unavailable. It requires symbol fingerprint validation for symbol-targeted calls and routes through the same ChangeSet transaction model.

Affected files: `crates/ac-changeset/src/lib.rs`.

Database changes: LSP rename strategy metrics persist through `edit_strategy_metrics`.

Tests: `phase13_structured_ast_lsp_format_and_metrics_are_available`.

Acceptance status: ACCEPTED; P13-G9 covered by supported fallback fixture.
