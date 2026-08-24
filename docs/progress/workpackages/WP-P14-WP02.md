# WP-P14-WP02 — command detection

Status: ACCEPTED

Objective: Detect project-native verification commands from repository files.

Implementation summary: `detect_project_capabilities` and `detect_commands` identify Cargo, Makefile, npm, browser, and security capabilities and map them to deterministic command specs.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: commands persist through `verification_runs.command`.

Tests: `phase14_profile_commands_gates_and_targeted_tests_work`.

Acceptance status: ACCEPTED; P14-G1 covered.
