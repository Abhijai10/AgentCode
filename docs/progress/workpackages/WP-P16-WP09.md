# WP-P16-WP09 — Hook Dispatch, Ordering And Timeout

Status: ACCEPTED

Objective: Execute lifecycle hooks deterministically with priority ordering and timeout handling.

Implementation summary: `HookRegistry::dispatch` selects hooks by event, orders by priority, applies timeout/failure policy, and emits typed invocation records.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `hook_invocations`.

Tests: `phase16_hooks_execute_timeout_block_completion_and_stop_recursion`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G6 and P16-G7 covered.
