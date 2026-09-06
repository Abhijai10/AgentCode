# ADOPTION_01_PROVIDER_FABRIC

Phase 1 WP03 adoption decision.

## Decision

AgentCode will build an owned Provider Fabric under Doc 01 using donor mechanisms
only as patterns. It will not embed OmniRoute/OpenCode/LiteLLM provider config as
durable truth.

## TAKE

- Typed provider/model records with capability flags and token/context limits from
  OmniRoute `@omniroute/opencode-provider/src/index.ts`.
- Stale discovery and warm-cache semantics from OmniRoute plugin warm-startup tests.
- Stream terminal-marker validation and client-disconnect distinction from
  `open-sse/utils/streamHandler.ts`.
- Pre-classification provider status restatement from
  `open-sse/config/upstreamStatusRestatement.ts`.
- Cost/response persistence on parse failures from mini-SWE-agent
  `LitellmModel.query()`.

## ADAPT

- OmniRoute routing/scoring into AgentCode’s Doc 01 sequence: hard filters before
  score, budget reserve before paid fallback, health evidence not inferred from a
  static catalog.
- Gemini CLI per-turn model unavailability into transient route-attempt state.
- LiteLLM/local model support into an adapter behind the owned provider fabric.

## WRAP

- LiteLLM, OpenAI-compatible gateways, and local model servers behind
  `ProviderAdapter` interfaces. They may execute requests but may not own
  provider identity, credentials, policy, or mission state.

## IGNORE / REJECT

- Reject raw API keys in durable provider config.
- Ignore OpenCode-specific generated config documents except as adapter examples.
- Reject routing decisions that treat fallback/retry as task completion.

## Contract Required For Phase 4

```text
ProviderCatalogRecord
ProviderConnectionRecord
ModelDiscoveryRecord
ModelCapabilityRecord
ProviderAttempt
ProviderFailureClass
ProviderStreamEvent
UsageAccountingRecord
AssignmentPlanEvidence
```

Each record must include donor-independent provenance, invalidation keys, credential
reference only, and failure/degraded states.

