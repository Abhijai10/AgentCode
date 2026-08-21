# WP-P04-WP01 — Provider Lifecycle Adapter Path

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 62439da380875cc06664b26f5cef9dcff17822c8
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`, `crates/ac-agent`
- **Architecture refs:** P01 WP03 implementation packet; ADR provider architecture
- **Acceptance gates:** provider lifecycle and HTTP transport gates PASS for Phase 3 autonomous coding scope

## Implemented In This Batch

- Added ProviderRegistry-owned adapter registration and model-scoped streaming execution.
- Added retry handling for retryable provider failures, stream-finished validation, cancellation handling, and route-attempt failure recording.
- Kept credentials as references (`credential_ref`) and did not add secrets.
- Connected `ac-agent` to `ProviderRegistry::stream_with_retry` so runtime/agent code remains provider-vendor agnostic.
- Added deterministic mock-provider tests for streaming retry lifecycle and cancellation.
- Added `ConfiguredProviderAdapter` as an optional configuration-based real-provider adapter path.
- Verified adapter configuration uses endpoint/credential references and does not embed secrets.
- Added optional `HttpProviderAdapter` behind the unchanged `ProviderAdapter` trait.
- Added endpoint/model/timeout configuration, retry compatibility through `ProviderRegistry::stream_with_retry`, cancellation checks, HTTP status failure normalization, and credential lookup by environment reference.
- Added mocked HTTP response normalization tests without hardcoded secrets.

## Validation

- `cargo fmt` PASS.
- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.

## Known Limitations

- Token accounting persistence and rate-limit policy integration.
- HTTP adapter is a minimal std-based POST transport for configurable HTTP endpoints; vendor-specific schemas, TLS, and streaming chunk decoders remain later provider hardening.

## Acceptance Decision

ACCEPTED for Phase 3: real provider transport exists without changing the provider trait, mock provider remains for CI, credentials stay outside code, and tests cover routing/cancellation/retry plus mocked HTTP normalization.
