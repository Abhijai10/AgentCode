# WP-P04-WP01 — Provider Lifecycle Adapter Path

- **Phase:** P04
- **Status:** IMPLEMENTING
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 62439da380875cc06664b26f5cef9dcff17822c8
- **Accepted commit:** not accepted yet
- **Owner modules:** `crates/ac-provider`, `crates/ac-agent`
- **Architecture refs:** P01 WP03 implementation packet; ADR provider architecture
- **Acceptance gates:** provider lifecycle partial; full P04 gates not complete

## Implemented In This Batch

- Added ProviderRegistry-owned adapter registration and model-scoped streaming execution.
- Added retry handling for retryable provider failures, stream-finished validation, cancellation handling, and route-attempt failure recording.
- Kept credentials as references (`credential_ref`) and did not add secrets.
- Connected `ac-agent` to `ProviderRegistry::stream_with_retry` so runtime/agent code remains provider-vendor agnostic.
- Added deterministic mock-provider tests for streaming retry lifecycle and cancellation.

## Remaining Before Acceptance

- Production OpenAI/Anthropic/Gemini/local adapters.
- Structured planner response schema and validation.
- Token accounting persistence and rate-limit policy integration.

