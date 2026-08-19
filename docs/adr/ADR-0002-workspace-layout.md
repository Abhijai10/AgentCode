# ADR-0002 — Workspace Layout

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

What is the monorepo shape for AgentCode V1?

## Constraints

- Doc 09 HC-P02 recommends: `apps/ core/ services/ adapters/ skills/ tests/ fixtures/
  benchmarks/ docs/` — "exact path may differ by language ecosystem".
- Doc 11 H-P02 recommended ownership: workspace manifests, `core/`, `services/`,
  `apps/desktop/`, `tests/`, `fixtures/`, `benchmarks/`, `migrations/`.
- Architecture linting must prevent: desktop UI → direct DB writes; provider gateway →
  Kernel task state; security adapter → separate mission scheduler; Kernel → React
  state.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| Single flat repo, no package separation | Fastest start | No dependency direction control; guarantees circular ownership later |
| npm-only monorepo | One toolchain | Kernel must be Rust (ADR-0001) |
| Cargo-only + Tauri sidecar TS | Hide TS | Provider fabric is a first-class TS layer (Doc 07 §103) |
| Hybrid workspace (chosen) | Cargo workspace + pnpm workspace + Makefile orchestration | Two ecosystems with explicit boundaries and one documented command set |

## Evidence

- Doc 09 HC-P02 "Workspace/package layout"; Doc 11 H-P02 "Recommended Module
  Ownership"; Doc 11 §35 "Dependency Direction Test".

## Decision

Top-level layout:

```text
apps/              desktop shell (Tauri 2 + React/TS/Vite) — standalone cargo pkg + pnpm pkg
services/          provider_gateway (TypeScript, OmniRoute-compatible layer)
crates/            Rust core crates (ac-common, ac-logging, ac-config, ac-db, ac-daemon, ...)
migrations/        SQLite migration SQL (canonical, owned by ac-db)
fixtures/          deterministic fixture repositories (FIX-*)
tests/             unit/ integration/ acceptance/ chaos/ benchmark (harnesses + examples)
benchmarks/        benchmark harnesses (Phase 2: structure only)
scripts/           repository tooling (architecture check, secret scan, fixture smoke)
docs/              governance, registry, ADR, legal, reference, extraction, progress
ci/                CI workflow config (GitHub Actions)
```

Dependency direction (enforced by `scripts/check-architecture.mjs`, see ADR-0013):

1. `apps/desktop` and `services/provider_gateway` may not import any Rust crate or
   `crates/ac-db` state types; they may not open/write the mission database.
2. `crates/*` may not import from `apps/`, `services/`, or reference donor repos.
3. `crates/ac-daemon` is the only place allowed to own the daemon lifecycle.
4. Kernel-task-state ownership lives in `crates/` (Kernel) only.
5. No crate may depend on `tests/` or `fixtures/`.

## Consequences

- Positive: ownership boundaries visible from directory names; architecture lint is
  cheap (grep/import rule table); clean phase growth path.
- Negative: two build systems; documented cross-ecosystem contracts needed (packets).
- Affected: Doc 09 HC-P02 canonical artifacts; all Phase 2 WPs.

## Verification

- `make validate` includes architecture-boundary check (P02-WP01, P2-G1..G4).
- Failure test: an intentionally wrong import fails the check (tests/acceptance).