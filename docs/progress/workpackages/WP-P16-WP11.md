# WP-P16-WP11 — MCP Server Registry And Connection Lifecycle

Status: ACCEPTED

Objective: Track MCP server identity, transport, trust, health, and restart count.

Implementation summary: `McpRegistry` registers servers as untrusted by default, connects, marks crashes, reconnects with restart accounting, and persists state through `ac-db`.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0011_extensions_skills_hooks_mcp.sql`.

Database changes: `mcp_servers`.

Tests: `phase16_mcp_discovery_filtering_invocation_and_recovery_work`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G11 covered.
