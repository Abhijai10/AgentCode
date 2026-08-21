# WP-P04-WP11 — Cost/Budget

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`
- **Acceptance gates:** P4-G8/P4-G9 PASS

## Implemented

- Added free/paid connection flags and routing profiles: FREE_ONLY, FREE_FIRST, LOCAL_FIRST, QUALITY_FIRST, PAID_ALLOWED, OFFLINE.
- Added model input/output cost metadata and estimated cost on routing decisions.
- FREE_ONLY rejects paid routes; FREE_FIRST permits paid fallback only after free routes fail.

## Validation

- Tests cover disabled paid policy and paid fallback when explicitly allowed.

## Acceptance Decision

ACCEPTED: paid calls are policy-controlled and observable.
