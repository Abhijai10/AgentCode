# AgentCode V1 Release Manifest

- Version: 1.0.0
- Source commit: final release commit produced by Phase 29 completion.
- Database schema: v18.
- Platforms: macOS Apple Silicon release candidate path.
- Artifacts: app bundle artifact records persisted through `release_artifacts`.
- Checksums: stored on artifact rows and final release manifest rows.
- Migrations: `0001_kernel_schema.sql` through `0018_release_candidate_v1.sql`.
- Evidence bundle: audit, security, validation, artifact and migration reports.

Known limitation: public signing/notarization credentials are prerequisite-bound.
