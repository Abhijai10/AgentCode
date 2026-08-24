# WP-P15-WP03 — Session registry

Status: ACCEPTED

Objective: Tie browser sessions to task-owned processes and isolate browser profile/storage state.

Implementation summary: `BrowserRuntime::create_session` rejects foreign or crashed processes, assigns isolated profile directories, stores sensitive storage-state references, and persists sessions through `ac-db`.

Affected files: `crates/ac-verification/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0010_browser_runtime.sql`.

Database changes: `browser_sessions`.

Tests: `phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work`, `phase15_browser_runtime_state_survives_reopen`.

Acceptance status: ACCEPTED; P15-G2 covered.
