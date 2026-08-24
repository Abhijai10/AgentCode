# WP-P16-WP07 — Importers

Status: ACCEPTED

Objective: Normalize compatible external skill formats without importing policy authority.

Implementation summary: `SkillRegistry::import_skill_markdown` converts markdown into a validated AgentCode skill manifest while preserving trust tier and scope validation.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: Uses `skills` when persisted by caller.

Tests: `phase16_imported_untrusted_skill_cannot_gain_tool_authority`.

Acceptance status: ACCEPTED; imported skills remain policy-bound.
