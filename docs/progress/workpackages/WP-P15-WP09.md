# WP-P15-WP09 — Responsive profiles

Status: ACCEPTED

Objective: Make representative mobile, tablet, and desktop viewports available to browser verification.

Implementation summary: `ViewportProfile` and `BrowserRuntime::default_viewports` provide stable profiles used by screenshot capture and tests.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: None.

Tests: `phase15_dev_server_responsive_profiles_and_crash_recovery_work`.

Acceptance status: ACCEPTED; P15-G10 covered.
