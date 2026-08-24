# WP-P14-WP09 — adversarial review

Status: ACCEPTED

Objective: Detect false-done risks such as placeholders, mocks, missing wiring, and weak assertions.

Implementation summary: `run_independent_verifier` applies an adversarial checklist to implementation/test summaries and returns blocking findings when false-done patterns are present.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: findings can be persisted through `verification_findings`/audit rows.

Tests: `phase14_independent_verifier_rejects_false_done_and_cannot_edit`.

Acceptance status: ACCEPTED; P14-G7, P14-G8, and P14-G9 covered.
