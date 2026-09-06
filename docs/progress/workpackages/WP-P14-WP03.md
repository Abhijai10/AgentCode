# WP-P14-WP03 — mechanical gates

Status: ACCEPTED

Objective: Normalize format/lint/typecheck/compile/build/test outcomes into explicit gate states.

Implementation summary: `GateStatus` and `normalize_gate_result` normalize passed, failed, unavailable, skipped, partial, baseline failure, regression, and stale states while appending raw command evidence.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: normalized gate states persist in `verification_runs.normalized_result`.

Tests: `phase14_profile_commands_gates_and_targeted_tests_work`.

Acceptance status: ACCEPTED; P14-G1 covered.
