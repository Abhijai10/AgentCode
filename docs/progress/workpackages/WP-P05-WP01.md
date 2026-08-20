# WP-P05-WP01 — Tool Pipeline Maturity

- **Phase:** P05
- **Status:** IMPLEMENTING
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 62439da380875cc06664b26f5cef9dcff17822c8
- **Accepted commit:** not accepted yet
- **Owner modules:** `crates/ac-tool`, `crates/ac-agent`
- **Architecture refs:** P01 WP05/WP09/WP10 implementation packets
- **Acceptance gates:** tool pipeline partial; full P05 gates not complete

## Implemented In This Batch

- Added repository tools: `repo.status`, `repo.diff`, `repo.branch`.
- Added development tools: `dev.test`, `dev.format`, `dev.check`.
- Routed fixed commands through SandboxManager with cleared environment and workspace root restrictions.
- Expanded tool evidence content hashes to include tool id, parameter length, and observation length.
- Added tests proving repository tool execution records evidence.

## Remaining Before Acceptance

- Project-specific command profile discovery.
- Command argument schema instead of whitespace-split shell-like payloads.
- Rich stdout/stderr artifact storage beyond in-memory evidence references.

