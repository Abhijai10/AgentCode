# EXTRACTION_04_AGENT_RUNTIME

Phase 1 WP04 extraction-only report.

## Donors Inspected

| Donor | Catalog ID | Pinned commit | Files inspected |
|---|---:|---|---|
| Codex | REF-013 | `77e688960196dbc82bbeb00c844d2555a61925aa` | `codex-rs/code-mode-runtime/src/session_runtime/types.rs`; `codex-rs/code-mode-runtime/src/session_runtime/mod.rs`; `codex-rs/app-server/tests/suite/v2/turn_start.rs`; `turn_interrupt.rs`; `session_end.rs`; `rollout-trace/src/reducer/inference.rs` |
| OpenHands | REF-036 | `b25f9b3969f924f37440fee908ff35309ec6eea2` | frontend event/store files and mock/live E2E docs/tests under `AGENTS.md`, `tests/e2e/mock-llm/*`, `src/types/agent-server/core/*` |
| software-agent-sdk | REF-052 | `98338ff37aea6627777b9978963ab727f51e4f40` | `openhands-sdk/openhands/sdk/conversation/conversation.py`; `conversation/state.py`; `agent/agent.py`; `agent/base.py`; `agent/parallel_executor.py`; `conversation/event_store.py`; tests under `tests/cross/*conversation*` and `tests/agent_server/*conversation*` |
| mini-SWE-agent | REF-028 | `25941c89cfbc91eb40b3f8756348c91d9977d57e` | `src/minisweagent/agents/default.py`; `exceptions.py`; `run/mini.py`; `tests/agents/test_default.py`; `tests/run/test_save.py` |
| Gemini CLI | REF-020 | `571851b1077a51cef757146ce13f9da887326bec` | `packages/core/src/agent/legacy-agent-session.ts`; `agent/event-translator.ts`; `scheduler/scheduler.ts`; `integration-tests/checkpointing.test.ts`; `legacy-agent-session.test.ts` |

## Source-Level Findings

### Runtime Entrypoints

mini-SWE-agent’s runtime entrypoint is `DefaultAgent.run()` in
`src/minisweagent/agents/default.py` lines 88-124. It initializes prompt messages,
loops forever, calls `step()`, handles `FormatError`, `InterruptAgentFlow`, and
uncaught exceptions, saves trajectory in `finally`, and exits when the last message
role is `exit`. `step()` is a direct `query()` then `execute_actions()` call (lines
126-157).

Control flow:

```text
run(task)
-> render system/user templates
-> while not exit:
   -> query model
   -> parse actions
   -> execute env actions
   -> append observation messages
   -> save serialized trajectory
-> return last extra
```

AgentCode decision: **STUDY/IGNORE for durability**. The loop is a clear teaching
model, but its transcript (`messages`) is the main state. AgentCode must not make
transcript the source of truth.

software-agent-sdk’s entrypoint is the `Conversation` factory in
`openhands-sdk/openhands/sdk/conversation/conversation.py`. It chooses
`LocalConversation` or `RemoteConversation` by workspace type (lines 122-235), merges
workspace/plugin/user tags for remote sessions (lines 162-190), rejects local-only
`persistence_dir` on remote workspaces (lines 155-160), and passes callbacks, hooks,
secrets, client tools, and observability metadata to the concrete conversation.

AgentCode decision: **ADAPT** factory separation into Worker-local vs remote session
hosting, but Kernel remains the owner of mission/task truth.

Gemini CLI’s `LegacyAgentProtocol.send()` starts a stream if no stream is active,
emits a user message event, schedules the run loop on a macrotask, and returns a
stream id before events are emitted (`legacy-agent-session.ts` lines 108-153).

AgentCode decision: **TAKE** non-racing stream startup: callers get
`agent_session_id/operation_id/stream_id` before the model loop emits events.

### Session Lifecycle and Model/Tool Loop

Gemini CLI’s `_runLoop()` is the most directly useful model/tool lifecycle. It:

- enforces `maxTurns` (`legacy-agent-session.ts` lines 180-191),
- calls `sendMessageStream()` with an `AbortSignal` (lines 196-202),
- emits translated stream events (line 215),
- fails on model error/invalid/context overflow (lines 217-222),
- collects tool calls (lines 211-213),
- schedules tool calls through `Scheduler.schedule()` (lines 250-253),
- emits tool response events (lines 276-285),
- records completed tool calls and telemetry (lines 292-303),
- stops on stop-tool/fatal-tool errors (lines 305-320),
- feeds tool response parts back into the next model turn (line 323).

AgentCode decision: **TAKE** the loop shape. **ADAPT** state and persistence: model
turns are temporary `AgentSession` operations, while progress/checkpoints/completion
proposals go to Kernel-owned durable records.

software-agent-sdk’s `ConversationState` is a durable event/state structure. It
persists `agent`, `workspace`, `persistence_dir`, execution status, skills, blocked
actions/messages, active branch leaf, stats, secret registry, tags, and `agent_state`
(`conversation/state.py` lines 82-230). Private fields include `_events`, `_view`,
`_cipher`, `_lock`, autosave flags and write guard (lines 232-258). `active_branch()`
returns the current branch rather than abandoned branches (lines 295-300).

AgentCode decision: **ADAPT** event log + branch/head model, but separate it:

```text
Kernel durable state: mission, task DAG, leases, checkpoints, evidence, completion.
Runtime session state: messages, stream ids, tool-call batch, abort controller,
current operation, temporary agent_state.
```

### Continuation, Checkpointing, Resume

mini-SWE-agent serializes trajectory to JSON on every loop iteration (`DefaultAgent.save()`,
lines 182-190) and includes messages, model/env config, costs, API calls, exit status,
and submission (`serialize()`, lines 159-180). This is simple and debuggable, but it
does not distinguish authoritative state from transcript.

AgentCode decision: **ADAPT only as raw evidence**. Store raw session transcript as
evidence/ref, never as task truth.

software-agent-sdk persists conversation state and event logs for resume. Its
`ConversationState.agent_state` explicitly persists feature-specific runtime state
and requires reassignment to trigger autosave (`conversation/state.py` lines
211-219). Tests `tests/cross/test_conversation_restore_behavior.py`,
`test_conversation_lease_behavior.py`, and `tests/agent_server/test_conversation_lease.py`
exercise restore and lease behavior.

AgentCode decision: **TAKE concept, ADAPT authority**. Runtime resume should restore
session-local continuity, then reconcile against Kernel leases/checkpoints. It must
not resurrect stale completion claims.

Gemini CLI has `integration-tests/checkpointing.test.ts` and session utilities for
checkpoint flows. Its runtime stream loop can abort and finish cleanly, but durable
Kernel-like state is not its concern.

AgentCode decision: **WRAP** checkpoint creation behind AgentCode Checkpoint/Git
Engine, not inside provider/runtime loops.

### Event Streams and Cancellation

Gemini CLI emits ordered `AgentEvent`s with generated ids and `streamId` fields
(`_nextEventFields()`, lines 420-425), prevents duplicate event ids in `_emit()`
(lines 327-344), and terminates with one agent_end (`_ensureAgentEnd()`, lines
365-369). `abort()` calls `AbortController.abort()` (lines 141-143), and the loop
checks the signal before, during, and after stream/tool execution (lines 205-209,
236-238, 255-257).

AgentCode decision: **TAKE** event stream idempotency and cancellation checks at
model-stream and tool-boundary points.

software-agent-sdk uses `ConversationExecutionStatus` with non-terminal `IDLE` and
terminal `FINISHED`, `ERROR`, `STUCK` (`conversation/state.py` lines 48-80). This
prevents reconnect snapshots from being mistaken for terminal completion.

AgentCode decision: **TAKE** explicit terminal/non-terminal states. AgentCode session
state is not task state, and `IDLE/STOPPED` cannot mean task complete.

### Worker/Session Boundaries

Doc 03 says AgentSession is temporary. Donors often merge conversation, transcript,
workspace, and agent loop into one object. AgentCode must not.

AgentCode boundary:

```text
Worker identity: durable Kernel-owned assignment/lease.
AgentSession: replaceable runtime process/session for a worker role.
Operation: current model/tool/checkpoint action.
Transcript: raw evidence and context input, not source of truth.
Completion: Kernel transition only after verifier/evidence gates.
```

## Tests Inspected

- Gemini CLI `legacy-agent-session.test.ts`: stream start/end, max turns, abort,
  tool-call continuation, errors.
- Gemini CLI `integration-tests/checkpointing.test.ts`: checkpoint integration.
- software-agent-sdk `tests/cross/test_conversation_restore_behavior.py`,
  `test_conversation_lease_behavior.py`, `tests/agent_server/test_conversation_service.py`,
  `test_event_streaming.py`, `test_event_router_websocket.py`.
- mini-SWE-agent `tests/agents/test_default.py`, `tests/run/test_save.py`,
  `tests/models/test_format_error_response_persistence.py`.
- Codex app-server `turn_start.rs`, `turn_interrupt.rs`, `session_end.rs`.

## AgentCode Adaptation Summary

| Mechanism | Classification | AgentCode use |
|---|---|---|
| mini-SWE `run -> step -> query -> execute` loop | STUDY | Clear control-flow model only. |
| mini-SWE transcript persistence | ADAPT | Raw evidence, not source of truth. |
| Gemini stream/run loop | TAKE | Runtime model/tool loop shape. |
| Gemini event ids and abort propagation | TAKE | AgentSession event stream contract. |
| software-agent-sdk ConversationState | ADAPT | Event/state persistence concepts, with stricter Kernel authority split. |
| Local vs remote conversation factory | ADAPT | Worker host/session boundary. |
| Conversation terminal status enum | TAKE | Avoid false completion from idle/stopped states. |
| Runtime-owned completion | REJECT | Kernel is sole completion authority. |

