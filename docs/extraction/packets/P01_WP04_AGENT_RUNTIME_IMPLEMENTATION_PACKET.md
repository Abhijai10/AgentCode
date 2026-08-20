# P01_WP04_AGENT_RUNTIME_IMPLEMENTATION_PACKET

Extraction-only packet for future Agent Runtime implementation.

## Required Control Flow

```text
Kernel assigns task/role/lease
-> Agent Runtime creates AgentSession(CREATED)
-> Context Engine supplies context_pack
-> Model Broker supplies assignment_plan/model_route_ref
-> Runtime starts operation/model turn
-> Provider Fabric streams normalized events
-> Runtime validates assistant message/tool calls
-> Tool Broker executes approved tool requests
-> Runtime sends tool observations into next model turn
-> Worker emits progress/checkpoint/completion proposal
-> Kernel records durable truth
```

## Runtime-Owned Temporary State

- current message buffer and provider stream id
- current tool-call batch
- abort controller/cancel token
- current operation id
- session-local agent_state
- raw transcript/evidence refs
- heartbeat timestamps

## Kernel-Owned Durable State

- mission and original requirements
- task DAG and task state
- worker identity and lease/fencing
- checkpoints accepted for task
- verification evidence
- completion transitions

## Required Failure Semantics

- Provider error is failed model turn, not completed turn.
- Tool failure is observation plus failure class, not automatic task failure.
- Runtime crash allows session replacement if worker lease remains valid.
- Cancellation stops at safest boundary: provider stream, queued tool, running tool,
  checkpoint, or session stop.
- Resume must reconcile session-local state against Kernel lease/task version.

## Tests To Build Later

- happy-path model/tool/model continuation through real runtime caller
- provider stream failure does not mark model turn complete
- user cancellation during stream and during tool execution
- runtime crash and replacement with same worker lease
- stale worker/session cannot emit checkpoint/completion
- transcript-only resume cannot alter task truth
- max-turn stop is session stop, not task completion

## Handoff To Phase 6 / Phase 12

Implement minimal single-worker runtime first. Multi-agent orchestration must reuse
the same session contract rather than invent role-specific loops.

