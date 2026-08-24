# WP-P15-WP10 — Visual model

Status: ACCEPTED

Objective: Connect browser screenshots to visual QA without allowing visual review to bypass evidence.

Implementation summary: `BrowserRuntime::visual_qa` consumes screenshot evidence, records findings with an adapter name, and persists visual QA reports through `ac-db`.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0010_browser_runtime.sql`.

Database changes: `browser_visual_qa`.

Tests: `phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work`, `phase15_browser_runtime_state_survives_reopen`.

Acceptance status: ACCEPTED; P15-G11 covered.
