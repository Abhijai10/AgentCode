# WP-P10-WP05 — Conflict Model

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Preserve contradictory facts instead of silently overwriting them.
- **Implementation details:** Added conflict detection and `conflict_set` assignment; conflicting facts become `CONFLICTED`.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** `conflicting_facts_are_stored_not_overwritten`.
- **Limitations:** Conflict detection is conservative and typed; sophisticated semantic contradiction detection is future work.

## Acceptance

ACCEPTED. P10-G7 PASS: conflicts become explicit `CONFLICTED` facts.
