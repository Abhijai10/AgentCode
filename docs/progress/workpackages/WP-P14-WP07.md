# WP-P14-WP07 — requirement evidence

Status: ACCEPTED

Objective: Link requirements directly to evidence rather than inferring verification from task pass state.

Implementation summary: `RequirementEvidenceLink` and `link_requirement_evidence` record explicit requirement-to-evidence verification with freshness keys.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `requirement_verifications`.

Tests: `phase14_evidence_requirement_freshness_and_integration_work`, `phase14_verification_state_survives_reopen`.

Acceptance status: ACCEPTED; P14-G4 covered.
