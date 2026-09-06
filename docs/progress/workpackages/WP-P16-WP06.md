# WP-P16-WP06 — Trust

Status: ACCEPTED

Objective: Ensure untrusted skill and extension content cannot override Tool Broker or policy decisions.

Implementation summary: Extension grants remain limited to declared capabilities, imported/untrusted manifests are data, and MCP trust defaults to untrusted until configured.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-tool/src/lib.rs`.

Database changes: `skills.trust_tier`, `mcp_servers.trust_tier`.

Tests: `phase16_imported_untrusted_skill_cannot_gain_tool_authority`, `phase16_mcp_tool_routes_through_broker_policy`, `phase16_mcp_discovery_filtering_invocation_and_recovery_work`.

Acceptance status: ACCEPTED; P16-G5 and P16-G15 covered.
