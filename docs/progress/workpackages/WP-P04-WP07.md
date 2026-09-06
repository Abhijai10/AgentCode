# WP-P04-WP07 — Model Broker

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`, `crates/ac-agent`
- **Acceptance gates:** P4-G3/DIVERSITY PASS

## Implemented

- Added deterministic broker scoring over role, task type, complexity, context, capabilities, privacy, health, cost, and routing profile.
- Agent provider calls now use brokered `TaskProfile` routing.
- Role-sensitive scoring allows implementation and verification tasks to select different model families.

## Validation

- Broker ranking and diversity-gate tests pass.

## Acceptance Decision

ACCEPTED: Model Broker returns ranked candidates independent of direct provider invocation.
