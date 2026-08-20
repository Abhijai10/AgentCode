# ADOPTION_04_AGENT_RUNTIME

Phase 1 WP04 adoption decision.

## Decision

AgentCode will implement an Agent Runtime as a replaceable execution harness for
logical roles. It will take the Gemini CLI loop/event/cancellation shape, adapt
software-agent-sdk persistence concepts, and reject transcript-as-truth.

## TAKE

- Stream start/end event discipline, event ids, and single terminal event from
  Gemini CLI `LegacyAgentProtocol`.
- Model/tool continuation loop from Gemini CLI `_runLoop()`.
- Explicit terminal/non-terminal execution states from software-agent-sdk
  `ConversationExecutionStatus`.

## ADAPT

- software-agent-sdk `ConversationState` event log and active-branch concepts into
  AgentCode session/evidence storage, while Kernel owns mission truth.
- mini-SWE-agent trajectory JSON into raw evidence artifacts.
- Local/remote conversation factory pattern into Worker host selection.

## WRAP

- Checkpoint creation through AgentCode Git/Checkpoint engine.
- Provider calls through Model Broker/Provider Fabric, never direct from runtime.
- Tools through Tool Broker only.

## IGNORE / REJECT

- Reject any donor pattern where `messages` or conversation transcript determines
  task completion.
- Reject runtime direct mutation of mission/task DAG.
- Ignore product-specific UI event grouping as architecture input for runtime.

## Required Runtime Contract

```text
AgentSession {
  agent_session_id
  mission_id
  task_id?
  worker_id?
  role
  model_route_ref
  context_pack_id
  permission_profile_ref
  skill_refs[]
  status
  started_at
  last_runtime_heartbeat_at
  last_model_turn_at
  current_operation_id?
  stop_reason?
  predecessor_session_id?
}
```

Session status is never task status.

