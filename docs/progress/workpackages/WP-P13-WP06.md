# WP-P13-WP06 — structured symbol edit

Status: ACCEPTED

Objective: Target symbol identity where possible instead of relying only on line positions.

Implementation summary: `EditStrategy::StructuredSymbol` validates a symbol fingerprint and replaces the single matching symbol line, rejecting ambiguous or stale symbol state.

Affected files: `crates/ac-changeset/src/lib.rs`.

Database changes: `symbol_fingerprint` persists on `edit_operations`.

Tests: `phase13_structured_ast_lsp_format_and_metrics_are_available`.

Acceptance status: ACCEPTED.
