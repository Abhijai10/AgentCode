# ADR-0013 — CI Strategy

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

What does CI run, where, and how does it stay deterministic?

## Constraints

- Doc 11 H19 (CI/acceptance matrix): Fast PR tier = format, lint, compile/typecheck,
  unit tests, migration sanity, architecture-boundary checks, secret scan, small
  deterministic fixtures, gate-catalog validation. "No live free provider is allowed
  to make ordinary PR CI nondeterministic."
- Doc 09 HC-P02 (CI jobs): format, lint, compile/typecheck, unit tests, migration
  validation, architecture-boundary lint, secret scan, fixture smoke.
- Doc 11 §37 (P02-WP03): CI on every main development PR/branch; avoid
  security/benchmark-heavy CI until later.
- P2-G4: CI runs basic validation.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|--------------|--------------|
| Self-hosted everything | Full control | Not available for open CI today |
| GitHub Actions + macOS + ubuntu (chosen) | Hosted, deterministic | — |
| CircleCI | Equivalent | GitHub Actions keeps CI config inside repo (Doc 09 canonical artifact "CI workflows") |

## Evidence

- Doc 11 H19 Fast PR tier; Doc 09 HC-P02; P2-G4.

## Decision

- GitHub Actions workflow `.github/workflows/ci.yml` with two jobs:
  1. **core-validate** (ubuntu-latest): `make validate` (format-check, lint,
     typecheck, unit tests, migrate-check, arch-check, secret-scan, fixture-smoke)
     plus dependency-check (manifest/lock consistency, allowlist, forbidden
     reference-path scan).
  2. **desktop-build** (macos-latest): build Tauri shell (cargo check/test in
     apps/desktop + `pnpm build` + frontend tests) — deterministic compile-level
     verification; interactive launch smoke stays a local/scripted check
     (scripts/desktop-smoke.sh) because hosted runners may lack a window server.
- No live model/provider calls in PR CI; no network-dependent tests; no chaos/bench
  in Fast PR.
- PR CI must pass for merge to `main` (branch policy, P00-WP07).
- The CI workflow itself is generated/verified against `make validate` targets so
  local and remote behavior match.

## Consequences

- Positive: deterministic Fast-PR validation; desktop compile verified on macOS.
- Negative: macOS runner minutes for desktop job; bounded.
- Affected: Doc 11 H19; later phases add Integration/Nightly tiers.

## Verification

- P2-G4; CI run against this batch's branch; gate-catalog validation script
  (scripts/validate-gate-catalog.mjs) checks P0/P1/P2 gate registry consistency.