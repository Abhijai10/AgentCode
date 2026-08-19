# ADR-0010 — Configuration Layering

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

How is configuration loaded, layered and owned across AgentCode?

## Constraints

- Doc 09 HC-P02 (Configuration layering): compiled defaults → user config → project
  config → environment references → test overrides → runtime flags; "Individual
  packages must not read arbitrary environment variables throughout the codebase."
- Doc 11 §41: centralized config loading; no per-subsystem parsing of
  `~/.agentcode/config` or environment.
- Sensitive values must be secret references rather than ordinary configuration
  (Secret Broker in later phases; config subsystem must support the indirection now).

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|--------------|--------------|
| figment | Layered config for Rust | Extra dependency with its own magic; explicit layering wanted for auditable precedence |
| Each crate reads env directly | Simple | Forbidden by Doc 09 HC-P02 |
| First-party `ac-config` (chosen) | Typed settings, explicit layer stack, env-ref indirection, serde+toml | Full control; small code; testable precedence rules |

## Evidence

- Doc 09 HC-P02; Doc 11 §41.
- (Previously cited P01-WP04 extraction (donor config patterns) — that extraction
  record does not exist yet; claim removed in the 2026-08-20 ADR audit. P01-WP04 may
  confirm or refine the layer stack when it runs.)

## Decision

- **Rust (`crates/ac-config`)**: one typed `Settings` struct per component; layer
  stack (highest wins):
  1. runtime flags (CLI args)
  2. test overrides (explicit `ConfigOverride` API — tests never touch real files)
  3. environment references (only via explicit `EnvRef` resolution — one code path)
  4. project config (`<project>/.agentcode/config.toml`)
  5. user config (`~/.agentcode/config.toml`)
  6. compiled defaults
- Sensitive fields are typed as `SecretRef` (a reference to a secret by name/key) —
  resolution is Secret Broker's job later; config never stores plaintext secrets.
- **TypeScript**: `services/provider_gateway` owns the single TS config loader
  implementing the same layer order for its subset (env refs via one resolver).
- One config crate for all Rust crates; no other crate parses env/files.

## Consequences

- Positive: precedence is auditable and testable; secret indirection ready.
- Negative: config change requires recompilation for defaults; acceptable.
- Affected: Doc 09 HC-P02; Phase 3 daemon bootstrap, Phase 6 desktop settings.

## Verification

- P02-WP07 unit tests: precedence order, env-ref resolution, test-override isolation,
  secret-ref (non-plaintext) enforcement.