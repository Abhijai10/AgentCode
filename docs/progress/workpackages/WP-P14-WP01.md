# WP-P14-WP01 — Verification profile

Status: ACCEPTED

Objective: Derive required verification layers from task risk and project capabilities without letting Worker text weaken them.

Implementation summary: `ac-verification` now provides `VerificationProfile`, `VerificationRisk`, `VerificationLayer`, and `derive_profile`, selecting format/lint/typecheck/unit/build/integration/browser/security layers from capabilities and risk.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0009_verification_evidence_engine.sql`.

Database changes: `verification_profiles`.

Tests: `phase14_profile_commands_gates_and_targeted_tests_work`, `phase14_verification_state_survives_reopen`.

Acceptance status: ACCEPTED; P14-G1 covered.
