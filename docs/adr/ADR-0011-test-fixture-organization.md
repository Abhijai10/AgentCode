# ADR-0011 — Test and Fixture Organization

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

How are tests and fixtures organized so unit/integration/acceptance/chaos/benchmark
grow without colliding, and fixtures stay deterministic and self-describing?

## Constraints

- Doc 11 §38 (P02-WP04): separate unit / integration / acceptance / chaos /
  benchmark structures from the beginning; executable examples required, empty
  directories do not count.
- Doc 11 H20: canonical fixture IDs (FIX-TS-SMALL, FIX-PY, FIX-RUST, ...); fixture
  version includes ground truth, expected defects, reset procedure, seed, network
  assumptions, cleanup.
- Doc 09 H13: fixture and benchmark registry.
- Doc 09 HC-P02: fixtures small, deterministic, self-describing, resettable, safe to
  repeat.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|--------------|--------------|
| Tests inline per crate only | Simple | No cross-cutting integration/acceptance/chaos separation |
| Separate test repositories | Heavy | Overkill for V1 |
| Hybrid (chosen) | Unit tests in-crate; cross-cutting harnesses in `tests/`; fixtures in `fixtures/` with registry | Matches Doc 09/11 canonical artifacts |

## Evidence

- Doc 11 §38, H20; Doc 09 H13, HC-P02.

## Decision

- **Unit tests**: inside each crate/package (`#[cfg(test)]` / `*.test.ts`).
- **Integration tests**: `crates/*/tests/` (Rust) and `tests/integration/` for
  cross-ecosystem scenarios; harnesses are runnable scripts/`cargo test
  --workspace`.
- **Acceptance**: `tests/acceptance/` — gate-oriented scenarios (e.g., architecture
  boundary violation must fail, fixture ground truth must hold). Executable now.
- **Chaos**: `tests/chaos/` — harness skeleton with one deterministic fault example
  (interrupted migration is a chaos-class failure test), enabled for extended runs.
- **Benchmark**: `benchmarks/` — cargo bench scaffolding + one trivial example that
  builds and runs (P02-WP04 executable example).
- **Fixtures**: `fixtures/FIX-*/` each with `README_AGENTCODE_FIXTURE.md` recording:
  version, ground truth, expected bug, relevant files, expected tests, reset
  procedure, network assumptions, cleanup; plus `fixtures/FIXTURE_REGISTRY.md` and
  `fixtures/fixture_registry.json`.
- Fixture IDs from Doc 11 H20. Phase 2 creates FIX-TS-SMALL plus the minimum
  additional fixtures that prove the harnesses (FIX-RUST, FIX-PY), all small.
- A fixture smoke script proves ground truth holds (and the known bug reproduces).

## Consequences

- Positive: gates map to executable harnesses; fixtures are shareable ground truth
  for later retrieval benchmarks.
- Negative: fixture growth needs discipline; registry keeps it bounded.
- Affected: Doc 09 H13, HC-P02; P2-G7.

## Verification

- P2-G3, P2-G7; `make fixture-smoke` and `make test` pass in CI.