# ADR-0007 — Desktop Shell Stack and Boundary

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

What is the desktop shell stack, and what is its boundary for Phase 2 (and beyond)?

## Constraints

- Doc 06 §8030: "Tauri 2 + React/TypeScript/Vite is the preferred V1 desktop direction
  unless extraction/testing reveals a serious blocker."
- Doc 06 §138: Tauri 2 desktop shell; native desktop packaging.
- Doc 06 §5860: 8 GB Mac target; desktop must remain lightweight.
- Doc 11 §43: Phase 2 desktop placeholder only needs start, log, exit cleanly; no fake
  UI features.
- Doc 03: desktop never mutates mission DB directly.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| Electron | Mature | Heavyweight; violates 8 GB Mac budget (Doc 06) |
| Tauri 1 | Older | Doc 06 explicitly prefers Tauri 2 |
| Native Swift | macOS-only, not cross-platform | Post-V1 Windows considered; Doc 07 §103 Tauri direction |
| Tauri 2 + React/TS/Vite (chosen) | Matches Doc 06 | — |

## Evidence

- Doc 06 §8030, §138, §5860; Doc 11 §43; host has Xcode + webkit (macOS).

## Decision

- `apps/desktop`: Tauri 2 shell, React 18 + TypeScript + Vite renderer, pnpm-managed
  frontend; the Tauri Rust crate is a standalone cargo package (outside the root Cargo
  workspace) so desktop build issues cannot block core validation.
- Phase 2 shell displays only truthful skeleton state (app name, version, build
  config paths) via one Tauri command reading Rust-side config; no fake mission/discuss/
  design/security features, no daemon connectivity claims.
- The shell never imports `crates/ac-db` or writes SQLite (architecture lint).
- IPC with the daemon arrives in Phase 3 per ADR-0005.

## Consequences

- Positive: preferred direction followed; lightweight; renderer stays presentation-only.
- Negative: Tauri build time is higher than pure web; mitigated by isolation.
- Affected: Doc 06 §138; Phase 6+ desktop feature work; Phase 3 IPC work.

## Verification

- P2-G8: desktop placeholder builds and launches on macOS (scripts/desktop-smoke.sh).
- Architecture lint: `apps/desktop` may not import `crates/ac-db`.