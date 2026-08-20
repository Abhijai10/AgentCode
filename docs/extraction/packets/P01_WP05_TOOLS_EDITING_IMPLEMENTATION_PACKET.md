# P01_WP05_TOOLS_EDITING_IMPLEMENTATION_PACKET

Extraction-only packet for future Tool Broker/Edit Engine implementation.

## Required Tool Broker Flow

```text
Agent Runtime submits ToolRequest
-> Tool Broker resolves tool_id/tool_version
-> validate schema and workspace/path refs
-> classify risk and resources
-> evaluate policy and hooks
-> request approval if required
-> execute through owned subsystem
-> capture raw output/artifacts/events
-> normalize ToolResult
-> return observation to Agent Runtime
-> persist execution evidence
```

## Required Edit Flow

```text
model proposes edit
-> parse into EditOperation(s)
-> resolve canonical workspace paths
-> read preimages and hashes
-> plan ChangeSet
-> validate stale preconditions
-> apply atomically where possible
-> journal partial progress
-> run configured validation hooks
-> emit diff/evidence
-> rollback or reconcile on failure/crash
```

## Required Command Flow

```text
command request
-> command parser/risk classifier
-> approval policy
-> sandbox/process profile
-> spawn process group
-> stream bounded output
-> timeout/cancel kills process tree
-> retain raw output ref
-> return normalized result
```

## Required Failure Cases

- invalid tool name or params
- unavailable tool binary/adapter
- policy denial
- approval timeout/rejection
- command timeout with child processes
- background process orphan detection
- file path escapes workspace
- stale preimage/hash mismatch
- multiple matches when single replacement required
- partial ChangeSet after crash
- hook blocks request

## Required Tests Later

- real Agent Runtime -> Tool Broker happy path
- denied destructive command never executes
- command timeout kills child process tree
- background process status and cancellation
- edit stale precondition failure
- multi-file ChangeSet rollback
- patch mixed line endings
- dynamic tool stderr/non-zero normalized as error
- hook-blocked tool persists blocker evidence

