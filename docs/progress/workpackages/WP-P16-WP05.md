# WP-P16-WP05 — Built-In Skills

Status: ACCEPTED

Objective: Support built-in skill scope without bypassing the extension trust and capability model.

Implementation summary: `SkillScope::BuiltIn` participates in normal discovery and persistence; built-in status does not grant tool authority.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `skills.scope`, `skills.trust_tier`.

Tests: `phase16_skill_registry_progressive_loading_and_scope_work`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G3 and P16-G4 covered with scoped selection.
