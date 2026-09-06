# WP-P16-WP02 — Registry

Status: ACCEPTED

Objective: Persist and query skill registry metadata without making skills authoritative.

Implementation summary: `SkillRegistry` registers validated manifests and `ControlPlaneDb::save_skill` persists registry state for reopen/recovery.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0011_extensions_skills_hooks_mcp.sql`.

Database changes: `skills`.

Tests: `phase16_skill_registry_progressive_loading_and_scope_work`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G1 covered.
