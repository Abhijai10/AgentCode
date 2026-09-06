# AgentCode Core Document Registry

This directory is the canonical registry of the **AgentCode V1 core documents** (Docs 01–11).

## Authority Statement

Docs 01–11 define the AgentCode V1 architecture, product contract, roadmap, acceptance
gates and phase-wise implementation playbook. They are the **locked baseline** for every
implementation decision in this repository.

- If existing code conflicts with Docs 01–11 and no approved amendment exists, the code
  is **implementation drift** and must be repaired.
- No document in this registry may be modified casually. Amendments require an ADR and
  reconciliation of dependent documents (see `docs/adr/README.md`).
- The machine-readable companion registry is `docs/registry/documents.json`.

## Document Index

| ID | File | Title | Rev | Status |
|----|------|-------|-----|--------|
| DOC-01 | `01_Model_Selector.md` | Model, Provider, Routing & Reliability Architecture Specification | 1 | FINAL (locked baseline) |
| DOC-02 | `02_Code_Intelligence_Context_Persistent_Memory.md` | Code Intelligence, Context & Persistent Memory Architecture | 2 | FINAL (locked baseline) |
| DOC-03 | `03_Autonomy_Kernel_Agent_Runtime.md` | Autonomy Kernel & Agent Runtime Architecture | 2 | FINAL (locked baseline) |
| DOC-04 | `04_Tools_Edit_Git_Sandbox_Skills_Hooks_HARDENED.md` | Tools, Edit, Git, Sandbox, Skills & Hooks Architecture | 2 | FINAL (locked baseline) |
| DOC-05 | `05_Verification_Security_RedTeam.md` | Verification, Security & Red-Team Architecture | 1 | FINAL (locked baseline) |
| DOC-06 | `06_Design_Studio_Product_UX.md` | Design Studio & Product UX Architecture | 2 | FINAL (locked baseline) |
| DOC-07 | `07_OSS_Extraction_Blueprint.md` | Implementation & OSS Extraction Blueprint | 2 | FINAL (locked baseline) |
| DOC-08 | `08_Product_Requirements_Document_PRD.md` | Product Requirements Document (PRD) | 2 | FINAL (locked baseline) |
| DOC-09 | `09_Master_Project_Roadmap.md` | Master Project Roadmap | 3 | FINAL (locked baseline) |
| DOC-10 | `10_Success_Definition_Acceptance_Gates.md` | Success Definition & Acceptance Gates | 2 | FINAL (locked baseline) |
| DOC-11 | `11_Phase_Wise_Implementation_Playbook.md` | Phase-Wise Implementation Playbook | 2 | FINAL (locked baseline) |

Titles and revisions are read from each document's own header (verified 2026-08-20);
sha256 of each file is recorded in `docs/registry/documents.json`.

## Registry Rules

1. **Canonical IDs** are assigned in `docs/registry/documents.json` and must not be reused.
2. **Renames** must update the registry in the same commit as the file rename; a rename
   that leaves two active authorities fails validation.
3. **Superseded aliases** (if any) are recorded in the registry; there are currently none.
4. **Missing source** must never be silently treated as verified — the registry records
   `"verified": false` until the file and checksum match.

## Related Registries

- `docs/registry/documents.json` — machine-readable core-document registry
- `docs/reference/AGENTCODE_REFERENCE_CATALOG.md` — OSS reference repository catalog
- `docs/adr/README.md` — Architecture Decision Records
- `docs/progress/PHASE_STATUS.md` — phase/milestone tracker
- `docs/legal/` — license matrix and third-party notices