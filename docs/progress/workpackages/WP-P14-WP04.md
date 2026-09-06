# WP-P14-WP04 — test registry

Status: ACCEPTED

Objective: Discover test identities and make test selection explicit.

Implementation summary: `discover_tests` walks the repository while excluding generated/runtime directories and records unit test identities for Rust and TypeScript-style test files.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: selected test identity is represented in verification manifests and persisted run rows.

Tests: `phase14_profile_commands_gates_and_targeted_tests_work`.

Acceptance status: ACCEPTED; P14-G2 and P14-G3 covered.
