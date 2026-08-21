# WP-P05-WP01 — Tool Pipeline Maturity

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 62439da380875cc06664b26f5cef9dcff17822c8
- **Accepted commit:** commit containing this record
- **Owner modules:** `crates/ac-tool`, `crates/ac-agent`
- **Architecture refs:** P01 WP05/WP09/WP10 implementation packets
- **Acceptance gates:** tool pipeline gates PASS for Phase 3 autonomous coding scope

## Implemented In This Batch

- Added repository tools: `repo.status`, `repo.diff`, `repo.branch`.
- Added development tools: `dev.test`, `dev.format`, `dev.check`.
- Routed fixed commands through SandboxManager with cleared environment and workspace root restrictions.
- Expanded tool evidence content hashes to include tool id, parameter length, and observation length.
- Added tests proving repository tool execution records evidence.
- Tool execution participates in the autonomous benchmark through `fs.read`, `fs.write`, `dev.test`, repository status/diff, verification retry, and repair execution.

## Validation

- `cargo fmt` PASS.
- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.

## Known Limitations

- Project-specific command profile discovery.
- Command argument schema instead of whitespace-split shell-like payloads.
- Rich stdout/stderr artifact storage beyond in-memory evidence references.

## Acceptance Decision

ACCEPTED for Phase 3: native tool execution is controlled by Tool Broker/Sandbox, emits evidence, and supports the autonomous coding/verification/repair path without direct bypass.
