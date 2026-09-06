# License Review Workflow

Status: ACCEPTED (P00-WP04). Owner: project governance / legal posture.

## Purpose

Every piece of external material considered for AgentCode (runtime dependency, binary
invocation, fork, adaptation, pattern study, vendored code) passes a recorded license
review before use. License compatibility is never inferred from repository popularity —
actual license files are inspected.

## Review Steps

1. **Locate** the license file(s): `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING`,
   `NOTICE`, `THIRD_PARTY_NOTICES*`, package metadata (`Cargo.toml`/`package.json`
   `license` field, PyPI metadata). Record the exact path and commit.
2. **Classify** per `docs/legal/OSS_LICENSE_MATRIX.md`:
   - `DIRECT_RUNTIME_DEPENDENCY` — linked/bundled into AgentCode artifacts.
   - `BINARY_TOOL_INVOCATION` — external tool invoked out-of-process.
   - `FORK` — modified copy of upstream source maintained by AgentCode.
   - `SOURCE_ADAPTATION` — AgentCode-derived source materially based on donor code.
   - `PATTERN_STUDY_ONLY` — behavior studied; no code copied.
   - `NO_PLANNED_REUSE` — cataloged, not used.
3. **Gate** per `DEPENDENCY_ADMISSION.md` license table.
4. **Record** in `docs/legal/OSS_LICENSE_MATRIX.md`, `THIRD_PARTY_NOTICES.md`, and
   machine-readable `third_party_manifest.json`.
5. **Attribution** obligations (Doc 07 §93) are captured in THIRD_PARTY_NOTICES.md
   before any code is committed.
6. **Review** by an independent reviewer for HIGH/CRITICAL risk items; record outcome.

## Compatibility Notes (V1 posture)

- Allowed runtime licenses: MIT, Apache-2.0, BSD-2/3, ISC, Zlib, MPL-2.0 (with
  boundary notes), CC0.
- Strong copyleft (GPL/AGPL) and non-OSS source-available (BSL/SSPL) runtime
  dependencies are blocked for V1 without an approved amendment.
- Python security tooling (external processes, not linked code) may retain their own
  licenses; each is recorded with classification `BINARY_TOOL_INVOCATION`.

## Manifest Schema (docs/legal/third_party_manifest.json)

```text
{
  "schema_version": 1,
  "entries": [
    {
      "id": "THIRD-<NNN>",
      "name": "...",
      "version": "...",
      "license": "...",
      "license_file": "path or URL + commit",
      "classification": "DIRECT_RUNTIME_DEPENDENCY | BINARY_TOOL_INVOCATION | FORK | SOURCE_ADAPTATION | PATTERN_STUDY_ONLY | NO_PLANNED_REUSE",
      "source": "upstream URL + pinned commit",
      "usage": "what AgentCode uses it for",
      "attribution_required": true/false,
      "review_status": "PENDING | PASS | BLOCKED",
      "reviewed_at": "date",
      "notes": "..."
    }
  ]
}
```

## Known Exceptions

Any exception (e.g., a needed tool with an unusual license) is recorded explicitly in
`OSS_LICENSE_MATRIX.md` under "Known Exceptions" with justification and an owner
sign-off — it is never silent (P1-G7).

## Workflow Enforcement

- CI secret/path checks refuse new files under `docs/legal/` that do not update the
  manifest (lightweight check in `scripts/check-legal-registry.mjs`).
- Phase gates P0-G5 (process exists), P1-G3/P1-G7 (matrix coverage + exceptions) and
  G-PHASE-10 (licensing) are evaluated against these records.