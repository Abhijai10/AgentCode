# WP-P04-WP10 — Fallback

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`, `crates/ac-agent`
- **Acceptance gates:** P4-G6/P4-G7/P4-G9/FAILURE TEST PASS

## Implemented

- `request_model` iterates ranked candidates, normalizes failure, updates health/circuit state, preserves task metadata, and selects next route.
- Caller receives one successful inference result when fallback succeeds.
- Agent does not restart its mission to survive provider failure.

## Validation

- Tests cover rate-limit fallback, timeout recovery, and paid fallback when explicitly allowed.

## Acceptance Decision

ACCEPTED: fallback is inside Provider Fabric and invisible to mission lifecycle callers.
