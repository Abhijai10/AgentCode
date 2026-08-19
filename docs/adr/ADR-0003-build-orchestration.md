# ADR-0003 — Build/Task Orchestration

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

Which tool orchestrates root-level developer commands (`format`, `lint`, `typecheck`,
`test`, `validate`, `ci`) across the Rust and TypeScript ecosystems?

## Constraints

- Doc 11 §36 (P02-WP02): one documented command performing format check, lint,
  compile/typecheck, unit tests. "The exact task runner is implementation-specific."
- Doc 09 HC-P02: reproducible root commands; a single `validate` command runs the fast
  baseline; committed lockfiles (Cargo.lock, pnpm-lock.yaml).
- Environment: macOS host with make 3.81, cargo 1.96, node 26, pnpm 11.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| just | Modern task runner | Not installed; adding a tool requires bootstrap step (violates minimal clean bootstrap) |
| npm scripts only | `npm run validate` | Cannot naturally orchestrate cargo; awkward |
| cargo-make | Cargo-centric | Poor TS integration |
| Makefile (chosen) | GNU make present on all target platforms | Zero-install, deterministic, both ecosystems callable |

## Evidence

- Host tool audit (make, cargo, node, pnpm all present); Doc 11 §36.

## Decision

- Root `Makefile` is the single documented entry point. Targets:
  `format`, `format-check`, `lint`, `typecheck`, `test`, `migrate-check`,
  `arch-check`, `secret-scan`, `fixture-smoke`, `validate` (fast baseline),
  `build`, `desktop-build`, `desktop-smoke`, `daemon-build`, `clean`, `bootstrap`.
- `make validate` = format-check + lint + typecheck + unit tests + migrate-check +
  arch-check + secret-scan + fixture-smoke (see ADR-0013 for CI mirroring).
- Each ecosystem keeps its own toolchain (cargo for Rust, pnpm for TS) and lockfiles:
  `Cargo.lock`, `pnpm-lock.yaml` committed.
- `make bootstrap` documents/verifies the exact setup from a clean checkout (no
  dependency on the reference-library directory).

## Consequences

- Positive: zero new tool installs; one canonical command surface.
- Negative: make is less expressive than just; acceptable for a fixed target set.
- Affected: Doc 09 HC-P02 build orchestration slice; every Phase 2+ WP.

## Verification

- P2-G1 (clean checkout builds) via `make bootstrap && make validate`.
- CI uses the same targets (ADR-0013).