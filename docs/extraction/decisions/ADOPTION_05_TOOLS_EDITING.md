# ADOPTION_05_TOOLS_EDITING

Phase 1 WP05 adoption decision.

## Decision

AgentCode will implement a native Tool Broker and Edit Engine. Donor tool systems
inform contracts, lifecycle, and edge cases, but no donor tool runtime becomes the
AgentCode authority boundary.

## TAKE

- software-agent-sdk typed ToolDefinition/Action/Observation and annotations.
- Gemini CLI scheduler lifecycle: validation, policy, confirmation, hooks, execution,
  telemetry, cancellation.
- mini-SWE-agent strict tool-call parsing and raw observation retention.
- Codex apply-patch scenario coverage expectations.

## ADAPT

- Gemini CLI shell metadata, background PID tracking, and path simplification into
  AgentCode command/process records.
- Gemini CLI edit exact/flexible/regex/fuzzy strategies into Edit Engine planning,
  guarded by ChangeSet preconditions.
- software-agent-sdk declared resources into Tool Broker concurrency locks.

## WRAP

- MCP/discovered/client tools as extension adapters behind native Tool Broker.
- Shell/process execution behind Process Manager.
- Patch application behind ChangeSet journal and rollback.

## IGNORE / REJECT

- Reject direct agent/runtime shell execution.
- Reject edits that write without preimage/hash/path preconditions.
- Reject model-visible raw secrets or unredacted sensitive command output.
- Ignore single-tool bash-only runtime as an AgentCode design.

## Required Contract For Phase 5 / Phase 13

```text
ToolDefinitionRecord
ToolRequest
ToolPolicyDecision
ToolApprovalRequest
ToolExecutionRecord
ToolResult
RawEvidenceRef
ChangeSet
EditOperation
FilePreimage
RollbackPlan
```

