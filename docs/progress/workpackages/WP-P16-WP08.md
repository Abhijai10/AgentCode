# WP-P16-WP08 — Hook Registry And Manifest

Status: ACCEPTED

Objective: Register lifecycle hooks with typed event, priority, timeout, idempotency, failure policy, and capability requirements.

Implementation summary: `HookManifest`, `HookEvent`, `HookFailurePolicy`, and `HookRegistry::register` provide validated hook metadata.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0011_extensions_skills_hooks_mcp.sql`.

Database changes: `hook_manifests`.

Tests: `phase16_hooks_execute_timeout_block_completion_and_stop_recursion`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G6 covered.
