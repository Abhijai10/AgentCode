# WP-P11-WP02 — Relevance Signals

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Score context fragments deterministically without introducing a learned ranking model.
- **Required implementation:** Added explicit scoring for hard includes, direct targets, errors, tests, diffs, project rules, freshness and task-term matches.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`
- **Completion criteria:** Role/task-specific pack ordering uses deterministic relevance traces captured in the manifest.
- **Tests:** `context_builder_applies_phase_11_gates`; `verifier_context_is_anti_anchored_against_worker_context`
- **Migrations:** None beyond Phase 11 manifest persistence.
- **Limitations:** Historical successful retrieval is represented through score/cache inputs, not an adaptive model.

## Acceptance

ACCEPTED. P11-G1 PASS: Worker and Verifier packs differ by role/task ranking.
