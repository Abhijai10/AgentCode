# WP-P00-WP06 — Developer Implementation Rules

- **Phase:** P00
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** c941bdd
- **Accepted commit:** (recorded in phase-00-completion.md)
- **Owner modules:** AGENTS.md
- **Architecture refs:** DOC-10 P0-G7; DOC-11 §21/H-P00, H33–H36
- **Acceptance gates:** P0-G7

## Objective

Top-level developer instructions embedding the source-of-truth hierarchy, no-drift
rule, read-before-write, WP procedure, gate requirement, no-fake-implementations,
dependency/license rules, Git safety, evidence/handoff, no-transcript rule, exact
session startup/shutdown.

## Inputs / Outputs

- Inputs: Doc 11 H34/H35, H28, H4
- Outputs: AGENTS.md

## Implementation

- Wrote concise operational AGENTS.md (12 sections); did not paste core docs into it.

## Tests / Evidence

- Independent Phase-0 understanding check (failure/understanding test) reads only
  governance artifacts and must answer: (a) can Kernel architecture be replaced
  casually? (b) may arbitrary cloned OSS be copied? (c) how does a phase become
  COMPLETE? — recorded in docs/progress/phase-00-review.md.

## Rollback / Cleanup / Handoff

- Handoff: all agents must read AGENTS.md before writing.