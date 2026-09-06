# WP-P04-WP09 — Circuit Breakers

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`
- **Acceptance gates:** P4-G6/P4-G7 PASS

## Implemented

- Added per-connection `CircuitBreaker`.
- Rate limits, timeouts, and provider-unavailable failures open cooldown.
- Successful route execution resets breaker state.

## Validation

- Tests prove 429 opens cooldown and routes fallback to another provider.

## Acceptance Decision

ACCEPTED: unhealthy providers are cooled down instead of repeatedly hammered.
