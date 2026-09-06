# Upgrade Path

Fresh install:

- Create a new control-plane database.
- Apply migrations `0001` through `0018`.
- Verify schema version v18 and first-run setup.

Upgrade:

- Back up the existing control-plane database.
- Apply pending migrations through `0018_release_candidate_v1.sql`.
- Verify release candidate and final release tables can be reopened.

Rollback:

- Keep the prior app bundle and database backup until validation completes.
- If update validation fails, retain current version and restore the previous bundle.

Recovery:

- After interruption, reopen the control-plane database and rerun migration validation.
- Release state is reconstructed from persisted candidates, validations, decisions,
  manifests and evidence bundles.
