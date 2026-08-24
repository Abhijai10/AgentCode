# WP-P14-WP08 — Verifier prompt/context

Status: ACCEPTED

Objective: Give the independent Verifier anti-anchored, read-only context.

Implementation summary: `build_verifier_context` includes actual requirement, diff, tests, and evidence refs while excluding Worker narrative and setting `can_edit = false`.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: verifier reports persist through verification/final-audit evidence records.

Tests: `phase14_independent_verifier_rejects_false_done_and_cannot_edit`.

Acceptance status: ACCEPTED; P14-G5 and P14-G6 covered.
