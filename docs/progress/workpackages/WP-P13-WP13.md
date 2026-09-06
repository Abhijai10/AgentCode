# WP-P13-WP13 — strategy metrics

Status: ACCEPTED

Objective: Measure edit quality by strategy, task/language, first-apply success, syntax failures, retries, unrelated diff, and verification rejection.

Implementation summary: `EditQualityMetrics` is produced with each transaction and persisted through `edit_strategy_metrics`. The model records degraded strategy paths without fabricating successful external-tool results.

Affected files: `crates/ac-changeset/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0008_advanced_edit_engine.sql`.

Database changes: `edit_strategy_metrics` table and query API.

Tests: `phase13_structured_ast_lsp_format_and_metrics_are_available`, `phase13_edit_transactions_survive_reopen_and_expose_recovery_rows`.

Acceptance status: ACCEPTED.
