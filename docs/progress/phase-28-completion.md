# Phase 28 Completion — V1 Release Candidate

Status: COMPLETE

Starting commit: 1b90f158b4bd6d921d64256f22c69509112d022f

Completed work packages: P28-WP01, P28-WP02, P28-WP03, P28-WP04, P28-WP05, P28-WP06, P28-WP07, P28-WP08, P28-WP09, P28-WP10, P28-WP11, P28-WP12, P28-WP13, P28-WP14.

Implementation evidence:
- crates/ac-daemon/src/release.rs
- crates/ac-db/src/security_release.rs
- migrations/0018_release_candidate_v1.sql
- tests/integration/final_release_flow.rs
- release/v1-manifest.md
- release/reports/audit-report.md
- release/reports/validation-report.md

Gate status: RC-01, RC-02, RC-03, RC-04, RC-05, RC-06, RC-07, RC-08, RC-09, RC-10, RC-11, RC-12, RC-13, RC-14, RC-15, RC-16, RC-17, RC-18, RC-19, RC-20 PASS through the recorded implementation, persistence, tests and evidence package.

Migration impact: schema v18 adds release candidates, validation runs, approval decisions, final manifests and release evidence bundles.

Limitations: public signing/notarization credentials remain prerequisite-bound; unavailable optional external scanners must remain explicit degraded/block states.

Validation: final commands listed in completion JSON and final response.

RC acceptance is represented by accepted release candidate records and production validation runs.
