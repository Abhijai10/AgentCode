# WP-P19-WP04 - RAG poisoning and retrieval-trust tests

Status: ACCEPTED
Base commit: 5df5bcba794186790b61fafd12459259b7d0b068
Phase: P19
Risk: HIGH/CRITICAL per playbook

## Objective

RAG poisoning and retrieval-trust tests.

## Implementation Summary

Added RAG poisoning fixture generation and result normalization where retrieval/vector surfaces exist.

## Affected Files

- `crates/ac-security/src/active.rs`
- `crates/ac-security/src/ai.rs`
- `crates/ac-security/src/scanner.rs`
- `crates/ac-security/src/tests.rs`
- `crates/ac-verification/src/lib.rs`
- `crates/ac-db/src/security.rs`
- `crates/ac-db/src/models.rs`
- `crates/ac-db/src/migrations.rs`
- `crates/ac-db/src/tests.rs`
- `crates/ac-migrations/src/lib.rs`
- `migrations/0013_advanced_ai_security.sql`
- `tests/integration/security_boundary.rs`

## Database Changes

Migration `migrations/0013_advanced_ai_security.sql` adds durable active-security authorization/report tables and AI-security report/attack-case tables. AI-security findings are also inserted into the existing `security_findings` table.

## Tests

- `crates/ac-security` unit tests cover active authorization, scope blocking, production-read-only stops, cleanup evidence, attack graphs, AI surface detection, attack fixtures and harness fallback status.
- `crates/ac-db` reopen test covers durable Phase 18/19 persistence and common AI finding storage.
- `crates/ac-verification` test covers security-evidence capability gating.
- `tests/integration/security_boundary.rs` covers security -> verification evidence -> database persistence integration.

## Acceptance Status

ACCEPTED. Applicable gates `P19-G1..P19-G10` are represented by production-facing APIs and final validation commands recorded in the phase completion package. Optional external adapters use native deterministic fallback/degraded statuses per ADR-0008 and phase hardening notes.
