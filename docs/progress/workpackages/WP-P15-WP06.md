# WP-P15-WP06 — Console and network diagnostics

Status: ACCEPTED

Objective: Capture page console failures and failed network activity as evidence-backed diagnostics.

Implementation summary: `BrowserRuntime::diagnostics` normalizes console error markers, network failure markers, and non-200 navigation status into structured diagnostics and evidence records.

Affected files: `crates/ac-verification/src/lib.rs`.

Database changes: None.

Tests: `phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work`.

Acceptance status: ACCEPTED; P15-G6 and P15-G7 covered.
