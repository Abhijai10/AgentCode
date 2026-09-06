# WP-P16-WP04 — Progressive Loading

Status: ACCEPTED

Objective: Show models skill summaries first and full instructions only after selection.

Implementation summary: `SkillSummary` omits full instructions, while `SkillRegistry::load_full` records load time and returns `LoadedSkill` only for the selected skill.

Affected files: `crates/ac-security/src/lib.rs`, `crates/ac-db/src/lib.rs`.

Database changes: `skills.loaded_at_ms`.

Tests: `phase16_skill_registry_progressive_loading_and_scope_work`, `phase16_extension_state_survives_reopen`.

Acceptance status: ACCEPTED; P16-G2 covered.
