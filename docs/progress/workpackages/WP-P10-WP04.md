# WP-P10-WP04 — Freshness Engine

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Mark affected facts stale or invalid when source evidence changes.
- **Implementation details:** Added `FreshnessDependency`, in-memory `apply_source_change`, and DB `mark_memory_for_source_change` reverse-map update using `memory_fact_evidence`.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** `freshness_invalidates_only_dependent_facts`; DB source-change reopen test.
- **Limitations:** Selective revalidation marks state; reindex/reproof scheduling belongs to later phases.

## Acceptance

ACCEPTED. P10-G4/P10-G5/P10-G6 PASS: freshness is recorded, relevant changes stale/invalid facts, unrelated facts remain fresh.
