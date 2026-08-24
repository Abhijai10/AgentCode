# WP-P05-WP11 — Role Permissions

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-security`, `crates/ac-tool`

## Acceptance Evidence

Planner, Worker, Researcher, and Verifier role profiles are capability allowlists. Role policy is an intersection and cannot elevate mission/task/sandbox policy; high-risk actions require an approval grant.

## Validation

P5-G9, P5-G10, and P5-G11 tests PASS.
