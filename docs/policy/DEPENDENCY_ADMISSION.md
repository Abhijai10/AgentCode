# Dependency Admission Policy

Status: ACCEPTED (ADR-0012). Owner: project governance / build / CI.

## Purpose

Every foundational/runtime dependency enters AgentCode only through this written,
reviewed process. The reference-library directory
(`/Volumes/T7 Shield/GitHub-Repos-dependency`) is **development research input only**.
It must never become a runtime or build dependency of AgentCode.

## Scope

Applies to:

- Rust crates (runtime + dev/test)
- npm packages (runtime + dev/test)
- external binaries / tools (per ADR-0008 and `docs/policy/TOOL_REGISTRY.md`)
- vendored source (forbidden except by explicit ADR + license review)

## Admission Record

Create one record per dependency in `docs/policy/dependency-admissions/` using the
schema below. A dependency is admitted only when every field is answered and the
license gate passes.

```text
DEP-ADM-<NNN> — <dependency name> @ <version>

Need:
  What capability requires it? Which Work Package/gate?

Proposed dependency:
  name, version, source (registry/URL), pinned ref.

Existing alternative check:
  What is already in the tree that could serve? Why insufficient?

License:
  license identifier; classification per docs/legal/OSS_LICENSE_MATRIX.md;
  license-file inspection result (do not infer from popularity).

Security status:
  known CVEs (osv/advisory check), supply-chain posture, integrity metadata.

Maintenance state:
  release cadence, last release date, issue response, bus factor.

Runtime/bundle cost:
  binary/source size impact, dependency tree delta, memory/CPU impact.

Install/postinstall behavior:
  scripts, network calls, build requirements, artifacts.

Why existing components are insufficient:
  explicit.

License gate result: PASS / PENDING / BLOCKED
Admission decision: ADMIT / REJECT / DEFER (with condition)
Reviewer + date + evidence refs (e.g., osv-scan output, license file path).
```

## License Gate (summary; see docs/legal/OSS_LICENSE_MATRIX.md)

| Class | Verdict |
|-------|---------|
| Permissive (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0) | PASS (record attribution) |
| Copyleft strong (GPL, AGPL) | BLOCKED for runtime deps unless approved amendment |
| Weak copyleft (LGPL) | REVIEW (boundary analysis required) |
| Unlicensed / unknown | BLOCKED |
| Source-available non-OSS (BSL, SSPL, custom) | BLOCKED unless approved amendment |
| Public domain / CC0 | PASS with attribution note |

## Process

1. Author the admission record (template above).
2. Run the repo checks: `make dependency-check` (manifest/lock consistency,
   allowlist diff, forbidden reference-path scan).
3. License review per `docs/legal/LICENSE_REVIEW.md` workflow.
4. Independent review for `HIGH`/`CRITICAL` risk WPs (Doc 11 H21).
5. Record decision in `docs/progress/` work package evidence.
6. Update `docs/legal/THIRD_PARTY_NOTICES.md` and
   `docs/legal/third_party_manifest.json`.

## Prohibitions

- No `curl | sh` production installation strategy.
- No arbitrary global installs as architecture.
- No runtime/build path that touches `/Volumes/T7 Shield/GitHub-Repos-dependency`.
- No un-pinned dependency in committed lockfiles.
- No dependency whose license file has not been inspected.