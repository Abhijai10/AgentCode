# Phase 0 ADR Audit (2026-08-20 recovery run)

Audit of ADR-0001..ADR-0013 created before this run. Each entry records: old status,
new status, evidence actually available, missing evidence, next resolving phase,
reason. Decisions are judged against the source-of-truth hierarchy (Docs 01–08 →
ADRs → Doc 09 → Doc 10 → Doc 11 → verified extraction records).

Method: read every ADR; checked each Evidence/Constraints claim against Docs 01–11
text and against the filesystem (docs/extraction/, docs/reference/).

| ADR ID | Old status | New status | Evidence actually available | Missing evidence | Next resolving phase | Reason |
|--------|-----------|-----------|-----------------------------|------------------|----------------------|--------|
| ADR-0001 Language and package split | ACCEPTED | ACCEPTED | Doc 07 §102–103, §104; Doc 06 §8030; Doc 02 §115; Doc 09 H12 | P01-WP03/P01-WP04 extraction confirmation (cited before it existed) | P01 (WP03/WP04) | Decision is directly doc-backed; fake extraction citation removed |
| ADR-0002 Workspace layout | ACCEPTED | ACCEPTED | Doc 09 HC-P02; Doc 11 H-P02, §35 | none | — | Fully doc-backed; no extraction claims |
| ADR-0003 Build/task orchestration | ACCEPTED | ACCEPTED | Doc 11 §36; Doc 09 HC-P02; host tool audit verified this run (make 3.81, cargo 1.96.1, node v26.3.0, pnpm 11.10.0) | none | — | Environment claim re-verified and true |
| ADR-0004 SQLite driver and migration framework | ACCEPTED | ACCEPTED | Doc 03 §0.31; Doc 11 H17/H18; Doc 09 HC-P02 | P01-WP04/P01-WP08 donor-pattern confirmation (cited before it existed) | P01 (WP04) | Driver/migration choice is a constrained Phase 2/3 implementation choice, doc-backed; fake extraction citation removed |
| ADR-0005 Local IPC direction | ACCEPTED (direction) with bounded-PENDING transport sub-item | ACCEPTED (direction) with bounded-PENDING transport sub-item | Doc 07 §104; Doc 03; Doc 11 H17; Doc 06 §4146 | transport decision (Unix-socket vs localhost TCP vs WebSocket) | P03 (daemon IPC prototype, per ADR-0005 deadline) | Correct bounded-PENDING pattern; no extraction claims; satisfies P0-G3 |
| ADR-0006 OmniRoute placement | ACCEPTED | ACCEPTED | Doc 07 §20–24, §97–98, §103; Doc 09 HC-P04; REF-032 pinned SHA in reference catalog | P01-WP03 extraction record (package layout, routing entrypoints, licensing) | P01 (WP03) | Placement is doc-backed; fake extraction citation removed; pinned-SHA claim fixed to reference catalog |
| ADR-0007 Desktop shell stack and boundary | ACCEPTED | ACCEPTED | Doc 06 §8030, §138, §5860; Doc 11 §43; host has Xcode/webkit (macOS) | none | — | Doc-backed; no extraction claims |
| ADR-0008 External binary/tool management policy | ACCEPTED | ACCEPTED | Doc 07 §107–114; Doc 11 H13/H16; Doc 09 HC-P02 | none | P05/P07/P08 (tool adoption) | Policy decision; no extraction claims; TOOL_REGISTRY entries stay open until adopting phase |
| ADR-0009 Structured logging foundation | ACCEPTED | ACCEPTED | Doc 09 HC-P02; Doc 11 H22–H23 | P01-WP04 donor logging confirmation (cited before it existed) | P01 (WP04) | Choice doc-backed; fake extraction citation removed |
| ADR-0010 Configuration layering | ACCEPTED | ACCEPTED | Doc 09 HC-P02; Doc 11 §41 | P01-WP04 donor config confirmation (cited before it existed) | P01 (WP04) | Choice doc-backed; fake extraction citation removed |
| ADR-0011 Test and fixture organization | ACCEPTED | ACCEPTED | Doc 11 §38, H20; Doc 09 H13, HC-P02 | none | P02 (WP04) | Doc-backed; no extraction claims |
| ADR-0012 Dependency admission policy | ACCEPTED | ACCEPTED | Doc 07 §89–98; Doc 11 H12–H13; docs/policy/DEPENDENCY_ADMISSION.md exists | admission records (DEP-ADM-*) — created at first actual dependency admission | P02 | Policy doc exists; no fake evidence; satisfies P0-G4 |
| ADR-0013 CI strategy | ACCEPTED | ACCEPTED | Doc 11 H19; Doc 09 HC-P02; P2-G4 | CI workflow implementation (.github/workflows/ci.yml) — Phase 2 deliverable, not a Phase 0 claim | P02 (WP03) | Decision doc-backed; verification section corrected to future tense; no fake CI-run claim |

## Summary

- 13/13 ADRs remain ACCEPTED — every decision is supported by Docs 01–11 text alone.
- 5 ADRs (0001, 0004, 0006, 0009, 0010) contained extraction-evidence citations for
  records that do not exist; all such claims were removed in this run (see the ADRs
  themselves, which now carry an audit note).
- ADR-0006's "pinned SHA" claim was corrected to point at the reference catalog
  (REF-032).
- ADR-0013's verification text was corrected to future tense (CI has not run).
- No ADR reopens locked architecture (each was checked against the ADR README Rule 1
  list and AGENTS.md §2).
- P0-G3 is satisfied: every foundational unresolved choice has an ADR (1–13) and the
  one genuinely open transport choice has an explicit bounded-PENDING marker with
  resolution evidence and deadline (ADR-0005).
