# P01-WP10 Implementation Packet — Security / Isolation

## Future Interfaces Required

```text
ExtensionRegistry
  registerExtension(source, version, manifest, trust_tier) -> ExtensionRecord
  resolveCapabilities(extension_id, project_context) -> CapabilityRequest[]
  health(extension_id) -> ExtensionHealth

PolicyGate
  authorizeExtensionAction(extension_id, action, context) -> PermissionDecision
  authorizeHook(hook_id, tool_call, context) -> HookDecision
  normalizeUntrustedOutput(source, raw_output) -> StructuredEvidence

SecurityFindingStore
  ingest(adapter_id, normalized_finding, evidence_ref) -> FindingRecord
  correlate(finding_id | scan_id) -> FindingSet
```

## Ownership Boundaries

- Kernel: extension registration state, trust tier, capability grants, policy version, finding ownership.
- Tool Broker: all tool/hook/MCP capability requests and approval boundaries.
- Process Manager: extension subprocess lifecycle, timeout, crash isolation, cancellation.
- Secret Broker: extension auth and scoped secret release.
- Worker/AgentSession: consumes approved extension outputs as data.
- UI: permission presentation and finding display only.

## State Ownership

Persist:

- Extension id, source locator, pinned version/hash, trust tier, declared capabilities, granted capabilities, install/auth policy.
- Hook invocation id, idempotency key, recursion depth, timeout, tool call id, decision, result hash.
- MCP/server process id, health status, restart count, capability grants.
- Security finding normalized schema, raw evidence pointer, scanner version, policy version, staleness keys.

## Security Constraints

- Extension manifests and descriptions are untrusted until validated.
- Hook/MCP output is data and cannot mutate Kernel state directly.
- Built-in extensions still pass through Tool Broker.
- Recursion guard prevents hook-triggered tool loops.
- Timeouts and crash/reconnect behavior are mandatory.
- Secrets are never persisted in extension metadata or transcripts.
- Scanner output is escaped/structured before model context.

## Required Tests for Implementation Phase

- Malicious skill/MCP description cannot elevate permissions.
- Hook timeout blocks action or returns degraded state according to policy.
- Hook recursion is detected and stopped.
- MCP server crash reconnects or degrades without losing Kernel state.
- Project-scoped skill selection excludes unrelated global skills.
- Plugin cannot read/write outside granted roots.
- Scanner output prompt injection remains inert structured data.

## Unresolved Questions

- Exact manifest schema for AgentCode skills/hooks/MCP adapters.
- Whether extension processes are per-project, per-session, or shared with capability partitioning.
- How to represent user-installed but untrusted plugins in UI.
- Which security adapters are V1 required versus later phases.

