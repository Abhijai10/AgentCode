# Architecture Decision Records (ADR)

ADRs record **constrained implementation decisions** — choices the locked architecture
(Docs 01–11) leaves open, plus any future amendment to the locked architecture itself.

## Rules

1. Do **not** create ADRs that reopen already-locked architecture (Kernel ownership,
   SQLite control plane, OmniRoute provider architecture, Tool Broker ownership,
   Git/worktree architecture, Tauri desktop direction, Code Intelligence authority,
   Context authority, Verification/evidence authority). Those require an approved
   amendment path, not an ADR.
2. ADRs are used for genuinely constrained implementation decisions (language/package
   split, IPC transport, migration framework, tool policy, etc.).
3. A decision that cannot yet be resolved responsibly receives an explicit **bounded
   `PENDING`** status stating exactly which evidence resolves it and when it must be
   revisited.
4. Statuses: `PROPOSED` → `ACCEPTED` | `REJECTED` | `SUPERSEDED`; bounded `PENDING`
   allowed for unresolved-but-time-boxed choices.
5. Every ADR records: ID, title, status, decision class, problem, constraints,
   considered alternatives, evidence, decision, consequences, affected
   interfaces/docs, supersession relationship.
6. Superseding an ADR requires a new ADR referencing the superseded one.

## Registry

| ID | Title | Status |
|----|-------|--------|
| ADR-0001 | Language and package split | ACCEPTED |
| ADR-0002 | Workspace layout | ACCEPTED |
| ADR-0003 | Build/task orchestration | ACCEPTED |
| ADR-0004 | SQLite driver and migration framework | ACCEPTED |
| ADR-0005 | Local IPC direction | ACCEPTED (transport bounded-PENDING sub-item) |
| ADR-0006 | OmniRoute placement in workspace | ACCEPTED |
| ADR-0007 | Desktop shell stack and boundary | ACCEPTED |
| ADR-0008 | External binary/tool management policy | ACCEPTED |
| ADR-0009 | Structured logging foundation | ACCEPTED |
| ADR-0010 | Configuration layering | ACCEPTED |
| ADR-0011 | Test and fixture organization | ACCEPTED |
| ADR-0012 | Dependency admission policy | ACCEPTED |
| ADR-0013 | CI strategy | ACCEPTED |

## Amendment Path (locked architecture)

Locked architecture changes require:

1. A proposal ADR describing the problem and the proposed change;
2. Reconciliation of all dependent core documents (docs/registry/documents.json lists
   dependencies);
3. Approval recorded in the ADR and the document registry revision bumped;
4. Doc 10 gate re-runs affected by the change.

See `docs/registry/documents.json` for the authority statement and dependencies.