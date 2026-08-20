# WP15 AI Security Extraction

## Donors Inspected

| Donor | Commit | Files inspected |
|---|---:|---|
| promptfoo | `c149fcf36c2a` | `src/evaluator.ts`; `src/redteam/index.ts` |
| garak | `7ff1f2778e4d` | `garak/harnesses/base.py`; `garak/detectors/base.py` |
| PyRIT | `4598a40aca7d` | `pyrit/converter/converter.py`; `pyrit/score/*` file inventory |

## Mechanisms Discovered

### promptfoo eval runner

Source: `src/evaluator.ts`.

Important symbols: `runAssertions`, `ProviderGroupedCallQueue`, `RateLimitRegistry`, `TokenUsageTracker`, `ProgressBarManager`, `PROMPT_CONVERSATION_CACHE_MAX`, `promptUsesConversationVariable`, `withTracedProviderCall`, `ResultFailureReason`.

Control flow: resolves prompts/providers/tests, detects conversation-aware prompts, schedules provider calls with concurrency/rate limits, runs assertions/grading, traces provider calls, tracks token usage, writes evaluation records.

Failure behavior: parse failure avoids poisoning conversation cache; provider/rate-limit errors are classified; max concurrency and timeouts are environment/config controlled.

### promptfoo redteam generation

Source: `src/redteam/index.ts`.

Important symbols: `Plugins`, `Strategies`, `riskCategorySeverityMap`, `getPluginSeverity`, `MAX_MAX_CONCURRENCY`, `rematerializeStrategyInputVars`, `resolveRedteamGenerationContext`.

Control flow: loads plugins/strategies/policies, extracts purpose/entities, materializes variables, assigns severities, validates strategy fanout/concurrency, and generates test cases.

Failure behavior: malformed materialized JSON falls back to prior variables; missing remote health/config blocks remote generation.

### garak harness/probe/detector model

Sources: `garak/harnesses/base.py`; `garak/detectors/base.py`.

Important symbols: `Harness`, `_initialize_runtime_services`, `_emit_plugin_cache_entry`, `_load_buffs`, `_run_detector`, `run`, `Detector`, `detect`, `HFDetector`.

Control flow: harness initializes services, loads buffs, emits plugin cache entry to report, checks probe/detector presence, validates modality match, runs probes against model, runs detectors over attempts, and evaluator consumes results.

Failure behavior: no probes/detectors raise `ValueError`; modality mismatch skips probe; detector dependencies load during init; HF detector disables unsafe safetensors conversion PR behavior.

### PyRIT converter/scorer architecture

Source: `pyrit/converter/converter.py`; scorer inventory under `pyrit/score/`.

Important symbols: `Converter`, `ConverterResult`, `SUPPORTED_INPUT_TYPES`, `SUPPORTED_OUTPUT_TYPES`, `TARGET_REQUIREMENTS`, `convert_async`, `convert_tokens_async`.

Control flow: converter subclasses must declare supported modalities and keyword-only init; optional converter target is validated against target requirements; token-delimited segments can be converted concurrently and reassembled.

Failure behavior: missing modality declarations raise `TypeError`; mismatched token delimiters raise `ValueError`.

## Limitations and Rejections

AI security tests are probabilistic. They require reproducible seeds/configs/model ids and cannot be treated as proof of absence. Generated attack prompts are untrusted data.
