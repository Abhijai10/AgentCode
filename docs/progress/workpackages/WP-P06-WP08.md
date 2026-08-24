# WP-P06-WP08 — Failure Retry

- **Phase:** P06
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Owner modules:** `crates/ac-runtime`, `crates/ac-db`

TaskGraph tracks dependency order, worker ownership, attempt outcome, retry count, failure class, cancellation, and exhaustion. SQLite reopen coverage proves recovery.
