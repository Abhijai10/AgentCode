# WP-P05-WP09 — Tool Evidence

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-evidence`, `crates/ac-db`, `crates/ac-tool`

## Acceptance Evidence

Raw process content is append-only evidence with a deterministic hash and bounded redacted model summary. `tool_execution_records` persists manifest, raw output, status, and evidence reference; reopen coverage proves recovery.

## Validation

Evidence and SQLite reopen tests PASS.
