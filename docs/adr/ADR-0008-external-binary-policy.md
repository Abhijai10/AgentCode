# ADR-0008 — External Binary/Tool Management Policy

- **Status:** ACCEPTED
- **Decision class:** POLICY
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

How does AgentCode acquire and manage external binaries/tools (ripgrep, tree-sitter
CLI, ctags, scanners, LSP servers, browser drivers, security scanners)?

## Constraints

- Doc 07 §109–114: per-tool choice of BUNDLED / MANAGED DOWNLOAD / SYSTEM DEPENDENCY /
  CONTAINERIZED / REMOTE API; explicit choice required.
- Doc 07 §108: feature detection registers capabilities dynamically; optional systems
  must not be required for a small local bug fix (Doc 07 §107).
- Doc 11 H13: external tool provenance; H16: macOS sandbox reality.
- Run prompt: no `curl | sh` as a production installation strategy; no global
  arbitrary installs as architecture.
- Doc 09 HC-P02: clean checkout builds without the donor-reference directory.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| Bundle everything | Simplest | License/size/bloat; conflicts with optionality (Doc 07 §107) |
| `curl | sh` / arbitrary installers | Common | Explicitly rejected; not auditable, no pinning, supply-chain risk |
| Per-tool registry with pinned releases + integrity checks (chosen) | Each tool records install strategy, pinned version/SHA256, license, capability key | Matches Doc 07 §109–114 and §114 version registry |

## Decision

- A **tool registry** (`docs/policy/TOOL_REGISTRY.md`) records every external binary:
  - strategy class: `BUNDLED` | `MANAGED_DOWNLOAD` | `SYSTEM_DEPENDENCY` |
    `CONTAINERIZED` | `REMOTE_API`
  - pinned version + integrity (SHA-256) for MANAGED_DOWNLOAD
  - license + provenance (per docs/legal/)
  - capability key used by feature detection (Doc 07 §108)
  - runtime/bundle cost, install/postinstall behavior (per P00-WP03 admission fields)
- MANAGED_DOWNLOAD is the default for AgentCode-managed binaries; downloads go through
  a versioned manifest with integrity verification — never `curl | sh`.
- SYSTEM_DEPENDENCY only for tools where the OS/platform guarantees are acceptable
  (documented per tool, e.g., macOS system `git` is assumed present; build tooling
  does not depend on it beyond what the OS provides).
- No external binary may be required for Phase 2 build/test; the reference-library
  directory is never consulted at build or runtime.
- Each tool's install policy is recorded when its Phase adopts it; nothing is adopted
  ad hoc.

## Consequences

- Positive: auditable supply chain; deterministic CI; optional tools stay optional.
- Negative: initial setup effort per tool; integrity manifest maintenance.
- Affected: Doc 07 §109–114; Phase 5 (tools), Phase 7 (git), Phase 8/9 (intelligence),
  security phases (13+).

## Verification

- Phase 2: zero external tool requirement (P2-G1 clean build).
- Tool admission checkpoints run per dependency admission (P00-WP03 process).