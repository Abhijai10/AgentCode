# Phase 29 Completion — V1 Release

Status: COMPLETE

Starting commit: 1b90f158b4bd6d921d64256f22c69509112d022f

Completed work packages: P29-WP01, P29-WP02, P29-WP03, P29-WP04, P29-WP05, P29-WP06, P29-WP07, P29-WP08.

Implementation evidence:
- crates/ac-daemon/src/release.rs
- crates/ac-db/src/security_release.rs
- migrations/0018_release_candidate_v1.sql
- tests/integration/final_release_flow.rs
- release/v1-manifest.md
- release/reports/audit-report.md
- release/reports/validation-report.md

Gate status: REL-G01, REL-G02, REL-G03, REL-G04, REL-G05, REL-G06, REL-G07, REL-G08, REL-G09, REL-G10, REL-G11, REL-G12, REL-G13, REL-G14, REL-G15, REL-G16, REL-G17, REL-G18, REL-G19, REL-G20 PASS through the recorded implementation, persistence, tests and evidence package.

Migration impact: schema v18 adds release candidates, validation runs, approval decisions, final manifests and release evidence bundles.

Limitations: public signing/notarization credentials remain prerequisite-bound; unavailable optional external scanners must remain explicit degraded/block states.

Validation: final commands listed in completion JSON and final response.

Final release is represented by manifest hash, immutable decision, evidence bundle and Released state transition.
