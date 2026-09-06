# Phase 1 Architecture Dependency Map

## Primary Chain

```text
Provider Fabric
  -> Agent Runtime
  -> Tools & Editing
  -> Git & Worktrees
  -> Code Intelligence
  -> Context & Memory
  -> Sandbox & Permissions
  -> Security / Isolation
  -> Specialized Capabilities
```

## Ownership Summary

| Layer | Phase 1 packet | Owner boundary for implementation |
|---|---|---|
| Provider Fabric | `P01_WP03_PROVIDER_FABRIC_IMPLEMENTATION_PACKET.md` | Provider/OmniRoute layer owns provider registry, routing, credentials, request/response normalization |
| Agent Runtime | `P01_WP04_AGENT_RUNTIME_IMPLEMENTATION_PACKET.md` | Runtime owns Worker/AgentSession loop; Kernel owns durable truth |
| Tools & Editing | `P01_WP05_TOOLS_EDITING_IMPLEMENTATION_PACKET.md` | Tool Broker owns tool execution; Edit Engine owns ChangeSets/patch application |
| Git & Worktrees | `P01_WP06_GIT_WORKTREES_IMPLEMENTATION_PACKET.md` | Git service owns branch/worktree/evidence state under Kernel control |
| Code Intelligence | `P01_WP07_CODE_INTELLIGENCE_IMPLEMENTATION_PACKET.md` | Code Intelligence owns rebuildable indexes and read-only evidence |
| Context & Memory | `P01_WP08_CONTEXT_MEMORY_IMPLEMENTATION_PACKET.md` | Context Engine owns prompt packs; Memory Service owns accepted facts with provenance |
| Sandbox & Permissions | `P01_WP09_SANDBOX_PERMISSIONS_IMPLEMENTATION_PACKET.md` | Permission/sandbox layer mediates filesystem/process/browser/network operations |
| Security / Isolation | `P01_WP10_SECURITY_ISOLATION_IMPLEMENTATION_PACKET.md` | Security policy owns trust boundaries, secrets isolation, and attack prevention |
| Browser + QA | `P01_WP11_BROWSER_QA_IMPLEMENTATION_PACKET.md` | Browser Broker/QA Runner produce observation and validation evidence |
| Design Studio | `P01_WP12_DESIGN_STUDIO_IMPLEMENTATION_PACKET.md` | Design Studio proposes; Edit/Browser/Verification apply and validate |
| AppSec | `P01_WP13_APPSEC_IMPLEMENTATION_PACKET.md` | Security scanner adapters normalize findings; Kernel decides |
| Cloud Security | `P01_WP14_CLOUD_SECURITY_IMPLEMENTATION_PACKET.md` | Cloud scanner adapters require scoped credentials and policy |
| AI Security | `P01_WP15_AI_SECURITY_IMPLEMENTATION_PACKET.md` | AI Security runner owns eval evidence; Provider Fabric owns model calls |
| Product UX | `P01_WP16_PRODUCT_UX_IMPLEMENTATION_PACKET.md` | UX presents Kernel/evidence state and permission boundaries |
| Packet Handoff | `P01_WP17_ADOPTION_PACKETS_IMPLEMENTATION_PACKET.md` | Implementation phases consume packets and use ADR process for deviations |

## Cross-Cutting Rules

- Kernel state outranks all derived evidence.
- Tool outputs, screenshots, scan reports, model outputs, embeddings, and summaries are evidence, not truth.
- All execution crosses Tool Broker/Sandbox/Permission boundaries.
- All future implementation must cite the relevant Phase 1 packet before coding.
- Donor repositories remain research input only.
