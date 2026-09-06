# WP-P13-WP05 — whole-file adapter

Status: ACCEPTED

Objective: Keep whole-file replacement available as a validated adapter rather than an ad hoc write.

Implementation summary: `EditStrategy::WholeFile` produces the same prepared edit model and requires hash/base-revision preconditions. The autonomous agent now prepares existing worktree file writes through the Phase 13 engine before Kernel approval.

Affected files: `crates/ac-changeset/src/lib.rs`, `crates/ac-agent/src/lib.rs`.

Database changes: whole-file strategy metrics and operations persist through migration 0008.

Tests: `phase13_failure_on_second_file_rolls_back_first_file`, `autonomous_demo_fixes_fixture_inside_isolated_workspace`.

Acceptance status: ACCEPTED; P13-G4 and quality gate covered.
