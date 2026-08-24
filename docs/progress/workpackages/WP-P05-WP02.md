# WP-P05-WP02 — Tool Registry

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-tool`

## Acceptance Evidence

`ToolBroker` holds typed definitions and executors, rejects duplicate registrations and version mismatches, evaluates capabilities before invocation, and emits denied/failed/succeeded evidence.

## Validation

`cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` PASS.
