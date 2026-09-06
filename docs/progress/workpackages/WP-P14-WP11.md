# WP-P14-WP11 — integration verification

Status: ACCEPTED

Objective: Verify integrated state with broad evidence after merge/integration.

Implementation summary: `verify_integration` requires broad test/build/check evidence and rejects non-passing manifests.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: integration reports append evidence and can be persisted as verification runs.

Tests: `phase14_evidence_requirement_freshness_and_integration_work`.

Acceptance status: ACCEPTED; P14-G11 covered.
