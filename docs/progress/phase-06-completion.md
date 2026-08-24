# Phase 6 Completion — Basic Worker Agent Loop

## Status

COMPLETE. P06-WP01 through P06-WP09 are ACCEPTED.

## Capabilities

- Ephemeral AgentSession and durable Worker, WorkerTask, TaskAttempt, and retry state.
- Dependency-ordered task decomposition: analyze, diagnose, modify, verify, request completion.
- Worker ownership/lifecycle, cancellation, checkpoint, retry, failure classification, and restart persistence.
- Provider-informed structured planning, context/evidence/memory input, Broker-only tools, verification/repair loop, and bounded stop conditions.
- Evidence-backed CompletionRequest; Kernel remains the mission completion authority.
- Real autonomous isolated-workspace benchmark covering inspect, edit, test, repair, ChangeSet, completion request, merge, and cleanup.

## Validation

P6-G1..P6-G8 pass through the autonomous benchmark and unit/integration coverage. The benchmark performs real tool and Git execution, not a mock-only flow.

## Migration

`0001_kernel_schema.sql` adds `workers`, `tasks`, and `task_attempts`; reopen test covers durable worker/task/attempt state.

## Limitations

- The scheduler is single-worker and deterministic; parallel lease scheduling is deferred to Phase 12.
- The benchmark has one provider class in deterministic CI; additional live-model matrix runs remain optional.
