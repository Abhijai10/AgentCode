# WP-P15-WP07 — Screenshot evidence

Status: ACCEPTED

Objective: Persist screenshots as evidence-linked browser observations with viewport and commit identity.

Implementation summary: `BrowserRuntime::capture_screenshot` produces screenshot evidence records with session, task, commit, viewport, URL, artifact URI, sensitivity, and durable storage through `ac-db`.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0010_browser_runtime.sql`.

Database changes: `browser_screenshots`.

Tests: `phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work`, `phase15_browser_runtime_state_survives_reopen`.

Acceptance status: ACCEPTED; P15-G5 covered.
