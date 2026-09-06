# WP16 Product UX Adoption Decision

| Mechanism | Classification | AgentCode decision |
|---|---:|---|
| Dyad page-object e2e workflows | TAKE | Product UX gates should exercise real workflows via stable page objects. |
| Dyad screenshot artifact checks | ADAPT | Use screenshots as UX evidence with visual/a11y checks. |
| Dyad rapid editor switch test | TAKE | Required stale-target regression pattern. |
| Bolt artifact action status | ADAPT | Surface Kernel/Worker action states, not UI-local guesses. |
| Bolt workbench navigation | TAKE | Evidence/files/actions need direct inspectable entry points. |
| Browser globals for uploaded screenshots | REJECT | Use typed Evidence Store attachments. |
| Continue context provider menu | ADAPT | UX should expose context sources with authority labels and permission status. |
| UI state as source of truth | REJECT | Kernel and evidence stores own truth. |

Security implications: UX must not leak secrets through screenshots, logs, context previews, or action histories.
