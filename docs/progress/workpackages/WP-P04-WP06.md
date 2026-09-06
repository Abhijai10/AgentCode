# WP-P04-WP06 — Local Ollama

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** MEDIUM | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`
- **Acceptance gates:** P4-G2 PASS

## Implemented

- Added `OllamaProviderAdapter` as a local-model adapter wrapping provider HTTP transport.
- Added local model identity/route support using `PrivacyClass::LocalOnly` and local connection flags.
- Offline/local-first routing can select Ollama-style local models when configured.

## Validation

- `local_ollama_route_is_selected_for_offline_profile` proves local route selection.

## Acceptance Decision

ACCEPTED: local Ollama route is first-class in the same provider fabric.
