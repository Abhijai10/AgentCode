# Phase 0 Independent Understanding Check

Reviewer: <independent model, no implementation context>
Date: 2026-08-20

Scope: governance artifacts only — AGENTS.md, docs/adr/README.md, ADR-0005, docs/policy/DEPENDENCY_ADMISSION.md, docs/legal/LICENSE_REVIEW.md, docs/progress/PHASE_STATUS.md, docs/policy/BRANCH_POLICY.md, docs/coredocs/README.md, docs/registry/documents.json (plus files they reference).

## Q1 — Kernel architecture replacement: PASS

Can the Kernel architecture (Kernel ownership of mission/task/completion truth, SQLite V1 control plane) be replaced casually? No. Required path: the locked-architecture amendment process.

Cited evidence:

- AGENTS.md §2 "No Casual Architecture Drift": "Locked (do not redesign without an approved amendment ADR): — Kernel ownership (mission/task/completion truth) — SQLite V1 control plane (single authoritative Kernel writer; no UI/provider/scanner/Worker direct writes)".
- AGENTS.md §1: "If code conflicts with locked architecture and no approved amendment exists, the code is **implementation drift** — repair it, do not redesign the architecture."
- docs/adr/README.md, Rule 1: "Do **not** create ADRs that reopen already-locked architecture (Kernel ownership, SQLite control plane, ...). Those require an approved amendment path, not an ADR."
- docs/adr/README.md, "Amendment Path (locked architecture)": (1) a proposal ADR describing the problem and the proposed change; (2) reconciliation of all dependent core documents (docs/registry/documents.json lists dependencies); (3) approval recorded in the ADR and the document registry revision bumped; (4) Doc 10 gate re-runs affected by the change.
- docs/registry/documents.json, DOC-03 authority: "Kernel ownership of mission/task/completion truth, SQLite V1 control plane, task leases, single authoritative Kernel writer, daemon process, event/state model" — DOC-03 is status FINAL (locked baseline).

So: no casual replacement; an approved amendment (proposal ADR → dependent-doc reconciliation → recorded approval + registry bump → Doc 10 gate re-runs) is required.

## Q2 — Copying cloned OSS into the tree: PASS

May an implementation agent copy arbitrary cloned OSS from the reference library (or anywhere else) into the AgentCode tree? No — the reference library is research input only, and any external material entering the tree is gated.

Cited evidence:

- AGENTS.md §7: "The reference-library directory (`/Volumes/T7 Shield/GitHub-Repos-dependency`) is **research input only** — never a runtime/build dependency. No committed path may reference it."
- AGENTS.md §6 "No Fake Implementations": forbidden while marking work complete is "arbitrary donor code copied without provenance/license review."
- AGENTS.md §7: "License review before any external material enters the tree (`docs/legal/LICENSE_REVIEW.md`); no license inference from popularity."
- docs/policy/DEPENDENCY_ADMISSION.md, Purpose: the reference library "must never become a runtime or build dependency of AgentCode." Scope: applies to Rust crates, npm packages, external binaries/tools, and "vendored source (forbidden except by explicit ADR + license review)". Process: admission record (DEP-ADM-<NNN>), `make dependency-check`, license review, independent review for HIGH/CRITICAL risk WPs, decision recorded in WP evidence, THIRD_PARTY_NOTICES + third_party_manifest.json updates. Prohibitions: "No runtime/build path that touches `/Volumes/T7 Shield/GitHub-Repos-dependency`", "No dependency whose license file has not been inspected."
- docs/legal/LICENSE_REVIEW.md, Purpose: "Every piece of external material considered for AgentCode (runtime dependency, binary invocation, fork, adaptation, pattern study, vendored code) passes a recorded license review before use." Steps: locate license files, classify per matrix (e.g., FORK, SOURCE_ADAPTATION, PATTERN_STUDY_ONLY), gate per the DEPENDENCY_ADMISSION table, record in matrix/THIRD_PARTY_NOTICES/third_party_manifest.json, attribution before code is committed, independent review for HIGH/CRITICAL items. License gate: strong copyleft (GPL/AGPL), unlicensed/unknown, and non-OSS source-available (BSL/SSPL) are BLOCKED for V1 runtime use without an approved amendment.

So: copying is not "arbitrary" — it is admissible only via the dependency-admission record, license review/gate with inspected license files, provenance/pinning, manifest/attribution updates, and vendored source additionally requires an explicit ADR; the reference library can never be a runtime/build path.

## Q3 — How a phase becomes COMPLETE: PASS

A phase is COMPLETE only when all four elements hold, and explicitly not by file existence or target-matching.

Cited evidence:

- docs/progress/PHASE_STATUS.md, status model: "COMPLETE (only when all Doc 10 gates for the phase pass and completion evidence exists)."
- docs/progress/PHASE_STATUS.md, "Phase Completion Rule": a phase becomes COMPLETE only when (1) all its Work Packages are ACCEPTED, (2) the Doc 10 phase gates pass with recorded evidence, (3) a completion package exists (docs/progress/phase-NN-completion.md + .json), (4) the failure/understanding check passes where Doc 10 requires it. Closing line: "Evidence wins; file existence alone never marks a phase complete."
- AGENTS.md §5: "A WP/phase is ACCEPTED/COMPLETE only when the relevant Doc 10 gates pass with recorded evidence (gate ID, run result, commit, artifacts). Evidence classes E0–E6 per Doc 10. File existence alone is never completion. Never mark a phase COMPLETE to match a target — evidence wins."
- AGENTS.md §12 "Exact Phase Completion": completion package contents are "WP acceptance list, gate runs, failure tests, evidence refs, review record"; "Phase completion requires the independent-understanding check where Doc 10 requires it."
- docs/policy/BRANCH_POLICY.md, rule 6: the completion package is committed on `main` together with the phase's final commit.

Not sufficient: file existence alone; marking COMPLETE to match a target/date; gates passing without recorded evidence (gate ID, run result, commit, artifacts); any single element of the four in isolation.

## Overall verdict: PASS

The independent reviewer's understanding of all three governance questions is correct and fully consistent with the artifacts. One governance-repair finding (below) concerns repo state consistency, not understanding of the rules.

## Findings that require governance repair:

1. **P00 and P02 COMPLETE statuses cite evidence that does not exist.** `docs/progress/PHASE_STATUS.md` marks P00 COMPLETE with "evidence: docs/progress/phase-00-completion.md" and P02 COMPLETE with "evidence: docs/progress/phase-02-completion.md". Neither `docs/progress/phase-00-completion.md` nor `docs/progress/phase-02-completion.md` (nor their `.json` companions) exists in the repository — only `PHASE_STATUS.md`, `roadmap_state.json`, and the P00 WP records exist under `docs/progress/`. Under PHASE_STATUS.md's own Phase Completion Rule (element 3: completion package must exist) and AGENTS.md §5 ("File existence alone is never completion"), these COMPLETE claims cannot yet be validated from the repo. Repair: produce and commit the missing completion packages (with gate runs, failure tests, evidence refs, review records) on `main` per AGENTS.md §12 and BRANCH_POLICY.md rule 6, or correct the tracker to IN_PROGRESS pending that evidence.
2. **Minor wording tension (no repair needed, noted for clarity):** AGENTS.md §2 says locked items require "an approved amendment ADR", while docs/adr/README.md Rule 1 says locked items "require an approved amendment path, not an ADR". These are consistent in substance (the amendment path is a stricter ADR-based procedure — proposal ADR + doc reconciliation + registry bump + gate re-runs), but the two phrasings could be unified to prevent a reader from treating an ordinary ADR as sufficient.

## Resolution (2026-08-20 recovery run)

Both findings repaired:

1. P00/P02 COMPLETE claims retracted and normalized to evidence (P00 = VALIDATING
   until its completion package is committed; P02 = NOT_STARTED — verified there is
   no implementation: no Cargo.toml, package.json, or lockfiles in the tree). The
   Phase 0 completion package (docs/progress/phase-00-completion.md + .json) was
   then produced with recorded gate runs, and P00 status was moved to COMPLETE in
   the same commit, per the tracker's Phase Completion Rule.
2. AGENTS.md §2 wording unified with docs/adr/README.md: "an approved amendment —
   see `docs/adr/README.md` amendment path". The substance (proposal ADR →
   dependent-doc reconciliation → recorded approval + registry bump → Doc 10 gate
   re-runs) is unchanged and identical in both documents.

Re-validation of the three questions after repair: see
`docs/progress/phase-00-governance-validation.md` (Q1/Q2/Q3 PASS; Doc 10 §34
SQLite→Redis failure test PASS).