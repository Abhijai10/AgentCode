# P01-WP10 — Security / Isolation Extraction

## Scope

Extraction-only campaign for treating tools, plugins, repositories, generated code, hooks, MCP-style extensions, runtime sandboxes, and security-tool output as untrusted. This document maps to Doc 11 `P01-WP10 — Skills, hooks and MCP extraction campaign` while emphasizing the user's requested Security / Isolation boundary.

## Donors Inspected

| Donor | Catalog ref | Pinned commit | Files inspected |
|---|---:|---|---|
| Codex | REF-013 | `77e688960196dbc82bbeb00c844d2555a61925aa` | `codex-rs/core/tests/suite/extension_sandbox.rs`; `codex-rs/connectors/src/app_tool_policy.rs`; `codex-rs/connectors/src/app_tool_policy_tests.rs`; `codex-rs/core-plugins/src/marketplace_policy.rs`; `codex-rs/core-plugins/src/marketplace_policy_tests.rs`; `codex-rs/core/src/guardian/policy.md`; `codex-rs/core/src/network_policy_decision.rs`; `codex-rs/network-proxy/src/environment_policy.rs` |
| Gemini CLI | REF-020 | `571851b1077a51cef757146ce13f9da887326bec` | `packages/core/src/safety/conseca/policy-enforcer.ts`; `packages/core/src/safety/conseca/policy-generator.ts`; `packages/core/src/policy/policy-engine.ts`; `packages/cli/src/config/extension-manager-permissions.test.ts`; `packages/core/src/policy/topic-policy.test.ts` |
| Cline | REF-009 | `8a038022a439f401d78764a059e1561578848e81` | `sdk/examples/plugins/gitignore-read-files-guard.ts`; `sdk/packages/core/src/hooks/checkpoint-hooks.ts`; `apps/cli/src/acp/permissions.ts`; `apps/vscode/src/core/hooks/hook-factory.ts`; `apps/vscode/src/core/hooks/templates.ts` |
| OpenHands | REF-036 | `b25f9b3969f924f37440fee908ff35309ec6eea2` | `src/api/cloud/proxy.ts`; `src/api/runtime-service/agent-server-runtime-service.ts`; `src/api/workspace-upload-path.ts`; `src/hooks/use-has-permission.ts`; `tests/e2e/mock-llm/*` |
| Munder Difflin | REF-029 | `5fb93721030b` | `blog/src/posts/agent-security-and-sandboxing.md`; `src/main/git.ts`; `src/main/fs.ts` |

## Mechanisms

### Extension and Tool Capability Policy

Codex connector and plugin policy files define separate policy surfaces for app tools, plugin auth, plugin install, marketplace policy, extension sandbox tests, and network/environment policy. The relevant pattern is separation of:

- what a tool/plugin says it can do;
- what policy allows it to do;
- what sandbox/process manager can enforce;
- what evidence is emitted.

Control flow:

1. Extension/tool metadata is parsed as data.
2. Policy determines install/auth/execution permissions.
3. Runtime sandbox and network policy enforce the permission profile.
4. Tests assert extension sandbox behavior and policy-specific allow/deny behavior.

Failure behavior:

- Unavailable network/policy/sandbox states become explicit errors or approval requirements.
- Extension-provided text does not override policy.

AgentCode adaptation:

- TAKE the capability/policy/enforcement split.
- ADAPT plugin/MCP/skill identity into AgentCode extension records with source, version, trust tier, declared capabilities, granted capabilities, and policy hash.

### Hook-Time Guarding

Cline `sdk/examples/plugins/gitignore-read-files-guard.ts` is a concise example of hook-time policy. It defines `FILE_ACCESS_TOOL_NAMES`, extracts paths from structured inputs and apply-patch headers, converts paths to workspace-relative values with escape prevention, checks `git check-ignore -z -v -n --no-index`, parses NUL-delimited output, and returns `{ skip: true, reason }` from `beforeTool`.

Control flow:

1. Plugin `setup` receives workspace root.
2. `beforeTool` ignores unrelated tools.
3. Requested paths are extracted and normalized relative to workspace root.
4. `git check-ignore` determines whether paths are ignored by workspace `.gitignore`.
5. Ignored paths block the tool call.

Failure behavior:

- Git errors log warning and allow rather than hard-block.
- Paths escaping workspace are dropped from plugin enforcement.

AgentCode adaptation:

- TAKE hook lifecycle as an extension point.
- ADAPT path extraction helpers for policy preflight.
- REJECT fail-open behavior for core security policy; optional hooks may fail open only when explicitly non-security.

### Permission Presentation vs Policy Authority

Cline `apps/cli/src/acp/permissions.ts` translates internal `ToolApprovalRequest` into ACP `RequestPermissionRequest` with `allow_once`, `allow_always`, and `reject_once`, sends pending/in-progress/failed tool call updates, and maps unknown/cancelled responses to denial.

AgentCode adaptation:

- TAKE pending/decision update flow.
- ADAPT options to AgentCode scoped approvals with expiry and invalidation keys.
- REJECT `allow_always` without a policy-scoped capability definition.

### Runtime and Repository Isolation

Munder Difflin's `agent-security-and-sandboxing.md` argues for isolated worktrees, single committer, no production credentials, pre-tool policy gate, untrusted inbound validation, and append-only audit. `src/main/git.ts` implements bounded git commands, status/diff/log reads, `safeJoin` path validation for diffs, and explicit binary/large-file handling.

OpenHands runtime APIs isolate cloud/local execution behind agent-server runtime URLs and session API keys. `workspace-upload-path.ts` strips unsafe file names and resolves relative working directories through the agent-server home path before upload.

AgentCode adaptation:

- TAKE single-committer and no-production-credentials principles.
- TAKE safe filename and workspace path resolution.
- WRAP remote runtime access with Kernel-scoped session credentials.
- REJECT renderer/UI direct authority over filesystem paths.

### Policy Generation and Safety Enforcement

Gemini CLI Conseca files (`policy-enforcer.ts`, `policy-generator.ts`) and policy engine tests are useful as examples of a generated/evaluated policy layer. The key AgentCode takeaway is that generated policies are artifacts requiring validation and versioning, not live authority.

AgentCode adaptation:

- ADAPT generated policy as reviewable input.
- REJECT model-generated policy as immediately trusted enforcement.

### Security Scanner and Untrusted Output Boundary

Doc 07 security sections require scanner output and OSS documentation to be treated as untrusted content. This aligns with donor patterns that separate raw output from policy. Scanner/advisory results may contain prompt injection and must be normalized into structured findings before reaching model context.

AgentCode adaptation:

- TAKE untrusted-output principle.
- ADAPT security tools as sandboxed adapters writing to one AgentCode Security Finding Store.

## Tests Inspected

- Codex `core/tests/suite/extension_sandbox.rs` for extension sandbox permissions.
- Codex connector/plugin policy tests for app tool and marketplace policy.
- Gemini extension-manager and topic-policy tests for extension/policy behavior.
- Cline checkpoint-hook tests for hook lifecycle and session resume behavior.
- OpenHands runtime/cloud tests for session-keyed proxy behavior.

## Limitations

- Cline plugin examples are illustrative and may fail open; they cannot be core AgentCode policy.
- Munder Difflin's security material is partly documentation, not full enforcement code.
- OpenHands cloud runtime isolation depends on service boundary; local enforcement is not proven by frontend APIs.
- Gemini generated policy requires validation before use.

## AgentCode Requirements Extracted

- Extension descriptions, hook text, MCP schemas/results, repository docs, generated code, and scanner output are untrusted data.
- Only Kernel/Tool Broker policy can grant capabilities.
- Hook/MCP/plugin execution must have timeout, recursion guard, idempotency key, scoped filesystem/network permissions, and crash isolation.
- Extension output must be normalized before model context or UI display.
- Security findings live in one AgentCode-owned store; no duplicate scanner databases.

