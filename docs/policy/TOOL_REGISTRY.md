# Tool Registry

Status: ACTIVE (ADR-0008). Owner: tool policy / build / CI.

Every external binary/tool used by AgentCode is recorded here with an explicit
installation strategy. Nothing is adopted ad hoc.

## Strategy Classes (Doc 07 §109–114)

- `BUNDLED` — shipped inside AgentCode artifacts (small, stable, permissive license).
- `MANAGED_DOWNLOAD` — fetched by AgentCode from a pinned URL with SHA-256 integrity
  verification; never `curl | sh`.
- `SYSTEM_DEPENDENCY` — provided by the OS/platform with acceptable guarantees.
- `CONTAINERIZED` — run inside a container.
- `REMOTE_API` — accessed as a network service.

## Registry Schema

```text
TOOL-<NNN> — <name>

Capability key:    (used by feature detection, Doc 07 §108)
Strategy:          BUNDLED | MANAGED_DOWNLOAD | SYSTEM_DEPENDENCY | CONTAINERIZED | REMOTE_API
Version/pin:       exact version or commit + SHA-256 for MANAGED_DOWNLOAD
License:           SPDX id; matrix classification; inspection ref (docs/legal/)
Runtime/bundle cost:
Install/postinstall behavior:
Optional (Doc 07 §107): yes/no — is AgentCode fully functional without it?
Adopting phase:    phase that introduces it
Admission record:  docs/policy/dependency-admissions/DEP-ADM-<NNN>.md
```

## Entries (Phase 2)

| ID | Name | Strategy | Adopting phase | Notes |
|----|------|----------|----------------|-------|
| TOOL-001 | git (macOS system) | SYSTEM_DEPENDENCY | P07 (Git foundation) | provided by macOS/Xcode toolchain; not used by Phase 2 build |
| (open) | ripgrep | (candidate: MANAGED_DOWNLOAD or BUNDLED) | P08 | extraction P01-WP07; decision recorded at adoption |

No external binary is required to build/test Phase 2 (P2-G1 clean-build gate).