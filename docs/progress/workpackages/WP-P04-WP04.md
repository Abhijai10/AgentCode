# WP-P04-WP04 — Model Catalog

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`
- **Acceptance gates:** P4-G3 PASS

## Implemented

- Added `ModelIdentity` separate from provider-scoped `ModelCapability`.
- Catalog stores model family, context window, coding/reasoning scores, vision/tool/structured-output flags, cost, and privacy class.
- `ModelRoute` maps model identity to provider connection/model name.

## Validation

- Ranking tests prove candidate ordering is based on model identity and policy, not provider connection identity alone.

## Acceptance Decision

ACCEPTED: model identity and provider route are separate.
