# WP-P05-WP10 — Secret Injection

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-sandbox`, `crates/ac-tool`, `crates/ac-evidence`

## Acceptance Evidence

`SecretBroker` resolves only named references at an approved sandbox execution boundary. Tool Broker captures raw evidence while redacting secret values from the agent-facing result and evidence summary.

## Validation

`approved_secret_process_is_redacted_from_agent_result` PASS.
