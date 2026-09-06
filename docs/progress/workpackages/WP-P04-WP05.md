# WP-P04-WP05 — Provider Adapters

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`
- **Acceptance gates:** P4-G1/P4-G5/P4-G7 PASS

## Implemented

- Kept mock/scripted provider for CI and failure injection.
- Added configuration-based HTTP adapter with endpoint, model, timeout, cancellation checks, status mapping, and environment credential lookup.
- Added generic configured adapter path.

## Validation

- Tests cover retry/cancel lifecycle, mocked HTTP response normalization, and failure classification.

## Acceptance Decision

ACCEPTED: provider adapters are real replaceable execution paths with deterministic test harnesses.
