# WP-P16-WP13 — MCP Trust, Policy And Failure Handling

Status: ACCEPTED

Objective: Treat MCP descriptions/results as untrusted data and prevent malicious MCP tools from bypassing policy.

Implementation summary: MCP servers start untrusted, exposure requires configured trust and matching role/task policy, invocations return explicit policy decisions, and broker execution records evidence for denied or successful calls.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-tool/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `mcp_servers`, `mcp_invocations`.

Tests: `phase16_mcp_discovery_filtering_invocation_and_recovery_work`, `phase16_mcp_tool_routes_through_broker_policy`.

Acceptance status: ACCEPTED; P16-G14 and P16-G15 covered.
