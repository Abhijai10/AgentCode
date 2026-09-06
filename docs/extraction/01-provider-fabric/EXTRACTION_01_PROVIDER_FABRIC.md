# EXTRACTION_01_PROVIDER_FABRIC

Phase 1 WP03 extraction-only report. Donor repositories are evidence, not
AgentCode architecture authority.

## Donors Inspected

| Donor | Catalog ID | Pinned commit | License | Files inspected |
|---|---:|---|---|---|
| OmniRoute | REF-032 | `df905914154b0d082c2bf34198ffad9656df9ec7` | MIT | `@omniroute/opencode-provider/src/index.ts`; `@omniroute/opencode-provider/tests/index.test.ts`; `@omniroute/opencode-plugin/src/index.ts`; `@omniroute/opencode-plugin/tests/warm-startup.test.ts`; `open-sse/utils/streamHandler.ts`; `open-sse/config/upstreamStatusRestatement.ts`; `open-sse/mcp-server/tools/pickFastestModel.ts`; `open-sse/services/xaiOauthQuotaFetcher.ts`; `open-sse/services/webSessionPoolHealth.ts` |
| Codex | REF-013 | `77e688960196dbc82bbeb00c844d2555a61925aa` | Apache-2.0 | `codex-rs/rollout-trace/src/reducer/inference.rs`; `codex-rs/rollout-trace/src/raw_event.rs`; `codex-rs/app-server/tests/suite/v2/model_list.rs`; `codex-rs/app-server/tests/suite/v2/model_provider_capabilities_read.rs` |
| Aider | REF-002 | `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` | Apache-2.0 | `aider/models.py`; `aider/openrouter.py`; `aider/main.py` |
| OpenCode | REF-034 | `9b0dd36cda0b9accb429a7f9f9ad9b054a27d04a` | MIT | `packages/llm/src/provider.ts`; `packages/llm/src/providers/openai.ts`; `packages/llm/src/providers/openrouter.ts`; `packages/llm/src/schema/options.ts`; `packages/sdk/openapi.json` provider/config endpoints |
| mini-SWE-agent | REF-028 | `25941c89cfbc91eb40b3f8756348c91d9977d57e` | MIT | `src/minisweagent/models/litellm_model.py`; `src/minisweagent/models/openrouter_model.py`; `src/minisweagent/models/utils/retry.py`; `tests/models/*` |
| Gemini CLI | REF-020 | `571851b1077a51cef757146ce13f9da887326bec` | Apache-2.0 | `packages/core/src/availability/modelAvailabilityService.ts`; `packages/core/src/availability/policyHelpers.ts`; `packages/core/src/utils/retry.ts`; `packages/core/src/utils/quotaErrorDetection.ts`; related tests |
| OpenHands / software-agent-sdk | REF-036 / REF-052 | `b25f9b3969f924f37440fee908ff35309ec6eea2` / `98338ff37aea6627777b9978963ab727f51e4f40` | MIT | `openhands-sdk/openhands/sdk/llm/llm.py`; `openhands-sdk/openhands/sdk/settings/model.py`; `tests/agent_server/test_llm_router.py`; `tests/agent_server/test_switch_llm_survives_reload.py` |

## Source-Level Findings

### Provider Registration and Model Catalog

OmniRoute’s OpenCode provider helper is the cleanest registration example. In
`@omniroute/opencode-provider/src/index.ts`, `createOmniRouteProvider()` validates
`baseURL` and `apiKey` (`requireNonEmpty`, `normalizeBaseURL`, lines 207-245),
builds a provider object keyed by `OMNIROUTE_PROVIDER_KEY`, and emits model entries
with capability flags and `limit.context` metadata (interfaces around lines
69-194). Tests in `@omniroute/opencode-provider/tests/index.test.ts` verify default
models, custom model lists, duplicate trimming, context-length propagation, live
`/v1/models` fetch parsing, and generated OpenCode config fields.

Control flow:

```text
caller options
-> normalizeBaseURL / requireNonEmpty
-> choose caller models or OMNIROUTE_DEFAULT_OPENCODE_MODELS
-> merge default capabilities + caller overrides
-> create OpenCodeProviderEntry
-> optional config merge/build helpers
```

AgentCode decision: **ADAPT** the typed catalog shape and capability normalization,
but do not copy the OpenCode-specific config writer. AgentCode’s provider catalog
belongs to OmniRoute/provider fabric, while Model Broker consumes normalized
capabilities under Doc 01.

OmniRoute’s plugin has a much broader live discovery/cache path. `@omniroute/opencode-plugin/src/index.ts`
uses warm startup, disk snapshots, in-flight request sharing, endpoint fallback, and
`Promise.allSettled` around models/combos/enrichment (see comments and logic around
lines 4718 and 5355-5521 from inspection). Tests in
`@omniroute/opencode-plugin/tests/warm-startup.test.ts` cover failed refresh keeping
warm snapshots, combos rejecting to models-only catalogs, concurrent invocation
dedupe, disk-cache disabling, and usable-only filtering.

AgentCode decision: **TAKE** the invalidation model: discovery records need source,
fetched_at, credential scope hash, cache TTL, fetch status, and stale reason. **ADAPT**
the warm snapshot behavior as `DISCOVERY_STALE_BUT_USABLE`; never let stale discovery
mean authenticated availability.

### Credentials and Configuration Ownership

OmniRoute’s OpenCode helper stores `apiKey` directly under provider options
(`OpenCodeProviderEntry.options.apiKey`, lines 187-191). The plugin tests also assert
generated env fields such as `OMNIROUTE_API_KEY`. This is appropriate for a config
adapter but conflicts with AgentCode’s Doc 01/Doc 04 rule that credentials are secret
references resolved only at execution boundaries.

AgentCode decision: **WRAP/REJECT raw-secret persistence**. Accept only:

```text
credential_ref
provider_connection_id
quota_domain_id
secret_store_provider
redaction_policy_ref
```

The adapter may receive raw credentials inside an execution envelope, but the model,
Kernel, durable task state, reports, and UI must not.

### Request and Response Normalization

OmniRoute’s `open-sse/utils/streamHandler.ts` normalizes streaming errors into
client-format-specific status envelopes. `getStreamErrorStatusKind()` maps HTTP
status codes to rate limit/auth/permission/client/server classes (lines 65-106);
`isClientDisconnectError()` treats client aborts as non-provider failures (lines
149-165); `getErrorStatusCode()` maps timeout-like names to 504 and unknown upstream
errors to 502 (lines 173-188). `hasClientTerminalSseMarker()` validates terminal SSE
markers for OpenAI, Responses, Claude, and finish_reason streams (lines 197-224).

AgentCode decision: **TAKE** the distinction between provider failure and caller
disconnect. **ADAPT** status normalization into a first-class
`ProviderFailureClass` enum:

```text
RATE_LIMIT | AUTHENTICATION | PERMISSION | BAD_REQUEST | PROVIDER_5XX |
TIMEOUT | STREAM_ABORT | CLIENT_CANCELLED | QUOTA_EXHAUSTED |
CONTEXT_LIMIT | UNSUPPORTED_CAPABILITY | MALFORMED_RESPONSE
```

Response normalization must output structured chunks with terminal markers, usage,
tool-call deltas, provider_request_id, and partial-output status. Partial stream
without terminal evidence is not success.

### Quota, Rate Limits, Retries, Timeout Handling

OmniRoute’s `open-sse/config/upstreamStatusRestatement.ts` is important because it
corrects provider-specific misclassification before fallback classification. The
registry maps provider id to ordered rules (`statusRestatementRegistry`, lines
86-89); `applyStatusRestatement()` keeps pass-through defaults, matches provider,
status and body markers, preserves Retry-After when available, and emits `ruleId`
plus original status (lines 101-129). The comment documents that this runs from the
`providerFailure` block before fallback classification (lines 10-21).

AgentCode decision: **TAKE** the pre-classification restatement hook, but store rule
application as audit evidence. This prevents retry storms caused by permanent errors
and prevents permanent aborts caused by temporary quota errors.

mini-SWE-agent’s `LitellmModel.query()` retries through `retry()` while aborting on
unsupported params, not found, permission denied, context window exceeded,
authentication, and keyboard interrupt (`litellm_model.py` lines 49-84). It also
persists response and cost on parse failures (`FormatError`, lines 87-98) and raises
if cost cannot be calculated unless configured to ignore errors (lines 108-126).

AgentCode decision: **ADAPT** its billed-failure accounting and parse-failure
evidence. **IGNORE** direct LiteLLM-as-fabric for V1 because AgentCode requires
owned provider identity, quota domains, trust policy, and normalized failure classes
before routing.

Gemini CLI’s availability service (`modelAvailabilityService.ts` and
`policyHelpers.ts`) uses per-turn retry markers and reset semantics. This is useful
for preventing the same bad model from being chosen repeatedly inside one turn.

AgentCode decision: **ADAPT** as transient `route_attempt_state`, not durable task
truth. Retries/fallbacks persist evidence but cannot mutate mission completion.

### Routing and Scoring

OmniRoute’s `open-sse/mcp-server/tools/pickFastestModel.ts` aggregates combos,
health, quota, and analytics via `Promise.allSettled`, then scores candidates by
latency/success/health/quota. It includes an `includeUnhealthy` option and tracks
whether a candidate has supporting signal. This proves a useful pattern: score only
after hard filters and missing-signal handling.

AgentCode decision: **ADAPT** the scoring inputs, but AgentCode routing must follow
Doc 01:

```text
hard filters: capability, trust, credentials, context, budget, availability
then scoring: latency, quality evidence, cost, quota, independence, local policy
then assignment plan with fallback graph and budget reserve
```

### Local Model Support

mini-SWE-agent supports local models indirectly through LiteLLM model names and an
optional `litellm_model_registry` (`litellm_model.py` lines 27-37 and 59-63). This
shows how low-friction local support can be exposed through config.

Aider also supports local models through provider/model settings. In
`aider/models.py`, `send_completion()` adds a dynamic `num_ctx` for Ollama models
when the caller did not provide one (lines 1012-1014), sets a default request timeout
(lines 1019-1021), forwards extra params (lines 1010-1011), and injects Copilot
headers only when `GITHUB_COPILOT_TOKEN` is present (lines 1026-1035). Its
`simple_send_with_retries()` doubles retry delay for retryable LiteLLM exceptions
until a cap (`models.py` lines 1039-1075).

AgentCode decision: **ADAPT** the idea of operator-supplied model registries, but the
provider fabric must normalize local models into the same capability/health/quota
contract and avoid hardcoded durable model IDs in Kernel state.

OpenCode’s `packages/llm/src/provider.ts` exposes structural provider helpers, while
`packages/llm/src/schema/options.ts` normalizes `Model` records with provider IDs,
defaults, compatibility, provider options, and tool-schema compatibility. Its
OpenAPI exposes `provider.list`, `provider.auth`, and OAuth authorization/callback
endpoints in `packages/sdk/openapi.json`. This is a useful external API shape but it
does not own AgentCode routing truth.

AgentCode decision: **ADAPT** provider/auth listing endpoints as UI/API precedent;
**IGNORE** OpenCode-specific session/provider persistence semantics for AgentCode
Kernel state.

### Cancellation

OmniRoute distinguishes client disconnect from upstream failure
(`isClientDisconnectError()`, lines 149-165). Gemini CLI passes an `AbortSignal` into
`sendMessageStream()` and scheduler execution (`legacy-agent-session.ts` lines
196-199 and 250-253). Codex rollout-trace records cancelled/failed inference attempts
and closes streams still live when a turn ends (`codex-rs/rollout-trace/src/reducer/inference.rs`,
comments and reducers around running/terminal transitions).

AgentCode decision: **TAKE** explicit cancellation propagation and **ADAPT** it into
`ProviderAttempt.cancel_requested_at`, `cancel_observed_at`, and terminal
`CLIENT_CANCELLED` vs `PROVIDER_ABORTED` statuses.

## Tests Inspected

- OmniRoute `@omniroute/opencode-provider/tests/index.test.ts`: provider object,
  config merge, model labels, context limits, live model fetch parsing.
- OmniRoute `@omniroute/opencode-plugin/tests/warm-startup.test.ts`: warm snapshot,
  failed refresh, per-endpoint fallback, concurrent in-flight guard, disk-cache mode.
- mini-SWE-agent `tests/models/test_litellm_model.py`,
  `tests/models/test_openrouter_textbased_model.py`,
  `tests/models/test_format_error_response_persistence.py`,
  `tests/models/test_truncation_finish_reason.py`: parsing, retry aborts, cost
  persistence, truncation/format errors.
- Gemini CLI `packages/core/src/availability/modelAvailabilityService.test.ts` and
  `packages/core/src/availability/policyHelpers.test.ts`: per-turn availability and
  retry-once behavior.
- Codex app-server tests `model_list.rs` and `model_provider_capabilities_read.rs`:
  model listing/capability read path.
- Aider model tests and source paths around LiteLLM/OpenRouter settings, retry, and
  local Ollama context sizing.
- OpenCode provider/config OpenAPI and LLM schema/provider source files for provider
  listing/auth and model option normalization.

## AgentCode Adaptation Summary

| Mechanism | Classification | AgentCode use |
|---|---|---|
| Typed provider/model capability catalog | TAKE | Provider fabric stores normalized provider/model/capability records. |
| OpenCode config writer | IGNORE | Product-specific adapter; not AgentCode architecture. |
| Warm discovery cache and stale snapshot | ADAPT | Discovery state can be stale-but-usable with explicit invalidation keys. |
| Raw `apiKey` in provider config | REJECT | Use credential refs and execution-time injection only. |
| SSE terminal-marker validation | TAKE | Partial streams without terminal markers are failures/degraded attempts. |
| Provider-specific status restatement | TAKE | Pre-classification normalization with rule evidence. |
| LiteLLM single abstraction | WRAP/STUDY | Useful adapter behind fabric, not source of truth. |
| Per-turn retry availability | ADAPT | Transient route attempt state; persisted as evidence, not task truth. |
| Health/quota/latency scoring | ADAPT | Only after Doc 01 hard filters and budget reserve checks. |
| Local model registry | ADAPT | Same provider contract; no special Kernel path. |
| Aider LiteLLM retry/local-model timeout handling | ADAPT | Adapter-level behavior only, behind AgentCode failure classes. |
| OpenCode provider/auth API shape | STUDY | Good API precedent; not an AgentCode state owner. |
