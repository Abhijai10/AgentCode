# P01-WP09 Implementation Packet — Sandbox & Permissions

## Future Interfaces Required

```text
PermissionPolicyEngine
  evaluateToolCall(tool_call, context, requested_capabilities) -> PermissionDecision
  evaluateCommand(argv, cwd, env_request, policy_context) -> ExecDecision
  evaluateFilesystem(request, cwd, workspace_roots) -> FsDecision
  evaluateNetwork(request, context) -> NetworkDecision

SandboxManager
  prepareExecution(exec_request, permission_decision) -> SandboxedExecutionPlan
  parseDenial(raw_result) -> SandboxDenial | None
  cleanup(execution_id) -> CleanupResult

ApprovalService
  requestApproval(decision, presentation) -> ApprovalResult
  cacheApproval(scope, decision, invalidation_keys) -> ApprovalCacheRecord
```

## Ownership Boundaries

- Kernel: durable policy version, mission risk mode, capability grants, approval records, sandbox evidence.
- Tool Broker: maps tool requests to required capabilities and asks PermissionPolicyEngine.
- Process Manager: launches sandboxed processes, enforces timeout/cancellation, captures output.
- Secret Broker: injects secrets only through scoped, audited grants.
- UI: displays approval prompt and decision options; does not decide policy.
- Worker/AgentSession: requests capabilities; does not self-authorize.

## State Ownership

Persist:

- Policy version, source layer, and effective permission profile.
- Approval record with scope, user decision, expiry, invalidation keys, and proposed amendment.
- Execution evidence: argv hash/plain allowed subset, cwd, workspace roots, sandbox backend, network mode, env policy id, result, denial parse.
- Sandbox backend capability status: available, degraded, unavailable, platform limitations.

## Security Constraints

- Restricted operation with unavailable sandbox fails closed.
- Deny paths override allow paths.
- Canonicalize and preserve symlink identity; test workspace symlink to `~/.ssh`.
- No raw env inheritance without policy.
- No broad prefix approvals for shells/interpreters/package scripts/git/rm/sudo.
- Approval cache invalidates on policy/workspace/tool/cwd/capability changes.
- Sandbox profile/temp files must have restrictive permissions and cleanup.

## Required Tests for Implementation Phase

- Workspace symlink to secret path cannot be read or written.
- Deny path overrides explicit allow path.
- Network is blocked by default.
- Dangerous nested shell command is rejected before spawn.
- Approval disabled means prompt-required command is forbidden/degraded.
- Sandbox backend unavailable returns explicit blocked state.
- Policy changes invalidate session approvals.
- Environment secret variables are excluded unless Secret Broker grants them.

## Unresolved Questions

- Which macOS backend is acceptable for V1: Seatbelt, containerized execution, or hybrid?
- Whether local developer mode can allow advisory-only sandboxing, and how it is labeled.
- Exact policy language and compatibility with future MCP/plugin capabilities.
- Whether command parsing should be implemented in Rust, tree-sitter shell, or a policy DSL.

