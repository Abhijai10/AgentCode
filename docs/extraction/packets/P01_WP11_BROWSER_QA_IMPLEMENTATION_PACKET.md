# P01-WP11 Browser and QA Implementation Packet

## Future Interfaces

```text
BrowserBroker.openContext(policy) -> BrowserContextReceipt
BrowserBroker.navigate(contextId, url, options) -> BrowserObservation
BrowserBroker.act(contextId, BrowserAction) -> BrowserObservation
BrowserBroker.capture(contextId, CaptureRequest) -> EvidenceId
ValidationRunner.run(TestPlan) -> ValidationRunReport
```

## Ownership

- Kernel owns permission, task decision, and accepted validation result.
- Browser Broker owns browser lifecycle and observations.
- Tool Broker owns process/browser execution boundary.
- Evidence Store owns DOM snapshots, screenshots, traces, HAR, reports, stdout/stderr.
- QA Runner owns test invocation and result normalization.

## State and Data Ownership

Browser session state is temporary unless persisted as evidence. QA reports are immutable evidence. Screenshots are observations, not truth.

## Integration Points

Tool Broker, Sandbox/Permissions, Evidence Store, Kernel validation gates, Design Studio, AppSec.

## Security Constraints

No unrestricted browser actions; isolate profiles; redact cookies/storage; scope active scans; record tool versions, URLs, commit/worktree, viewport, and timeout.

## Unresolved Questions

Which browsers are REQUIRED_V1? What is the default screenshot retention policy? Which active scans require human approval?
