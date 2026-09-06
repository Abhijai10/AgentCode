# WP-P15-WP11 — Crash recovery

Status: ACCEPTED

Objective: Recover crashed browser sessions while preserving task ownership and stale evidence references.

Implementation summary: `BrowserRuntime::mark_crashed` and `recover_crashed_session` relaunch task-owned browser state, carry forward the last URL, and record stale evidence references on the recovered session.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0010_browser_runtime.sql`.

Database changes: `browser_processes`, `browser_sessions`.

Tests: `phase15_dev_server_responsive_profiles_and_crash_recovery_work`, `phase15_browser_runtime_state_survives_reopen`.

Acceptance status: ACCEPTED; P15-G9 covered.
