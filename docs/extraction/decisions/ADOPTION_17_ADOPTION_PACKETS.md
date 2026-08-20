# WP17 Adoption Decisions and Implementation Packets Decision

| Mechanism | Classification | AgentCode decision |
|---|---:|---|
| TAKE/ADAPT/WRAP/REJECT/IGNORE classification | TAKE | Required for every mechanism entering future phases. |
| Source-level donor citations | TAKE | Every conclusion needs repo, commit, path, symbol, flow, failure behavior. |
| Implementation packet contract | TAKE | Future phases consume packet interfaces instead of rereading donors. |
| Donor architecture as authority | REJECT | Donors are evidence only; Docs/ADRs/Kernel architecture win. |
| Phase 1 extraction docs as production code | REJECT | Extraction artifacts guide implementation but do not implement runtime behavior. |

## Decision

WP17 accepts the packet structure used across Phase 1 as the handoff format for implementation phases. Adoption decisions remain scoped to their WPs and cannot silently amend locked architecture.
