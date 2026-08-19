# ADR-0006 — OmniRoute Placement in Workspace

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

Where does the OmniRoute-based provider fabric live in the AgentCode workspace, and
what is the build relationship?

## Constraints

- Doc 07 §20–24: OmniRoute is the provider-fabric reference; keep its routing model,
  modify for AgentCode needs; must NOT own Kernel/context/verification authority.
- Doc 07 §103: PROVIDER FABRIC = custom OmniRoute fork / TypeScript-compatible layer.
- Doc 09 HC-P04 (Phase 4): Model Broker & OmniRoute Provider Fabric.
- Doc 07 §98: do not vendor entire repositories; §97: no git submodules for the
  reference library; §96: pin source versions.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| Git submodule of upstream OmniRoute | Direct link | Forbidden by Doc 07 §97; creates runtime dependence on reference library |
| Vendor whole OmniRoute tree into repo | Copy full source | Forbidden by Doc 07 §98 |
| npm dependency on published OmniRoute package | Upstream package | OmniRoute is a Next.js app/framework, not a publishable routing lib for AgentCode's needs; we need a *compatible layer*, not the app |
| First-party `services/provider_gateway` TS package implementing the adopted routing model (chosen) | Clean-room/adapted layer with attribution | Matches Doc 07 §20–24 and §103 |

## Evidence

- Doc 07 §20–24, §97–98, §103; extraction P01-WP03 (OmniRoute package layout,
  routing core entrypoints, licensing) — see docs/extraction/03-provider-fabric/.

## Decision

- The provider fabric is the first-party TypeScript package
  `services/provider_gateway` (pnpm workspace), implementing the AgentCode-adopted
  subset of the OmniRoute routing model (top-K/fallback, capability mapping) per the
  P01-WP03 extraction record.
- Upstream OmniRoute source remains a research input in
  `/Volumes/T7 Shield/GitHub-Repos-dependency/OmniRoute` at a pinned SHA; it is never
  a runtime dependency and never vendored wholesale.
- Phase 2 creates only the package skeleton (builds, health-check truthfulness); the
  routing implementation arrives in Phase 4 (HC-P04).
- Attribution and license obligations per docs/legal/ and Doc 07 §93.

## Consequences

- Positive: no fork-maintenance burden in V1; clean license boundary; Phase 4 can
  iterate against extraction records.
- Negative: re-implementing routing mechanics takes Phase 4 effort (planned).
- Affected: Doc 09 HC-P04, Doc 07 §20–24; Phase 4 work.

## Verification

- Phase 2: `services/provider_gateway` builds, lints, tests, and its smoke test
  reports truthful skeleton state (no fake routing results).
- Phase 4 gates (P4-*) prove routing behavior against FIX-PROVIDER-MOCK.