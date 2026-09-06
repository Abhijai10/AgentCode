# WP-P15-WP01 — Playwright adapter

Status: ACCEPTED

Objective: Provide a browser automation adapter behind the existing verification authority boundary.

Implementation summary: `ac-verification` now exposes `BrowserRuntime` with explicit adapter mode selection. The production-facing API defaults to a deterministic harness when Playwright is unavailable, preserving the same launch/session/action/evidence contracts without requiring an optional dependency.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0010_browser_runtime.sql`.

Database changes: `browser_processes`, `browser_sessions`.

Tests: `phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work`, `phase15_browser_runtime_state_survives_reopen`.

Acceptance status: ACCEPTED; P15-G1 covered.
