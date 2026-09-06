# EXTRACTION_05_TOOLS_EDITING

Phase 1 WP05 extraction-only report.

## Donors Inspected

| Donor | Catalog ID | Pinned commit | Files inspected |
|---|---:|---|---|
| Gemini CLI | REF-020 | `571851b1077a51cef757146ce13f9da887326bec` | `packages/core/src/scheduler/scheduler.ts`; `scheduler/tool-executor.ts`; `scheduler/policy.ts`; `tools/tool-registry.ts`; `tools/edit.ts`; `tools/shell.ts`; `services/shellExecutionService.ts`; tests for scheduler, shell, edit, policy |
| software-agent-sdk | REF-052 | `98338ff37aea6627777b9978963ab727f51e4f40` | `openhands-sdk/openhands/sdk/tool/tool.py`; `tool/registry.py`; `agent/parallel_executor.py`; `openhands-tools/openhands/tools/file_editor/*`; `terminal/*`; `apply_patch/definition.py`; related tests |
| Aider | REF-002 | `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` | `aider/run_cmd.py`; `aider/coders/*`; `aider/linter.py`; `aider/main.py`; command/edit/lint tests |
| Cline | REF-009 | `8a038022a439f401d78764a059e1561578848e81` | `apps/vscode/src/core/prompts/responses.ts`; `apps/vscode/src/core/hooks/templates.ts`; `apps/vscode/src/core/hooks/hook-factory.ts`; `apps/vscode/src/hosts/vscode/vscode-to-file-migration.ts`; hook/prompt tests |
| OpenCode | REF-034 | `9b0dd36cda0b9accb429a7f9f9ad9b054a27d04a` | `packages/llm/src/tool.ts`; `packages/llm/src/tool-runtime.ts`; `packages/llm/src/protocols/utils/tool-stream.ts`; `packages/sdk/openapi.json`; `packages/opencode/src/worktree/index.ts` |
| mini-SWE-agent | REF-028 | `25941c89cfbc91eb40b3f8756348c91d9977d57e` | `src/minisweagent/environments/local.py`; `models/utils/actions_toolcall.py`; `agents/default.py`; env/model tests |
| Codex | REF-013 | `77e688960196dbc82bbeb00c844d2555a61925aa` | `codex-rs/apply-patch/tests/suite/*`; `codex-rs/git-utils/src/apply.rs`; `codex-rs/windows-sandbox-rs/src/unified_exec/*`; `docs/exec.md`; `docs/execpolicy.md`; app-server command/process tests |
| OpenHands | REF-036 | `b25f9b3969f924f37440fee908ff35309ec6eea2` | `tools/canvas_ui_tool.py`; `tests/e2e/mock-llm/files/mock-llm-files-and-git.spec.ts`; `AGENTS.md` tool/e2e notes |

## Source-Level Findings

### Tool Contracts and Registry

software-agent-sdk has the strongest typed contract. In
`openhands-sdk/openhands/sdk/tool/tool.py`, `ToolAnnotations` carries MCP-style
readOnly/destructive/idempotent/openWorld hints (lines 215-248). `DeclaredResources`
exists so parallel execution can lock resources (`tool.py` lines 250-260).
`ToolExecutor` and `ToolDefinition` define action/observation schema ownership,
executor binding, and JSON-schema validation (same file around lines 283-471).
`tool/registry.py` registers either fixed tool instances with executors or
subclasses with `create(**params)`, and rejects invalid factories.

AgentCode decision: **TAKE** typed ToolDefinition/Action/Observation concepts,
annotations, declared resources, and registry validation. **ADAPT** them under
Doc 04 Tool Broker, which owns capability exposure and policy.

Gemini CLI’s `ToolRegistry` supports built-in and discovered tools. In
`packages/core/src/tools/tool-registry.ts`, `DiscoveredToolInvocation.execute()`
spawns the configured tool-call command with JSON params on stdin, optionally through
`sandboxManager.prepareCommand()` (lines 61-85), captures stdout/stderr/error/exit
code/signal (lines 87-140), and returns a structured error if anything non-zero or
stderr occurs (lines 142-165). The description includes discovery provenance and
execution mechanics (lines 182-201).

AgentCode decision: **ADAPT** dynamic tool discovery and provenance text, but wrap
execution behind Tool Broker. Discovered tools are untrusted by default.

mini-SWE-agent exposes exactly one `bash` tool in
`models/utils/actions_toolcall.py` (`BASH_TOOL`, lines 11-27). `parse_toolcall_actions()`
rejects no tool calls, unknown tools, invalid JSON, and missing `command` (lines
30-76). `format_toolcall_observation_messages()` maps outputs back to tool messages
with raw output, return code, timestamp, and exception info (lines 79-113).

AgentCode decision: **TAKE** strict tool-call validation and raw-output retention;
**IGNORE** single bash-only tool surface.

OpenCode separates canonical tool dispatch from provider IO. In
`packages/llm/src/tool-runtime.ts`, `ToolRuntime.dispatch()` looks up a named tool,
returns structured unknown-tool/no-executor errors, decodes input, executes the tool,
encodes the result, and emits either `LLMEvent.toolError` or `LLMEvent.toolResult`
(lines 18-78). `packages/llm/src/tool.ts` defines type-safe tools with parameter and
success schemas, cached codecs, and `toDefinitions()` for model-facing schemas.
`packages/llm/src/protocols/utils/tool-stream.ts` accumulates streamed tool-call
arguments by provider stream key, emits input start/delta/end events, and finalizes
one or all pending calls depending on provider protocol.

AgentCode decision: **TAKE** canonical dispatch that does not own provider IO or
continuation. **ADAPT** streamed tool-input accumulation for provider normalization.

### Tool Lifecycle and Permission Boundaries

Gemini CLI’s `Scheduler` is a useful lifecycle orchestrator. In
`packages/core/src/scheduler/scheduler.ts`, `schedule()` accepts one or many tool
requests plus an abort signal, enqueues if another batch is active, or starts a batch
immediately (lines 195-223). `_enqueueRequest()` removes queued work on abort and
rejects cancelled work (lines 225-260). `cancelAll()` clears queued and active calls
and marks them cancelled (lines 262-280). The scheduler owns confirmation, policy,
hooks, state, execution, tool modification, telemetry, and MCP progress wiring
(constructor lines 118-138).

AgentCode decision: **TAKE** scheduler lifecycle shape and cancellation semantics.
**ADAPT** to Tool Broker with durable `tool_call_id`, `operation_id`,
`idempotency_key`, policy decision, raw evidence ref, and artifacts.

Gemini CLI’s scheduler imports `checkPolicy`, `resolveConfirmation`,
`evaluateBeforeToolHook`, and `ToolExecutor` (`scheduler.ts` lines 10-15), showing a
clear lifecycle:

```text
validate request
-> policy decision
-> optional confirmation
-> before-tool hook
-> execute
-> after/tool state update
-> telemetry/event
```

AgentCode decision: **TAKE** this order, with AgentCode-specific policy authority.

Cline’s hook templates show concrete PreToolUse/PostToolUse boundaries. In
`apps/vscode/src/core/hooks/templates.ts`, generated PreToolUse input includes
`taskId`, `preToolUse.toolName`, and `parameters`; the sample blocks
`execute_command` containing a dangerous `rm -rf /` pattern and returns JSON with
`cancel` and `errorMessage` (lines around 225-260). PostToolUse input includes tool
name, success, duration, and result (around lines 267-310). `hook-factory.ts`
constructs named hook input envelopes and enriches them with task/workspace context.

AgentCode decision: **ADAPT** named hook phases and JSON block response, but hooks
cannot override Tool Broker/Kernal/secret policy.

### Command Execution

mini-SWE-agent’s `LocalEnvironment.execute()` runs shell commands directly with
combined env, cwd, timeout, stdout+stderr capture, and exception-to-output conversion
(`environments/local.py` lines 24-43). `_run()` uses `subprocess.Popen(shell=True)`,
starts a new process group on POSIX, kills the process group on timeout, and returns a
completed process (lines 72-92).

AgentCode decision: **ADAPT** process-group timeout killing and normalized output.
**REJECT** direct shell execution from an agent/runtime; all commands must go through
Tool Broker/Process Manager, with workspace canonicalization and approval policy.

Aider’s `aider/run_cmd.py` has two command paths: `run_cmd_subprocess()` streams
stdout one character at a time while capturing combined stdout/stderr (lines 42-85),
and `run_cmd_pexpect()` uses an interactive shell under TTY with captured output
(lines 89-132). `run_cmd()` chooses pexpect on non-Windows TTY and falls back to
subprocess, converting `OSError` to `(1, error_message)` (lines 11-23).

AgentCode decision: **STUDY/ADAPT** interactive-vs-noninteractive command execution,
but **REJECT** direct shell execution and one-character model-facing streaming as the
Tool Broker contract.

Gemini CLI’s `ShellToolInvocation` includes structured params (`command`,
`description`, `dir_path`, background/delay and additional permissions; `shell.ts`
lines 86-93), display/explanation helpers (lines 140-166), path simplification for
permission prompts with sensitive directory guards (lines 168-243), and background
PID capture wrapping (lines 111-138). Tests cover shell safety, substitution,
background execution, and service behavior.

AgentCode decision: **TAKE** command metadata and background-process tracking ideas;
**ADAPT** permission prompts into AgentCode risk classes and approval boundaries.

Codex’s `windows-sandbox-rs/src/unified_exec/*` and app-server command/process tests
show platform-specific command execution abstraction, streamed output, and sandbox
preparation. This is implementation evidence for later V1 macOS/Windows boundary
work, not a Phase 1 code source.

AgentCode decision: **STUDY/WRAP**. Use as a reference for process-tree and sandbox
edge cases, not copied architecture.

### Edit Strategies

Gemini CLI’s `tools/edit.ts` implements a layered search/replace edit. It has
`applyReplacement()` for new/existing files (lines 87-107), content hashing
(`hashContent`, lines 109-116), trailing newline restoration (lines 118-129), exact
replacement with multiple-occurrence guard (`calculateExactReplacement`, lines
140-177), flexible line-stripped replacement (`calculateFlexibleReplacement`, lines
179-237), regex/fuzzy recovery paths, path correction, diff display, omission
placeholder detection, and telemetry (`EditStrategyEvent`, `EditCorrectionEvent`).

AgentCode decision: **ADAPT** edit recovery strategies as pre-ChangeSet planning
helpers. The actual write must become an AgentCode ChangeSet with file preimages,
hash preconditions, journal, rollback, and verification hooks per Doc 04.

Cline’s edit recovery prompts are useful failure behavior evidence. In
`apps/vscode/src/core/prompts/responses.ts`, missing `write_to_file` content and
repeated write failures instruct the model to switch to skeleton + smaller
`replace_in_file` chunks after repeated failures (lines 52-93), failed
`replace_in_file` tells the model to reread latest file state and retry with fewer
precise blocks (around lines 301-345), and interrupted edits are described as
reverted to original state (line 245).

AgentCode decision: **ADAPT** failure-specific recovery guidance, but enforce it in
tool results and ChangeSet state rather than only prompt text.

software-agent-sdk’s `openhands-tools/openhands/tools/file_editor/*` provides a
stateful file editor with file cache/history/diff utilities and typed action/
observation wrappers. It is better aligned with a first-class editor subsystem than
mini-SWE’s shell-only approach.

AgentCode decision: **ADAPT** typed editor actions and history/diff concepts, but
AgentCode owns ChangeSet journaling and workspace path policy.

Codex `codex-rs/apply-patch/tests/suite/*` is strong test evidence for patch grammar
and scenarios, including mixed line endings fixtures. `git-utils/src/apply.rs` shows
patch application through Git-aware utility boundaries.

AgentCode decision: **TAKE** patch scenario coverage expectations; **WRAP** patch
application behind Edit Engine, never direct model shell invocation.

### Validation Hooks and Failure Recovery

Gemini CLI has pre-tool hooks and policy integration in scheduler, plus tests
`scheduler_hooks.test.ts`, `scheduler_parallel.test.ts`, `policy.test.ts`, and
tool-specific tests. software-agent-sdk persists hook-blocked actions/messages in
`ConversationState.blocked_actions` and `blocked_messages`
(`conversation/state.py` lines 152-162), which prevents blocked behavior from being
lost across event processing.

AgentCode decision: **TAKE** hook-blocked action persistence as evidence, but store it
under Tool Broker/Kernel event records according to ownership.

Failure recovery requirements for AgentCode:

```text
tool validation failure -> no execution, structured ToolResult
policy denial -> no execution, denial event
approval required -> WAITING_APPROVAL, resumable
command timeout -> kill process tree, raw output retained
edit precondition failure -> no write, stale preimage result
partial ChangeSet apply -> journal reconciliation/rollback
hook block -> no execution, blocker evidence
```

## Tests Inspected

- Gemini CLI: `scheduler.test.ts`, `scheduler_parallel.test.ts`,
  `scheduler_hooks.test.ts`, `policy.test.ts`, `shell.test.ts`,
  `shellBackgroundTools.test.ts`, `edit.test.ts`, `tool-registry.test.ts`.
- software-agent-sdk: `tests/cross/test_registry_directories.py`,
  `test_default_tool_names.py`, `test_toolshield_tool_experience_mapping.py`,
  `tests/agent_server/test_tool_router.py`, terminal/file editor tests in
  `openhands-tools`.
- mini-SWE-agent: `tests/environments/test_local.py`,
  `tests/models/test_actions_toolcall.py`.
- Codex: `codex-rs/apply-patch/tests/suite/*`,
  `codex-rs/windows-sandbox-rs/src/unified_exec/tests.rs`,
  app-server `command_exec.rs` and `process_exec.rs`.
- Aider command/edit/lint paths and tests around command execution and coder edits.
- Cline prompt/hook tests: `writeToFileMissingContentError.test.ts`,
  `toolSpecificMissingParamErrors.test.ts`, hook shell-escape and migration tests.
- OpenCode LLM tool runtime/schema files and OpenAPI tool/session/permission/PTY
  endpoints.

## AgentCode Adaptation Summary

| Mechanism | Classification | AgentCode use |
|---|---|---|
| Typed ToolDefinition/Action/Observation | TAKE | Tool Broker contract. |
| Tool annotations/resources | TAKE | Risk/resource policy and parallel locking. |
| Gemini Scheduler lifecycle | TAKE | Tool-call state machine shape. |
| Dynamic discovered tools | ADAPT | Untrusted tools with provenance and policy filters. |
| Direct subprocess shell execution | REJECT | Must go through Tool Broker/Process Manager. |
| Process-group timeout kill | ADAPT | Process Manager requirement. |
| Gemini edit recovery strategies | ADAPT | Pre-ChangeSet edit planning. |
| Codex apply-patch scenario suite | TAKE | Patch grammar/failure coverage expectation. |
| OpenCode canonical tool dispatch/streamed input accumulator | TAKE | Provider-neutral tool-call normalization. |
| Cline hook phases and edit recovery prompts | ADAPT | Tool Broker hooks and structured recovery results. |
| Aider interactive command execution | STUDY | Useful terminal mode evidence; not direct AgentCode execution. |
| Transcript-only tool observations | ADAPT | Raw evidence retained outside model-facing summaries. |
