# WP-P16-WP01 — Skill Manifest

Status: ACCEPTED

Objective: Define portable skill metadata with identity, version, scope, trust, triggers, capability requirements, context cost, and full instructions.

Implementation summary: `ac-security` now provides `SkillManifest`, scoped/trusted metadata validation, and explicit full-instruction storage separate from summaries.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0011_extensions_skills_hooks_mcp.sql`.

Database changes: `skills`.

Tests: `phase16_skill_registry_progressive_loading_and_scope_work`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G1 covered.
