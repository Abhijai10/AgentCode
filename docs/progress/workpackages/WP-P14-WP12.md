# WP-P14-WP12 — freshness invalidation

Status: ACCEPTED

Objective: Invalidate stale verification evidence when dependent source paths change.

Implementation summary: `invalidate_stale_evidence` compares changed paths to manifest freshness dependencies and returns explicit stale/not-stale decisions.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: freshness dependencies persist on `verification_runs`.

Tests: `phase14_evidence_requirement_freshness_and_integration_work`.

Acceptance status: ACCEPTED; P14-G10 covered.
