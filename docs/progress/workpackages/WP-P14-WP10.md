# WP-P14-WP10 — test tampering detection

Status: ACCEPTED

Objective: Detect skipped tests and weakened assertions.

Implementation summary: `detect_test_tampering` compares base/changed test text and flags skips, removed assertions, `assert!(true)`, and trivial equality assertions.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: tampering findings can be stored as verification findings.

Tests: `phase14_test_tampering_detects_weakened_assertions_and_skips`.

Acceptance status: ACCEPTED; P14-G8 and P14-G9 covered.
