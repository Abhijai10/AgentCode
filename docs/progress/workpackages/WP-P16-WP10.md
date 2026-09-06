# WP-P16-WP10 — Hook Failure, Recursion And Idempotency

Status: ACCEPTED

Objective: Prevent hooks from recurring indefinitely, escaping policy, or silently allowing completion failures.

Implementation summary: Hook dispatch tracks active/completed idempotency keys, converts timeout/failure policy into structured outcomes, blocks workspace escapes, and lets completion hooks reject completion.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: `hook_invocations` stores outcomes.

Tests: `phase16_hooks_execute_timeout_block_completion_and_stop_recursion`.

Acceptance status: ACCEPTED; P16-G8, P16-G9, and P16-G10 covered.
