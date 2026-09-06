# WP-P05-WP01 — Tool Schema

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-tool`, `crates/ac-agent`
- **Architecture refs:** P01 WP05/WP09/WP10 implementation packets
- **Acceptance gates:** tool pipeline gates PASS for Phase 3 autonomous coding scope

## Acceptance Evidence

- Typed `ToolDescriptor`, `ExecutionManifest`, `ToolEvent`, `ToolError`, `ToolRequest`, and `ToolResult` are available in `ac-tool`.
- The real `AutonomousAgent` requests tools only through `ToolBroker`.

## Validation

- `cargo fmt` PASS.
- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.

## Handoff

Stable typed contracts support P05-WP02 through P05-WP11. Raw evidence is locally addressable through `EvidenceStore` and persistence records.
