# P01-WP10 — Security / Isolation Adoption Decision

## Decision

AgentCode will treat all extensions, hooks, MCP servers, tools, repositories, generated code, and scanner output as untrusted. Skills/hooks/MCP are capability-requesting extensions, not policy authorities. The Kernel and Tool Broker own grants; Process Manager and sandbox backends enforce them.

## Mechanism Classifications

| Mechanism | Donor evidence | Classification | Rationale |
|---|---|---:|---|
| Extension capability/policy/enforcement split | Codex connector/plugin policy files | TAKE | Prevents extension metadata from becoming authority. |
| Extension sandbox tests | Codex `extension_sandbox.rs` | TAKE | Required acceptance shape for malicious extension behavior. |
| Hook `beforeTool` interception | Cline plugin example | ADAPT | Useful lifecycle point; core security hooks must fail closed. |
| Path extraction from structured tool inputs and patches | Cline `gitignore-read-files-guard.ts` | TAKE | Needed for preflight policy decisions on edit/read tools. |
| Optional plugin fail-open behavior | Cline plugin example | REJECT for security | Security policy cannot silently allow on checker failure. |
| ACP permission presentation | Cline `apps/cli/src/acp/permissions.ts` | ADAPT | UI protocol is useful; AgentCode needs scoped approval semantics. |
| `allow_always` permission option | Cline | REJECT as broad grant | Must be replaced by policy-scoped, expiring grants. |
| Single committer / isolated worktree / no prod credentials | Munder Difflin | TAKE | Aligns with Kernel authority and blast-radius control. |
| Safe upload filename and absolute workspace resolution | OpenHands `workspace-upload-path.ts` | TAKE | Prevents path traversal and remote/local cwd confusion. |
| Session API key runtime access | OpenHands runtime service | WRAP | Use scoped credentials through Secret Broker, not raw UI/session state. |
| Generated safety policy | Gemini Conseca policy files | STUDY/ADAPT | Generated policy is reviewable input, never direct authority. |
| Security scanner raw output | Doc 07 + donor policy separation | TAKE | Scanner text may be hostile; normalize into structured findings. |

## Security Rejections

- No plugin/hook/MCP text may override project rules, Kernel state, Tool Broker permissions, or secret policy.
- No built-in extension bypasses Tool Broker.
- No duplicate security finding database per scanner.
- No raw scanner output or repository text becomes system/developer instruction.
- No production credentials in Worker environment.

## Architecture Decision

TAKE the capability/policy/enforcement split and single-committer isolation principle. ADAPT hook/MCP lifecycles as untrusted, timeout-bound capability requests. WRAP remote runtimes and extension processes. REJECT broad persistent permissions and fail-open security hooks.

