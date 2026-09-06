# WP-P04-WP12 — Routing Evidence

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`, `crates/ac-db`, `migrations/0001_kernel_schema.sql`
- **Acceptance gates:** P4-G10/P4-G11 PASS

## Implemented

- Added `RoutingDecision` with task, candidate list, selected route, rejection reasons, fallback reason, latency, token usage, cost, and timestamp.
- Added `provider_routing_decisions` migration table.
- Added `ControlPlaneDb::save_routing_decision` and `routing_decision` recovery API using sanitized record payloads.

## Validation

- Provider tests verify decision capture.
- DB test verifies routing decision evidence survives reopen without secret leakage.

## Acceptance Decision

ACCEPTED: routing decisions are durable evidence and do not expose secrets.
