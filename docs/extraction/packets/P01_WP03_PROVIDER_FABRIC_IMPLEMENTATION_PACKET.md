# P01_WP03_PROVIDER_FABRIC_IMPLEMENTATION_PACKET

Extraction-only packet for future Phase 4 implementation. Do not implement during
Phase 1.

## Interfaces To Build Later

```text
ProviderCatalogService
  register_provider(definition, provenance) -> ProviderCatalogRecord
  discover_models(provider_connection_ref) -> DiscoveryRunResult
  get_normalized_model(model_ref) -> ModelCapabilityRecord

ProviderAdapter
  prepare_request(NormalizedInferenceRequest) -> ProviderWireRequest
  stream(request, cancel_token) -> AsyncIterator[ProviderStreamEvent]
  classify_error(error, response?) -> ProviderFailureClass

RouteAttemptRecorder
  start_attempt(assignment_plan_id, provider_connection_id, model_id)
  record_stream_event(...)
  finish_attempt(status, usage, failure_class?, partial_output_ref?)
```

## Required Control Flow

```text
Kernel/Agent Runtime asks Model Broker for route
-> Model Broker applies hard filters
-> Provider Fabric selects provider connection and adapter
-> Context/Runtime supplies normalized request
-> adapter streams provider wire response
-> Provider Fabric normalizes deltas/tool calls/usage/errors
-> Agent Runtime receives only validated normalized events
-> RouteAttemptRecorder persists attempt evidence
```

## Required State Ownership

- Kernel owns mission/task/lease/completion.
- Model Broker owns model selection policy and assignment plan.
- Provider Fabric owns provider catalog, connection refs, health, quotas, discovery,
  request/response/error normalization.
- Agent Runtime owns model-turn continuity and cancellation propagation.
- No raw credential leaves the authorized adapter boundary.

## Failure Requirements

- 429, timeout, 5xx, stream abort, malformed response, auth, permission, context
  overflow, unsupported capability, and client cancellation must be distinguishable.
- Partial stream is `FAILED_PARTIAL` unless a provider-format terminal marker is
  validated.
- Retry/fallback must be budget-bounded and record retry-after/reserve decisions.
- Multiple keys under one quota domain must cool down the quota domain, not just the
  key, when evidence proves shared exhaustion.

## Phase 4 Tests To Implement

- Mock 429 with Retry-After and provider-specific restatement.
- Timeout and stream-idle abort with partial output evidence.
- 5xx retry then fallback with cost reserve enforcement.
- Client cancellation does not mark provider unhealthy.
- Paid-disabled policy blocks paid fallback.
- Stale discovery snapshot is explicit degraded state.
- Local model registry creates normal capability record.
- Production caller path: Agent Runtime -> Model Broker -> Provider Fabric.

## Invalidation Keys

- provider adapter version
- provider catalog version
- credential_ref version / revoked state
- quota domain policy version
- model discovery fetched_at + TTL
- routing policy version
- token accounting schema version

