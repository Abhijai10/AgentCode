# ADR-0012 — Dependency Admission Policy

- **Status:** ACCEPTED
- **Decision class:** POLICY
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

How does a new runtime/foundational dependency enter AgentCode, and what must be
verified first?

## Constraints

- Doc 07 §89–95 (license strategy/classification/gate, attribution, manifest), §96–97
  (pin versions, no submodules for reference library), §98 (do not vendor entire
  repos).
- Doc 11 H12 (package-install policy), H13 (external tool provenance), H4 gate: no
  unvetted dependencies.
- Doc 09 HC-P02: lockfiles committed; CI verifies manifest/lock consistency.
- Reference-library directory is research input only — never a runtime dependency.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|--------------|--------------|
| Install-as-needed | Fast | No provenance; violates auditability |
| Cargo/pnpm allowlists only | Technical guard | Doesn't cover license/security review |
| Admission form + review (chosen) | Every dependency passes a written admission record | Matches Doc 11 H12 spirit; audit trail |

## Evidence

- Doc 07 §89–98; Doc 11 H12–H13; P00-WP03 procedure (this batch).

## Decision

- Any new foundational/runtime dependency (Rust crate, npm package, external binary)
  requires a **dependency admission record** (schema in
  `docs/policy/DEPENDENCY_ADMISSION.md`) covering:
  - need; proposed dependency; existing-alternative check; version/source; license;
    security status; maintenance state; runtime/bundle cost; install/postinstall
    behavior; why existing components are insufficient.
- License gate per Doc 07 §91 (OSS_LICENSE_MATRIX.md classification) — unclassified
  license = blocked.
- No `curl | sh`; no arbitrary global installs as architecture (ADR-0008).
- Lockfiles (Cargo.lock, pnpm-lock.yaml) committed; CI checks manifest/lock
  consistency and a pinned-allowlist diff gate for runtime deps.
- Runtime dependency on the reference-library folder is forbidden and CI asserts it
  (no `GitHub-Repos-dependency` path references in build manifests).
- Dev/test dependencies go through the same form at reduced depth but still record
  license + security status.

## Consequences

- Positive: auditable dependency provenance; CI-enforced allowlist growth.
- Negative: small overhead per dependency; enforced by process + scripts.
- Affected: Doc 07 §89–98; every phase introducing dependencies.

## Verification

- P0-G4; `make dependency-check` (manifest/lock consistency + allowlist + forbidden
  path scan) runs in CI.