# WP-P15-WP08 — Dev-server integration

Status: ACCEPTED

Objective: Manage development-server readiness as an explicit browser verification dependency.

Implementation summary: `BrowserRuntime::manage_dev_server` validates command and port configuration, records alive/readiness/route-loadability states, and persists dev-server records through `ac-db`.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0010_browser_runtime.sql`.

Database changes: `browser_dev_servers`.

Tests: `phase15_dev_server_responsive_profiles_and_crash_recovery_work`, `phase15_browser_runtime_state_survives_reopen`.

Acceptance status: ACCEPTED; P15-G8 covered.
