# WP-P01-WP02 — License and Provenance Matrix

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd (initial docs)
- **Accepted commit:** 1935016 (recovery run; matrix generated at that tree state)
- **Owner modules:** docs/reference/, docs/legal/
- **Architecture refs:** DOC-10 P1-G3, P1-G7; DOC-11 H-P01 §P01-WP02; DOC-07 §89–95
- **Acceptance gates:** P1-G3, P1-G7

## Objective

Truthful license/provenance record for every candidate direct-dependency/adaptation
source, derived from actual license files at pinned SHAs — never inferred.

## Inputs / Outputs

- Inputs: reference catalog (P01-WP01) + the 62 local clones
- Outputs: `docs/reference/LICENSE_PROVENANCE_MATRIX.md`,
  `docs/reference/licenses_scan.json`, updates to
  `docs/legal/OSS_LICENSE_MATRIX.md` (known exceptions)

## Implementation

- Every cataloged repo: pinned HEAD SHA, license file path, SPDX (from file text),
  reuse classification (currently PATTERN_STUDY_ONLY for all — no code copied, no
  extraction decision yet), review state, exception/blocker.
- License text read directly; or-later clauses resolved; dual licenses recorded;
  CC-BY-SA vs CC0 disambiguated; daytona kept UNKNOWN (no license file) and
  BLOCKED_FOR_COPY.

## Tests / Evidence

- 62/62 repos classified from actual files; only daytona UNKNOWN.
- Exceptions recorded in OSS_LICENSE_MATRIX.md Known Exceptions (P1-G7):
  daytona (UNKNOWN), cloudsploit/ScoutSuite/ctags (GPL family), semgrep (LGPL),
  trailofbits-skills (CC-BY-SA-4.0), ripgrep (dual), dyad (portions clause),
  munder-difflin (REJECTED per Doc 07 §25–26).
- Classification verified by re-reading license file headers during the run (sample
  of P0/P1 + all exception repos inspected by direct file read).

## Failure Cases Covered

- Unknown license is never guessed (daytona = UNKNOWN/BLOCKED_FOR_COPY).
- License file presence is verified, not assumed from README claims.
- Or-later/single-version distinctions preserved in SPDX.

## Rollback / Cleanup / Handoff

- Rollback: regenerate from catalog + clones; read-only scan.
- Handoff: P01-WP03..WP17 consult the matrix before any TAKE/ADAPT/WRAP decision;
  P0-G5/G-PHASE-10 records reference it.
