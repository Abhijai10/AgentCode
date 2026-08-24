# Phase 16 Completion — Skills, Hooks & MCP

Status: COMPLETE

Starting commit: `6264cd3`

Implementation commit: pending final commit

Completed work packages: P16-WP01 through P16-WP13 are ACCEPTED.

Implemented capabilities: skill manifests, durable skill registry, scoped discovery, progressive full-instruction loading, built-in/project/task skill scopes, imported markdown skills, extension trust enforcement, hook manifests, lifecycle dispatch, timeout/failure/idempotency/recursion handling, completion-hook rejection, MCP server lifecycle, MCP tool discovery, role/task capability filtering, selected MCP tool routing through Tool Broker, MCP crash/reconnect state, and durable extension persistence.

Architecture notes: Kernel authority remains unchanged. Skills, hooks, and MCP metadata are untrusted extension-plane data until selected and policy-approved. Executable extension actions route through `ToolBroker`; durable state uses the existing SQLite control-plane migration path.

Migration impact: added `migrations/0011_extensions_skills_hooks_mcp.sql`; database `user_version` is now 11.

Validation evidence:
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- JSON validation
- `git diff --check`

Gate mapping: P16-G1 through P16-G15 pass via Phase 16 tests in `ac-security`, `ac-tool`, and `ac-db`.

Limitations: External MCP transports are represented by typed lifecycle records and deterministic local broker routing in this phase; unavailable external runtime clients degrade to explicit metadata/lifecycle state rather than synthetic success.
