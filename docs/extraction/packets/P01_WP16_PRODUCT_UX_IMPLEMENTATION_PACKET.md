# P01-WP16 Product UX Implementation Packet

## Future Interfaces

```text
ProductEventStream.subscribe(scope) -> KernelEventView
EvidenceNavigator.open(evidenceId) -> EvidenceView
UXRegressionRunner.run(flowSet) -> UXRegressionReport
```

## Ownership

Kernel owns mission/task state. UX owns presentation and user actions. Evidence Store owns artifacts. Browser/QA owns UX regression runs.

## State/Data

UI state is cached view state only. Persist user-visible events, action ids, evidence links, permission prompts, and degraded states from authoritative stores.

## Integration Points

Kernel event stream, Evidence Store, Browser/QA, Design Studio, Security findings, Context Engine.

## Security Constraints

Redact sensitive logs/screenshots; label untrusted context; require approval for risky actions; preserve accessibility.

## Unresolved Questions

What are V1 critical flows? Which accessibility gates block acceptance? How should degraded state be grouped for non-technical UI?
