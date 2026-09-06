# WP-P04-WP02 — Provider Interface

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`, `crates/ac-agent`
- **Acceptance gates:** P4-G1/P4-G2/P4-G3/P4-G4 PASS

## Implemented

- Kept `ProviderAdapter` unchanged and replaceable.
- Added `ProviderConnection`, `ModelIdentity`, `ModelRoute`, `TaskProfile`, `RoutingProfile`, and `RouteExecution`.
- Switched agent provider calls to `ProviderRegistry::request_model`, so routing is on the production autonomous path.

## Validation

- Provider routing tests cover remote, paid, free, local, fallback, and role-diversity paths.
- Agent autonomous benchmark still passes through brokered provider selection.

## Acceptance Decision

ACCEPTED: callers issue one model request and do not depend on the serving provider.
