# WP-P01-WP01 — Reference Catalog and Repository Identity

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd (initial docs)
- **Accepted commit:** 1935016 (recovery run; catalog regenerated at that tree state)
- **Owner modules:** docs/reference/, docs/extraction/
- **Architecture refs:** DOC-10 P1-G1, P1-G2; DOC-11 H-P01 §P01-WP01; DOC-07 §14–19
- **Acceptance gates:** P1-G1, P1-G2

## Objective

Catalog of every reference-library repository with stable catalog IDs, local path,
upstream URL, HEAD SHA, branch, dirty state, license files, category, priority,
maintenance status where locally verifiable, primary AgentCode relevance, extraction
status.

## Inputs / Outputs

- Inputs: `/Volumes/T7 Shield/GitHub-Repos-dependency` (62 repositories)
- Outputs: `docs/reference/AGENTCODE_REFERENCE_CATALOG.md`,
  `docs/reference/agentcode_reference_catalog.json`,
  `docs/reference/licenses_scan.json`,
  `scripts/collect-reference-catalog.mjs`, `scripts/enrich-reference-catalog.mjs`

## Implementation

- Collector scans the reference root: remote URL, branch, HEAD SHA, dirty count,
  license-file names, package.json license field per repo.
- Enricher assigns catalog IDs (REF-001..REF-062), Doc 07 categories/priorities,
  primary role, possible AgentCode use, and reads actual license files to classify
  SPDX (never inferred from popularity).
- Catalog rules: every repo has a recorded SHA before extraction; dirty repos used
  read-only; reference library never a runtime/build dependency.

## Tests / Evidence

- Regenerated 2026-08-20: 62 repositories, all with recorded HEAD SHA (0 missing).
- Dirty state re-checked: only `cline` is dirty (3759 files) — recorded, read-only.
- License classifier corrected during the run (verified against file text):
  - ripgrep: `Unlicense OR MIT` (dual, per COPYING) — was MIT
  - trailofbits-skills: `CC-BY-SA-4.0` (stray "CC0" string had fooled the old
    classifier) — was CC0-1.0
  - ctags `GPL-2.0-or-later`, cloudsploit `GPL-3.0-or-later`, ScoutSuite
    `GPL-2.0-or-later`, semgrep `LGPL-2.1-or-later` — or-later clauses now resolved
  - daytona: UNKNOWN (no license file) — BLOCKED_FOR_COPY
- `node scripts/collect-reference-catalog.mjs` → catalog written: 62 repositories
- `node scripts/enrich-reference-catalog.mjs` → enriched; unknown licenses: daytona
  (no file) only.

## Failure Cases Covered

- Same basename does not imply same repository (each record keyed by directory +
  remote URL).
- Missing source cannot be silently treated as verified (dirty/no-SHA/unlicensed
  states explicit).
- License unknown stays UNKNOWN (daytona), never guessed.

## Rollback / Cleanup / Handoff

- Rollback: regenerate catalog from the two scripts; no reference repo is modified
  (read-only scanning).
- Handoff: P01-WP02 consumes this catalog for the license/provenance matrix;
  P01-WP03..WP17 pin `extracted_at_commit` per repo.
