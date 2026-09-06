# WP-P16-WP03 — Discovery

Status: ACCEPTED

Objective: Discover applicable skills by project/task scope and task hints.

Implementation summary: `SkillRegistry::summaries` and `select` filter built-in, project, and task skills by scoped context and deterministic trigger matching.

Affected files: `crates/ac-security/src/lib.rs`.

Database changes: None beyond registry metadata from WP02.

Tests: `phase16_skill_registry_progressive_loading_and_scope_work`.

Acceptance status: ACCEPTED; P16-G1, P16-G3, and P16-G4 covered.
