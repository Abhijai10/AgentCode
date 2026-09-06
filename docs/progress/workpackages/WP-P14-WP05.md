# WP-P14-WP05 — targeted selection

Status: ACCEPTED

Objective: Select likely relevant tests and broaden when confidence is low.

Implementation summary: `select_tests` chooses related tests by path proximity, records exclusions/reasons, and broadens to the full registry when confidence falls below the threshold.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: selection evidence persists through verification manifests.

Tests: `phase14_profile_commands_gates_and_targeted_tests_work`.

Acceptance status: ACCEPTED; P14-G2 covered.
