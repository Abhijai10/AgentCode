# WP-P15-WP02 — Browser process manager

Status: ACCEPTED

Objective: Launch, track, classify, and recover browser processes under capability control.

Implementation summary: `BrowserRuntime::launch`, process records, state transitions, and crash marking enforce `BrowserAutomation` capability and bind processes to task identity.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0010_browser_runtime.sql`.

Database changes: `browser_processes`.

Tests: `phase15_browser_capability_denial_and_selector_failure_are_explicit`, `phase15_dev_server_responsive_profiles_and_crash_recovery_work`, `phase15_browser_runtime_state_survives_reopen`.

Acceptance status: ACCEPTED; P15-G1 and P15-G9 covered.
