# WP-P06-WP06 — Test Loop

- **Phase:** P06
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Owner modules:** `crates/ac-agent`, `crates/ac-verification`

The agent runs real verification, observes failures, generates a bounded repair plan, applies repair through Tool Broker, and retries.
