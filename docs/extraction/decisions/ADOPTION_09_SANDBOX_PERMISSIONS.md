# P01-WP09 — Sandbox & Permissions Adoption Decision

## Decision

AgentCode will implement sandboxing as a Kernel/Tool Broker policy decision plus Process Manager enforcement. Donor sandbox managers may inform backend adapters, but no Worker, plugin, MCP server, or UI may directly grant filesystem, process, network, or secret authority.

## Mechanism Classifications

| Mechanism | Donor evidence | Classification | Rationale |
|---|---|---:|---|
| `Skip / NeedsApproval / Forbidden` approval requirement | Codex `core/src/tools/sandboxing.rs` | TAKE | Cleanly separates no-prompt, prompt, and hard-deny states. |
| Prompt rejection when approval policy forbids asking | Codex `core/src/exec_policy.rs` | TAKE | Prevents false progress under `Never` or granular-disabled approval modes. |
| Banned approval-prefix suggestions | Codex | TAKE | Avoids converting a narrow approval into arbitrary code execution. |
| Shell-wrapper command parsing | Codex | ADAPT | Needed, but parser must be versioned and treated as policy input evidence. |
| Per-key approved-for-session cache | Codex | ADAPT | Keep cache, but include workspace, policy version, tool id, cwd, and capability hash. |
| Required sandbox fail-closed | Codex `exec-server/src/fs_sandbox.rs` | TAKE | If enforcement is required and unavailable, execution must be blocked/degraded. |
| Path URI/native split | Codex | TAKE | Supports remote executors and prevents host/path confusion. |
| Environment policy filters | Codex `shell_environment_policy.rs` | TAKE | Structured env policy is mandatory for secrets isolation. |
| macOS Seatbelt profile generation | Gemini `MacOsSandboxManager.ts` | STUDY/ADAPT | Useful backend reference, but AgentCode must prove enforceability on target macOS. |
| Secret/governance file deny categories | Gemini `sandboxManager.ts` | ADAPT | Make categories configurable/project-scoped and Secret Broker-owned. |
| Forbidden-over-allowed path precedence | Gemini | TAKE | Deny must dominate allow. |
| Noop sandbox fallback | Gemini | REJECT | Silent passthrough violates AgentCode fail-closed policy. |
| Remote/cloud runtime proxy | OpenHands | WRAP | Use as remote executor shape; Kernel still owns policy and evidence. |
| Docker/Podman deployment | SWE-ReX | WRAP | Useful isolation backend, but direct container runtime invocation is Process Manager-only. |
| Permission UI translation | Cline/OpenCode | ADAPT | UI can present decisions; policy engine owns decision semantics. |

## Security Posture

- Default execution is constrained.
- Network is denied unless capability policy grants it.
- Filesystem writes are scoped to workspace/worktree roots and explicit writable entries.
- Secret paths and environment variables are denied unless Secret Broker grants scoped injection.
- Permission prompts cannot invent broader future policy from model-supplied phrasing.
- Sandbox denial output is untrusted evidence, not instructions.

## Architecture Decision

TAKE Codex's policy and fail-closed semantics as the primary model. ADAPT Gemini's path expansion and macOS backend ideas after local enforceability tests. WRAP OpenHands/SWE-ReX remote/container isolation behind AgentCode Process Manager. REJECT no-op sandbox fallbacks and direct unrestricted execution.

