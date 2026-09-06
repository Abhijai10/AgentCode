# WP-P15-WP04 — Page actions

Status: ACCEPTED

Objective: Support production-facing navigation, click, type, select, scroll, and wait actions with evidence.

Implementation summary: `BrowserAction` and `BrowserRuntime::act` apply page mutations through registered sessions, validate selectors, update current URL, and emit browser action evidence for the actual operation result.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: None.

Tests: `phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work`, `phase15_browser_capability_denial_and_selector_failure_are_explicit`.

Acceptance status: ACCEPTED; P15-G3 and the behavioral end-user flow gate covered.
