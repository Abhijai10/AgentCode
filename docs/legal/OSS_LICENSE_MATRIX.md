# OSS License Matrix

Status: ACTIVE (Phase 0). Updated by P01-WP02 (reference-library classification) and
by every dependency admission (Phase 2+).

Classification legend:

- `DIRECT_RUNTIME_DEPENDENCY` — linked/bundled into AgentCode artifacts.
- `BINARY_TOOL_INVOCATION` — external tool invoked out-of-process.
- `FORK` — modified copy of upstream source maintained by AgentCode.
- `SOURCE_ADAPTATION` — AgentCode-derived source materially based on donor code.
- `PATTERN_STUDY_ONLY` — behavior studied; no code copied.
- `NO_PLANNED_REUSE` — cataloged, not used.

License gate verdicts: PASS / REVIEW / BLOCKED / PENDING (per `DEPENDENCY_ADMISSION.md`).

## Admitted Dependencies

**None.** No dependency is admitted yet: there are no Cargo.toml/Cargo.lock,
package.json/pnpm-lock.yaml or node_modules in the repository. Nothing may claim a
PASS license verdict, a lockfile pin, or an inspected license file until Phase 2
installs the dependency and the admission record (DEP-ADM-NNN) exists.

## Proposed Phase 2 Candidates (NOT admitted — PROPOSED only)

Declared licenses below are upstream metadata only; license files have NOT been
inspected (no installed artifacts exist). Every row becomes ACTUAL with a pinned
version + inspected license file at its Phase 2 admission.

| Dependency | Declared license | Classification | License review | Resolved version | Admission record |
|------------|------------------|----------------|----------------|------------------|------------------|
| rusqlite (bundled SQLite) | MIT (crate) / Public Domain (SQLite) | DIRECT_RUNTIME_DEPENDENCY | NOT_INSPECTED (PROPOSED) | null (pin at admission) | P02 |
| tracing / tracing-subscriber | MIT OR Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | NOT_INSPECTED (PROPOSED) | null | P02 |
| serde / serde_json / toml | MIT OR Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | NOT_INSPECTED (PROPOSED) | null | P02 |
| anyhow / thiserror | MIT OR Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | NOT_INSPECTED (PROPOSED) | null | P02 |
| pino | MIT | DIRECT_RUNTIME_DEPENDENCY (TS) | NOT_INSPECTED (PROPOSED) | null | P02 |
| react / react-dom | MIT | DIRECT_RUNTIME_DEPENDENCY (desktop renderer) | NOT_INSPECTED (PROPOSED) | null | P02 |
| tauri / tauri-build (Tauri 2) | MIT OR Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY (desktop) | NOT_INSPECTED (PROPOSED) | null | P02 |
| vite / typescript / eslint / vitest / prettier / @tauri-apps/cli | MIT or Apache-2.0 per tool | BINARY_TOOL_INVOCATION (dev) | NOT_INSPECTED (PROPOSED) | null | P02 |

Machine-readable companion: `docs/legal/third_party_manifest.json` (CAND-* entries,
status PROPOSED, review NOT_INSPECTED).

## Foundation Extraction Sources (reference library — research input only)

Classification and license inspection of every cataloged reference repository is
recorded in `docs/reference/LICENSE_PROVENANCE_MATRIX.md` (P01-WP02) and
`docs/reference/licenses_scan.json`. The reference library is never a runtime or
build dependency; no committed path references it.

## Known Exceptions

| Repository | Exception | Justification | Owner sign-off |
|------------|-----------|----------------|----------------|
| daytona (REF-017) | License UNKNOWN — local clone contains no license file | No license file or declared license is present in the clone; upstream license link in README references a tag (v0.190.0) not present in the clone | recorded by P01-WP02; reuse BLOCKED_FOR_COPY until upstream license text is confirmed and reviewed |
| cloudsploit (REF-011) | GPL-3.0 | Strong copyleft: BLOCKED for runtime/adaptation reuse without an approved amendment; cataloged as research/pattern-study only | recorded by P01-WP02 |
| ScoutSuite (REF-048) | GPL-2.0 | Strong copyleft: BLOCKED for runtime/adaptation reuse without an approved amendment; cataloged as research/pattern-study only | recorded by P01-WP02 |
| ctags (REF-016) | GPL-2.0 | Strong copyleft: BLOCKED for reuse; tool-invocation boundary would need review per ADR-0008 | recorded by P01-WP02 |

## Governing Rules

1. No code is copied without a `SOURCE_ADAPTATION`/`FORK` classification, license
   inspection, and attribution in `THIRD_PARTY_NOTICES.md`.
2. `PATTERN_STUDY_ONLY` implies the AgentCode implementation is original; no
   verbatim code blocks are taken from the donor.
3. This matrix is updated in the same commit as the dependency admission.
4. No dependency row may claim PASS/inspected/pinned state before its actual
   admission (Repair 5, 2026-08-20 recovery run).
