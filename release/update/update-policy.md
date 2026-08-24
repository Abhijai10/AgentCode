# Update And Rollback Policy

Every update decision records:

- current version
- available version
- artifact integrity verification
- decision
- rollback reference
- recovery action

Failed integrity verification blocks the update and preserves the current version.
Failed applied update restores the previous artifact and keeps user data directories.

V1 public update automation requires signed/provenanced artifacts. Manual update is the
honest fallback until signing/notarization credentials and distribution channel are
available.
