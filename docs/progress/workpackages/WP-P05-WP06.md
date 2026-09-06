# WP-P05-WP06 — Exact Search

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-tool`

## Acceptance Evidence

`fs.search` returns deterministic path, line, and literal-match records through the Tool Broker while refusing symlink traversal. It has no external binary dependency.

## Validation

P5-G1 path/search coverage PASS.
