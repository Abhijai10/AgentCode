# WP-P16-WP12 — MCP Capability Discovery And Normalization

Status: ACCEPTED

Objective: Discover MCP tools, normalize schemas/capabilities, expose only task-relevant capabilities, and invoke selected tools through existing policy.

Implementation summary: `McpToolRecord`, discovery APIs, role/task filtering, invocation authorization, and `register_mcp_tool_with_broker` route selected MCP tools through `ToolBroker`.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-tool/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0011_extensions_skills_hooks_mcp.sql`.

Database changes: `mcp_tools`, `mcp_invocations`.

Tests: `phase16_mcp_discovery_filtering_invocation_and_recovery_work`, `phase16_mcp_tool_routes_through_broker_policy`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G12, P16-G13, and P16-G14 covered.
