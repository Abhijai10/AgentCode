# WP-P10-WP03 — Confidence Model

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Record confidence separately from freshness and authority.
- **Implementation details:** Facts validate confidence in `0..=100`; generated `CONTEXT.md` labels confidence alongside freshness; DB persists confidence.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** service and DB tests assert confidence-bearing facts survive.
- **Limitations:** Promotion policy is manual/API-level for now; automated promotion can build on this field.

## Acceptance

ACCEPTED. P10-G3 PASS: confidence is recorded and rendered distinctly from freshness.
