# WP-P04-WP03 — Credential References

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`, `crates/ac-config`, `crates/ac-db`
- **Acceptance gates:** P4-G11 PASS

## Implemented

- Provider definitions and connections store credential references only.
- `HttpProviderAdapter` resolves optional credential values from environment at execution boundary.
- Routing candidates and persisted routing decisions omit credential refs and raw secret values.

## Validation

- Tests verify configured adapters and routing decisions do not expose secret material.

## Acceptance Decision

ACCEPTED: credentials remain references and are excluded from routing logs/evidence.
