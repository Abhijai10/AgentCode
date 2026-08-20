# WP11 Browser and QA Adoption Decision

| Mechanism | Classification | AgentCode decision |
|---|---:|---|
| Playwright isolated `BrowserContext` | TAKE | Browser Broker owns contexts per task/run; Kernel owns authorization and final decision. |
| Playwright page/network/error events | TAKE | Persist as evidence events with URL, context id, timestamp, and tool version. |
| Playwright JSON reporter | TAKE | Normalize QA results into `ValidationRunReport`; keep attachments in Evidence Store. |
| browser-use event bus/browser session | ADAPT | Use event-driven observation, but keep planning outside browser tool. |
| browser-use resilient event bus | TAKE | Resume/restart must tolerate torn-down sessions and mark state degraded. |
| browser-use coupled agent/browser loop | REJECT | AgentCode separates Browser Tool, Runtime Worker, and Kernel authority. |
| Dyad screenshot nonzero evidence | ADAPT | Store screenshots and metadata, but require assertions/visual review before validation. |
| Dyad rapid file-switch regression | TAKE | Future browser QA must include stale-target UI tests. |
| ZAP full active scan | WRAP | Only through explicit security policy, scope, timeout, and approval. |
| Uncontrolled navigation/form actions | REJECT | Browser actions require Tool Broker policy, provenance, and cancellation. |

Security implications: browser contexts may contain cookies, local storage, screenshots, and secrets. Evidence storage must redact or gate access; cross-origin and credentialed sessions require explicit permission.
