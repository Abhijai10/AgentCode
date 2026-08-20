# Third-Party Notices

AgentCode is distributed under the license chosen by its owners (to be recorded in
the repository root before any release).

This file is maintained by the license-review workflow (`docs/legal/LICENSE_REVIEW.md`)
and `docs/legal/third_party_manifest.json` (machine-readable companion).

## Currently in the tree

### rusqlite 0.32.1

License: MIT.
Use: SQLite driver for the AgentCode control-plane store.
Admission: `docs/policy/dependency-admissions/DEP-ADM-001-rusqlite.md`.
License inspected at:
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rusqlite-0.32.1/LICENSE`.

### libsqlite3-sys 0.30.1

License: MIT for Rust binding crate; bundled SQLite source remains public domain per
SQLite project terms.
Use: native SQLite binding used by `rusqlite` with the `bundled` feature.
Admission: `docs/policy/dependency-admissions/DEP-ADM-001-rusqlite.md`.
License inspected at:
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libsqlite3-sys-0.30.1/LICENSE`.

## Reference-Research Attribution

The following repositories were studied as research input during Phase 1 (pattern
study). No code was copied. Pinned SHAs, license files and classifications per
repository are recorded in `docs/reference/AGENTCODE_REFERENCE_CATALOG.md`,
`docs/reference/LICENSE_PROVENANCE_MATRIX.md` and `docs/reference/licenses_scan.json`:

OmniRoute, codex, aider, OpenHands, opencode, mini-SWE-agent, SWE-ReX, tree-sitter,
ast-grep, ripgrep, Letta Code, RTK, Gemini CLI, cline, goose, LangGraph, Microsoft
Agent Framework, Playwright, OpenHack, and the other repositories cataloged as
REF-001..REF-062.

Attribution obligations for any `SOURCE_ADAPTATION`/`FORK` material (none admitted so
far) will be added here before the material is committed, per Doc 07 §93.

---
*This file is generated/maintained by the license-review workflow; the authoritative
machine-readable record is `third_party_manifest.json`.*
