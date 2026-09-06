# WP-P11-WP06 — Token Budget

- **Phase:** P11
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** dae337e
- **Accepted commit:** 17f2e92
- **Purpose:** Track target input, hard ceiling, reserved output, tool result allowance and history allowance.
- **Required implementation:** Added `TokenBudget` validation and `available_input()` enforcement inside the Context Engine pack builder.
- **Affected crates/files:** `crates/ac-context/src/lib.rs`
- **Completion criteria:** Invalid budgets fail, reserves are honored, and selected context cannot exceed the computed input ceiling.
- **Tests:** `mandatory_context_over_hard_ceiling_is_rejected`; `context_builder_applies_phase_11_gates`
- **Migrations:** None beyond Phase 11 metrics persistence.
- **Limitations:** Token accounting uses fallback estimates until provider tokenizer integration.

## Acceptance

ACCEPTED. P11-G7 PASS: context hard ceiling is respected.
