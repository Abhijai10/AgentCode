# WP-P11-WP03 — Hard Inclusion

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Ensure mandatory requirements and explicitly requested sources bypass normal ranking.
- **Required implementation:** `ContextPackRequest` carries acceptance criteria, mission subset, explicit refs and project rules; builder treats these as mandatory and errors when they cannot fit.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`
- **Completion criteria:** Acceptance criteria and scoped project rules are included; mandatory context over budget fails instead of being silently dropped.
- **Tests:** `context_builder_applies_phase_11_gates`; `mandatory_context_over_hard_ceiling_is_rejected`
- **Migrations:** None beyond Phase 11 manifest persistence.
- **Limitations:** Mandatory source truncation is intentionally not performed; callers must request a larger profile or reroute.

## Acceptance

ACCEPTED. P11-G2/P11-G3/P11-G7 PASS: required criteria/rules are preserved and hard ceilings are enforced.
