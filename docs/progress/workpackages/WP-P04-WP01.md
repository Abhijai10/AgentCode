# WP-P04-WP01 — OmniRoute Fork

- **Phase:** P04
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** `48bc0e267c43c8881acf917ee885d1a2901afaa3`
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-provider`, `docs/vendor/OMNIROUTE_BASE.md`
- **Acceptance gates:** P4-G3/P4-G4/P4-G10/P4-G11 PASS

## Implemented

- Added AgentCode-controlled OmniRoute-equivalent routing layer inside the existing ProviderRegistry.
- Added route resolution over registered provider connections and model routes.
- Added deterministic candidate scoring and rejection reasons.
- Added `docs/vendor/OMNIROUTE_BASE.md` recording no donor source was copied.

## Validation

- `cargo check --workspace --all-targets` PASS.
- `cargo test -p ac-provider --lib` PASS.
- Full workspace validation recorded in `docs/progress/phase-04-completion.md`.

## Acceptance Decision

ACCEPTED: route resolution is production-integrated and does not create a parallel provider architecture.
