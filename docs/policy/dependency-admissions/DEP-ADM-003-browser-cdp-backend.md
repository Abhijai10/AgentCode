# DEP-ADM-003 — Browser CDP Backend Dependencies

Need:
  Batch 4 replaces the fake browser runtime with a real Chrome DevTools Protocol backend. The backend needs WebSocket transport for CDP, JSON payload handling, PNG screenshot base64 decoding, and isolated temporary browser profiles.

Proposed dependencies:
  tungstenite 0.30.0, base64 0.23.1, tempfile 3.27.0 from crates.io, pinned in Cargo.lock. serde_json 1.0.151 is reused from the existing admitted workspace dependency set.

Existing alternative check:
  The prior ac-verification BrowserRuntime stored caller-provided HTML and simulated DOM/action/screenshot behavior in memory. The Rust standard library does not provide WebSocket framing or temporary directory cleanup semantics sufficient for CDP lifecycle management.

License:
  tungstenite, base64, and tempfile are MIT OR Apache-2.0. Local license files inspected under ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/{tungstenite-0.30.0,base64-0.23.1,tempfile-3.27.0}/LICENSE-*.

Security status:
  crates.io registry resolution completed through Cargo. No postinstall scripts, managed browser downloads, or network-at-startup behavior are introduced. Browser execution remains explicitly capability-gated and sandbox-plan authorized before spawn.

Maintenance state:
  tungstenite is a maintained Rust WebSocket implementation. base64 and tempfile are core Rust ecosystem crates used widely for protocol payload handling and temporary filesystem lifecycle.

Runtime/bundle cost:
  Adds a small WebSocket stack and temporary-directory helper to ac-verification. No browser binary is bundled; the runtime discovers an explicitly configured or system-installed Chrome/Chromium executable and reports Unavailable when absent.

Install/postinstall behavior:
  Cargo builds Rust crates only. No global installs, browser downloads, or external installer scripts.

Why existing components are insufficient:
  The deterministic HTML harness cannot provide real navigation, DOM state, console/page-error events, network observations, accessibility tree data, or screenshot pixels. Direct CDP requires WebSocket framing and screenshot payload decoding.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  Codex, 2026-08-25. Evidence: Cargo.lock entries; focused tests cargo test -p ac-verification and elevated real-browser test cargo test -p ac-verification batch4_browser_runtime_drives_real_chrome_dom_and_evidence -- --nocapture.
