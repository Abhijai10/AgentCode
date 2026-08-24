# WP-P29-WP07 — Rollback and incident-response plan

Status: ACCEPTED

Objective: Rollback and incident-response plan.

Implementation summary: Upgrade path and deployment notes record rollback/recovery procedures.

Affected files:
- crates/ac-daemon/src/release.rs
- crates/ac-daemon/src/lib.rs
- crates/ac-db/src/security_release.rs
- crates/ac-db/src/models.rs
- migrations/0018_release_candidate_v1.sql
- tests/integration/final_release_flow.rs
- release/

Database changes: schema v18 via migrations/0018_release_candidate_v1.sql.

Tests: cargo test --workspace (includes ac-daemon release tests, ac-db reopen tests and integration final_release_flow).

Acceptance status: ACCEPTED at starting commit 1b90f158b4bd6d921d64256f22c69509112d022f; final accepted commit recorded in phase completion package.
