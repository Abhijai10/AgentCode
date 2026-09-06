# P01-WP09 — Sandbox & Permissions Extraction

## Scope

Extraction-only campaign for command interception, filesystem scope, network scope, environment filtering, approval semantics, sandbox launch, denial parsing, and cleanup.

## Donors Inspected

| Donor | Catalog ref | Pinned commit | Files inspected |
|---|---:|---|---|
| Codex | REF-013 | `77e688960196dbc82bbeb00c844d2555a61925aa` | `codex-rs/core/src/exec_policy.rs`; `codex-rs/core/src/tools/sandboxing.rs`; `codex-rs/exec-server/src/fs_sandbox.rs`; `codex-rs/config/src/shell_environment_policy.rs`; `codex-rs/exec/tests/suite/sandbox.rs`; `codex-rs/core/tests/suite/exec_policy.rs`; `codex-rs/core/tests/suite/extension_sandbox.rs` |
| Gemini CLI | REF-020 | `571851b1077a51cef757146ce13f9da887326bec` | `packages/core/src/services/sandboxManager.ts`; `packages/core/src/sandbox/macos/MacOsSandboxManager.ts`; `packages/core/src/policy/sandboxPolicyManager.ts`; `packages/core/src/services/sandboxManager.test.ts`; `packages/core/src/sandbox/macos/MacOsSandboxManager.test.ts` |
| OpenHands | REF-036 | `b25f9b3969f924f37440fee908ff35309ec6eea2` | `src/api/runtime-service/agent-server-runtime-service.ts`; `src/api/cloud/sandbox-service.api.ts`; `src/api/workspace-upload-path.ts`; `__tests__/api/cloud/sandbox-service.test.ts`; `__tests__/api/runtime-service/agent-server-runtime-service.test.ts` |
| SWE-ReX | REF-057 | `5c995c365dfb` | `src/swerex/deployment/docker.py`; `tests/test_docker_deployment.py`; `tests/test_runtime.py` |
| Cline | REF-009 | `8a038022a439f401d78764a059e1561578848e81` | `apps/cli/src/acp/permissions.ts`; `docs/sdk/guides/permission-handling.mdx` |
| OpenCode | REF-034 | `9b0dd36cda0b9accb429a7f9f9ad9b054a27d04a` | `packages/core/src/permission.ts`; `packages/opencode/src/acp/permission.ts`; `packages/opencode/test/acp/permission.test.ts`; `packages/core/test/permission.test.ts` |

## Mechanisms

### Exec Policy and Approval Classification

Codex `codex-rs/core/src/exec_policy.rs` is the strongest donor. Important symbols: `ExecPolicyCommandOrigin`, `UnmatchedCommandContext`, `BANNED_PREFIX_SUGGESTIONS`, `prompt_is_rejected_by_policy`, `ExecPolicyError`, `child_uses_parent_exec_policy`, and dangerous-command helpers.

Control flow:

1. Parse shell commands, including lowered shell-wrapper commands.
2. Match explicit policy rules before heuristics.
3. Classify unmatched commands using safe-command and dangerous-command heuristics.
4. Convert rule and heuristic outcomes into `ExecApprovalRequirement::{Skip, NeedsApproval, Forbidden}`.
5. Reject approval prompts when `AskForApproval::Never` or granular approval flags disallow asking.
6. For approved/skipped commands, optionally propose scoped exec-policy amendments but ban broad prefixes such as shells, interpreters, package-run commands, `git`, `rm`, and `sudo`.

Failure behavior:

- Policy file read/parse errors are typed.
- Dangerous commands can be rejected before execution when approvals are disabled.
- Environment command restrictions override saved prefix approvals.
- Policy changes invalidate session approvals.

Tests inspected:

- `core/tests/suite/exec_policy.rs` covers forced `rm` approval/rejection, nested command parsing, environment restrictions overriding saved approvals, and policy-change invalidation.
- `cli/tests/execpolicy.rs` covers command policy behavior at the CLI boundary.

AgentCode adaptation:

- TAKE the three-way `Skip / NeedsApproval / Forbidden` decision shape.
- TAKE banned approval-prefix concept.
- ADAPT policy layering to AgentCode's Risk/Capability policy and Kernel/Tool Broker ownership.

### Tool Runtime Approval Cache and Sandbox Attempts

Codex `codex-rs/core/src/tools/sandboxing.rs` defines `ApprovalStore`, `with_cached_approval`, `PermissionRequestPayload`, `ExecApprovalRequirement`, `default_exec_approval_requirement`, `SandboxOverride`, and sandbox retry primitives.

Control flow:

1. Build approval keys per tool call.
2. If all keys have `ApprovedForSession`, skip prompting.
3. Otherwise fetch review decision, emit telemetry, and cache per-key session approvals only for approved-for-session.
4. Determine default execution approval from `AskForApproval` and active filesystem sandbox kind.
5. Refuse bypassing sandbox when deny-read restrictions require sandbox enforcement.

Failure behavior:

- Empty approval keys do not use the cache.
- Granular config can forbid sandbox approval prompts.
- Sandbox bypass is not allowed if it would discard deny-read restrictions.

AgentCode adaptation:

- TAKE per-key approval cache and telemetry.
- ADAPT cache invalidation with policy version, workspace id, tool id, cwd, and permission profile.
- REJECT any approval cache that survives policy/workspace changes without invalidation evidence.

### Filesystem Sandbox Enforcement

Codex `codex-rs/exec-server/src/fs_sandbox.rs` defines `FileSystemSandboxRunner`, `sandbox_command`, `SandboxCwd`, `native_workspace_root`, `helper_read_roots`, and policy normalization helpers.

Control flow:

1. Convert sandbox context cwd/workspace roots from `PathUri` to native absolute paths.
2. Materialize project-root aliases against workspace roots.
3. Build `FileSystemSandboxPolicy` and minimal helper read roots.
4. Force restricted network for filesystem helper.
5. Select a sandbox with `SandboxablePreference::Require`.
6. If no sandbox backend can enforce policy, return invalid request: filesystem sandbox cannot be enforced.
7. Transform the helper command for direct sandboxed spawn with native workspace roots and runtime paths.

Failure behavior:

- Dynamic permissions without cwd are invalid.
- Foreign/non-native path URIs are invalid.
- Missing sandbox backend is a hard failure, not a no-op.

Tests inspected:

- `exec/tests/suite/sandbox.rs` covers canonical allowed paths, sandbox root behavior, and metadata handling.
- `core/tests/suite/extension_sandbox.rs` covers extension sandbox permission behavior.

AgentCode adaptation:

- TAKE hard-fail when sandbox enforcement is required but unavailable.
- TAKE URI/native path split and workspace-root materialization.
- ADAPT for macOS target: sandbox claims must name the actual backend and enforcement level.

### Environment Filtering

Codex `codex-rs/config/src/shell_environment_policy.rs` defines `ShellEnvironmentPolicyToml`, filter validation, case-insensitive duplicate rejection, legacy exclude/include migration, explicit `set`, and conversion into runtime `ShellEnvironmentPolicy`.

Failure behavior:

- `filters` cannot mix with legacy `exclude`/`include_only`.
- Duplicate filters ignoring case are rejected.

AgentCode adaptation:

- TAKE environment policy as typed config with case-insensitive matching.
- ADAPT defaults to exclude secrets unless explicitly passed by Secret Broker.

### macOS Sandbox Profiles and Path Resolution

Gemini CLI `packages/core/src/services/sandboxManager.ts` defines `SandboxManager`, `ExecutionPolicy`, `SandboxModeConfig`, `ResolvedSandboxPaths`, `GOVERNANCE_FILES`, `SECRET_FILES`, `findSecretFiles`, `NoopSandboxManager`, and `resolveSandboxPaths`.

`MacOsSandboxManager.prepareCommand`:

1. Initializes shell parsers.
2. Sanitizes environment.
3. Rejects override attempts when mode forbids them.
4. Translates virtual read/write commands.
5. Computes readonly, yolo, default network, persistent command permissions, and additional request permissions.
6. Adds `.git` write access for approved git commands.
7. Resolves original and real paths, forbidden paths, global includes, policy allow/read/write, and git worktree `.git`/common dir.
8. Builds a macOS Seatbelt profile and runs `/usr/bin/sandbox-exec -f <temp-profile> -- <command>`.
9. Cleans temporary profile file.

Tests inspected:

- `services/sandboxManager.test.ts` covers secret file detection, shallow secret scans, forbidden-over-allowed path priority, workspace filtering, and case-insensitive conflicts.
- `sandbox/macos/MacOsSandboxManager.test.ts` covers profile construction and sandbox command preparation.

AgentCode adaptation:

- TAKE realpath expansion, forbidden-over-allowed precedence, secret/governance deny categories, and denial parsing.
- ADAPT Seatbelt backend only after proving macOS enforcement limitations in AgentCode target environment.
- REJECT `NoopSandboxManager` as an execution fallback for restricted operations.

### Remote Runtime / Container Isolation

OpenHands `src/api/runtime-service/agent-server-runtime-service.ts` routes local runtime calls directly through typed SDK clients and cloud runtime calls through `callCloudProxy` with `hostOverride`, `session-api-key`, and timeout. `src/api/cloud/sandbox-service.api.ts` batch-fetches cloud sandboxes from the active cloud backend and refuses non-cloud backends.

SWE-ReX `src/swerex/deployment/docker.py` `DockerDeployment.start` pulls/builds image, finds a free port, generates a container name and auth token, runs Docker/Podman with port mapping, starts a remote runtime with token, waits until alive, and `stop` closes runtime then kills container and optionally removes images.

Failure behavior:

- OpenHands throws for cloud sandbox access on non-cloud backend.
- SWE-ReX `is_alive` fails before start and reports container stdout/stderr if startup terminates.
- Docker pull/build/start failures surface as exceptions.

Tests inspected:

- OpenHands cloud sandbox test verifies cloud API path and bearer auth.
- SWE-ReX docker tests cover start/is_alive/stop and config validation.

AgentCode adaptation:

- WRAP remote/container isolation backends behind Process Manager.
- TAKE health checks, tokenized runtime access, and startup timeout behavior.
- REJECT direct Docker/Podman shell execution from Worker sessions.

## Limitations

- Gemini's macOS `sandbox-exec` path is useful but target-enforceability must be verified; Apple has deprecated parts of this surface.
- SWE-ReX provides process isolation but not AgentCode policy semantics.
- OpenHands browser/frontend runtime calls are not a complete local sandbox model.
- Cline/OpenCode permission UIs do not alone prove sandbox enforcement.

## AgentCode Requirements Extracted

- Sandbox policy is represented before execution and persisted as evidence.
- Every filesystem/process operation enters through Tool Broker/Process Manager.
- Required sandbox enforcement must fail closed.
- Approvals are scoped, cached only with invalidation keys, and never broadened by model text.
- Environment is deny-by-default for secrets; Secret Broker is the only source of secret injection.
- macOS support must distinguish enforceable sandbox guarantees from advisory filtering.

