# WP-P14-WP06 — evidence store

Status: ACCEPTED

Objective: Persist commit-aware verification evidence manifests.

Implementation summary: `record_evidence_manifest` ties proof to commit, worktree, environment, tool version, command, raw artifact, normalized result, requirements, and freshness dependencies through the existing evidence pipeline.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0009_verification_evidence_engine.sql`.

Database changes: `verification_runs`.

Tests: `phase14_evidence_requirement_freshness_and_integration_work`, `phase14_verification_state_survives_reopen`.

Acceptance status: ACCEPTED; P14-G3 and P14-G4 covered.
