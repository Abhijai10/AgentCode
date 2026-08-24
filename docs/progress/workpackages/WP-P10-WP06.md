# WP-P10-WP06 — Decision Store

- **Phase:** P10
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** d5a338e
- **Accepted commit:** 7d75621
- **Objective:** Persist user/architecture decisions independently of generated summaries.
- **Implementation details:** Added `MemoryDecision`, `record_decision`, and durable `memory_decisions` with authority refs and supersession.
- **Files changed:** `crates/ac-context/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0005_persistent_memory.sql`
- **Migrations:** `0005_persistent_memory`
- **Tests:** context handoff test includes decisions; DB reopen test counts decision history.
- **Limitations:** No file-backed `DECISIONS.md` writer yet; durable DB history is the authority.

## Acceptance

ACCEPTED. P10-G10 PASS: decision history persists independently.
