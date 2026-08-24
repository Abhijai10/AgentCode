# WP-P05-WP03 — Policy Engine

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-security`, `crates/ac-tool`, `crates/ac-sandbox`

## Acceptance Evidence

Capability policy denies before execution. `PermissionContext` intersects role, mission, task, sandbox, risk, and approval; R3/R4 actions require approval. Denials produce evidence.

## Validation

`role_and_risk_are_an_intersection_not_a_capability_escalation` PASS.
