# ADR-0009 — Structured Logging Foundation

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

Which structured logging stack and conventions do all AgentCode services share?

## Constraints

- Doc 09 HC-P02 (Structured error/log foundations): component namespace, error code,
  severity, retryability, correlation ID, mission/task IDs where applicable,
  redaction.
- Doc 11 §40/H22: structured logger usable by all core services; redaction testable
  even before Secret Broker exists.
- ADR-0001: Rust core + TS services.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| log crate only | Minimal | No structured fields/correlation support |
| tracing (chosen, Rust) | De-facto structured ecosystem; layers for redaction; subscriber-based output control | — |
| custom logger in TS (chosen: pino) | Small, fast, JSON-native | pino is MIT, single dependency; JSON structured output aligned with Rust JSON layer |

## Evidence

- Doc 09 HC-P02; Doc 11 H22–H23; extraction P01-WP04: donor runtimes use
  structured/tracing-style logging (codex-rs tracing; opencode pino-style) — see
  docs/extraction/04-agent-runtime/.

## Decision

- **Rust (`crates/ac-logging`)**: `tracing` + `tracing-subscriber` with a JSON
  formatter for daemon logs and a human formatter for dev; a redaction layer with
  secret/canary pattern support (regex + exact-value redaction registry);
  correlation ID span field; stable error codes defined in `crates/ac-common` per
  Doc 11 H23 (error taxonomy: `COMPONENT-CODE` scheme with retryability flag).
- **TypeScript**: `pino` wrapper with identical field conventions (`component`,
  `severity`, `error_code`, `correlation_id`, `mission_id`, `task_id`, redacted
  fields) in a small shared module under `services/provider_gateway` (single TS
  logging authority for services; desktop renderer logs are dev-only).
- Log records never include raw secrets; redaction is applied at emission.
- A canary-secret test proves redaction works before any real secrets exist
  (P02-WP06 test).

## Consequences

- Positive: uniform observability across processes; redaction proven early.
- Negative: two implementations (Rust/TS) sharing a convention contract; documented in
  docs/development/logging.md.
- Affected: Doc 09 HC-P02; all phases' observability work.

## Verification

- P2-G6; unit tests in `crates/ac-logging` (redaction, correlation propagation,
  error-code format) and TS logging smoke test.