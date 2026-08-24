# WP-P10-WP02 — Evidence References

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Ensure important memory facts and decisions point back to evidence.
- **Implementation details:** Facts require source evidence; dependencies record evidence refs plus optional file/symbol/hash; DB stores `memory_fact_evidence`.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** evidence-required validation; DB reopen test loads memory evidence rows.
- **Limitations:** Evidence refs are IDs/strings; raw evidence remains owned by Evidence Store.

## Acceptance

ACCEPTED. P10-G2 PASS: important facts and decisions require provenance references.
