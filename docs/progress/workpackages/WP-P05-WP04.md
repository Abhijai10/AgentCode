# WP-P05-WP04 — Workspace Path Guard

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-tool`, `crates/ac-sandbox`

## Acceptance Evidence

Filesystem paths canonicalize the workspace root and target, reject absolute/traversal paths, and reject symlink resolution outside the workspace for reads, writes, creates, deletes, lists, and search traversal.

## Validation

`workspace_guard_blocks_traversal_and_symlink_escapes` PASS.
