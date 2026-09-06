# WP-P13-WP11 — ast-grep transforms

Status: ACCEPTED

Objective: Support safe repetitive structural migrations with an explicit fallback when external ast-grep is unavailable.

Implementation summary: `EditStrategy::AstGrep` provides a deterministic in-process pattern/rewrite fallback that validates non-empty patterns and fails if no source pattern matches. The limitation is explicit: no external `ast-grep` binary is required for this V1 path.

Affected files: `crates/ac-changeset/src/lib.rs`.

Database changes: AST strategy metrics persist through `edit_strategy_metrics`.

Tests: `phase13_structured_ast_lsp_format_and_metrics_are_available`.

Acceptance status: ACCEPTED; P13-G8 covered by fallback transform.
