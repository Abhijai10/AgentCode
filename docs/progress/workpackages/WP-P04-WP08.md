# WP-P04-WP08 — Health Scoring

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** MEDIUM | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`
- **Acceptance gates:** P4-G4/P4-G6/P4-G7 PASS

## Implemented

- Added `HealthObservation` for success/failure/latency per connection.
- Provider execution records health observations after each route attempt.
- Broker excludes open circuits and disabled connections.

## Validation

- Rate-limit and timeout fallback tests verify health is updated without caller restart.

## Acceptance Decision

ACCEPTED: health state affects routing through the existing registry.
