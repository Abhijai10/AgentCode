# WP-P05-WP05 — Filesystem

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-tool`

## Acceptance Evidence

Brokered workspace tools provide list, read, create, write, delete, and recursive exact search. Writes create parent directories only after the path guard validates their canonical parent; directory deletion is denied.

## Validation

Workspace command and path-guard tests PASS.
