# WP11 Browser and QA Extraction

## Donors Inspected

| Donor | Commit | Files inspected |
|---|---:|---|
| Playwright | `5f8e7eac8305` | `packages/playwright-core/src/server/browserContext.ts`; `packages/playwright/src/reporters/json.ts` |
| browser-use | `898f23f0b672` | `browser_use/browser/session.py`; `browser_use/agent/service.py` |
| Dyad | `22de43cf0d16` | `e2e-tests/app_screenshot.spec.ts`; `e2e-tests/edit_code.spec.ts` |
| ZAP | `e3793d73d04f` | `docker/zap-full-scan.py` |

## Mechanisms Discovered

### Playwright browser context lifecycle

Source: `packages/playwright-core/src/server/browserContext.ts`.

Important symbols: `BrowserContext`, `BrowserContextEvent`, `_closedStatus`, `_closePromise`, `_permissions`, `_downloads`, `tracing`, `fetchRequest`, `initialize`, `grantPermissions`, `Tracing`.

Control flow: a `BrowserContext` is constructed with browser/options/context id, owns page/network/dialog/download/tracing state, exposes events for page/request/response/requestfailed/pageerror/close, initializes debugger/console/service-worker blocking/permissions, and closes through tracked status and close promise.

Failure behavior: close status prevents double-close ambiguity; service workers can be blocked by option; page errors and failed requests are first-class events rather than hidden logs.

Useful pattern: isolate browser context per task/run and emit typed browser events into Evidence Store.

Limitation: Playwright itself is a browser automation engine, not an authority. AgentCode must wrap it behind Tool Broker policy.

### Playwright JSON QA report

Source: `packages/playwright/src/reporters/json.ts`.

Important symbols: `JSONReporter`, `onConfigure`, `onBegin`, `onError`, `onEnd`, `_serializeReport`, `_serializeTestResult`, `attachments`, `stdout`, `stderr`.

Control flow: reporter captures config, suite, errors; on run end it serializes project config, suites/specs/tests/results, stats, stdout/stderr, retry, steps, and attachments.

Failure behavior: top-level errors are preserved; unexpected/flaky/skipped/expected stats are counted from test outcomes; attachments can be paths or base64 bodies.

Useful pattern: normalize all QA runs into structured report plus raw artifacts.

Limitation: test PASS/FAIL is evidence, not Kernel decision.

### browser-use event-driven observation loop

Sources: `browser_use/browser/session.py`; `browser_use/agent/service.py`.

Important symbols: `BrowserSession`, `Target`, `CDPSession`, `ResilientEventBus`, `BrowserStartEvent`, `NavigateToUrlEvent`, `BrowserStateRequestEvent`, `BrowserErrorEvent`, `Agent`, `AgentHistoryList`, `AgentStepInfo`, `register_should_stop_callback`, `max_failures`, `step_timeout`, `loop_detection_enabled`.

Control flow: `BrowserSession` owns browser profile, CDP sessions, targets, event bus, navigation/state events, reconnect/stop events, and watchdog integration. `Agent` combines task, LLM, browser session, tools, callbacks, sensitive data, max failures, step timeout, screenshots/vision, planning, loop detection, and history.

Failure behavior: `ResilientEventBus` no-ops on torn-down bus for warm resume; agent has `max_failures`, `final_response_after_failure`, step timeout, and stop callbacks.

Useful pattern: event-driven browser observations and explicit stop/failure controls.

Limitations: browser-use couples browser observation to agent planning; AgentCode must split browser tool execution from Kernel decisions.

### Dyad browser QA fixtures

Sources: `e2e-tests/app_screenshot.spec.ts`; `e2e-tests/edit_code.spec.ts`.

Important symbols: `SCREENSHOT_FILENAME_REGEX`, `expect.toPass`, `expectFileContent`, `selectFileAndWaitForEditor`, `replaceEditorContent`.

Control flow: screenshot fixture generates app, waits for preview iframe, checks `.dyad/screenshot/<sha>.png` exists and has nonzero size, then asserts UI preview image visible. Edit fixture uses UI code mode, switches files rapidly, saves, and polls filesystem content.

Failure behavior: polling gives asynchronous UI operations time to settle; rapid-switch test catches stale editor target bugs.

Useful pattern: visual evidence plus filesystem verification for browser-assisted UI changes.

Limitation: raw screenshot existence is not proof of visual correctness.

### ZAP browser/security QA scan workflow

Source: `docker/zap-full-scan.py`.

Important symbols: `main`, `config_dict`, `out_of_scope_dict`, `ignore_scan_rules`, `in_progress_issues`, report flags `-r/-J/-w/-x`, exit codes `0/1/2/3`.

Control flow: parses target/config/context/progress/report flags, starts/spiders target, runs active scan, applies rule severity overrides, writes selected reports, exits by fail/warn/other error.

Failure behavior: invalid options exit 3; warning/fail policy is configurable; scan rules can be ignored or marked in-progress.

Useful pattern: destructive/active browser security scans require explicit policy and output classification.

Limitation: active scanning is unsafe as an autonomous default.

## AgentCode Constraints

Browser/QA flow must remain `Observation -> Evidence -> Validation -> Kernel decision`. Browser actions are tools. Screenshots, DOM snapshots, and QA reports are evidence with provenance and freshness, not truth.
