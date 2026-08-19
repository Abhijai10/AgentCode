# AgentCode Reference Catalog

Canonical name: **AGENTCODE_REFERENCE_CATALOG**. Machine-readable companion:
`agentcode_reference_catalog.json` (this file is generated from it).

- Reference root: `/Volumes/T7 Shield/GitHub-Repos-dependency`
- **This library is DEVELOPMENT RESEARCH INPUT ONLY. It is never a runtime or build
  dependency of AgentCode.** No committed path references it.
- Status: ACTIVE, updated 2026-08-20 (batch: Phase 0 + Phase 1 foundation slice).
- Categories and priorities follow Doc 07 §14–19.

## Generation

```text
node scripts/collect-reference-catalog.mjs   # identity scan (SHA, branch, dirty, licenses)
node scripts/enrich-reference-catalog.mjs    # categories, priorities, roles, SPDX classification
```

License classification is read from actual license files (never inferred from
popularity). See `docs/legal/OSS_LICENSE_MATRIX.md` for the detailed matrix and
`docs/reference/licenses_scan.json` for the scan data.

## Repositories (62)

| Catalog ID | Repository | Priority | Category | HEAD SHA | Branch | License | Extraction status |
|------------|------------|----------|----------|----------|--------|---------|-------------------|
| REF-001 | agent-framework | P1 | ORCHESTRATION | 2213ef8493eb | main | MIT | NOT_EXTRACTED |
| REF-002 | aider | P0 | CODING AGENTS | 5dc9490bb35f | main | Apache-2.0 | NOT_EXTRACTED |
| REF-003 | ast-grep | P0 | CODE INTELLIGENCE | 0eb08389b6c4 | main | MIT | NOT_EXTRACTED |
| REF-004 | bolt.diy | P1 | DESIGN / APP BUILDERS | 2e254ac19a69 | main | MIT | NOT_EXTRACTED |
| REF-005 | browser-harness | P3 | BROWSER / QA | 41108b8676d4 | main | MIT | NOT_EXTRACTED |
| REF-006 | browser-use | P3 | BROWSER / QA | 898f23f0b672 | main | MIT | NOT_EXTRACTED |
| REF-007 | caveman | P3 | CODING AGENTS | ed48ec44b759 | main | MIT | NOT_EXTRACTED |
| REF-008 | checkov | P2 | APPLICATION SECURITY | 5458c889320a | main | Apache-2.0 | NOT_EXTRACTED |
| REF-009 | cline | P1 | CODING AGENTS | 8a038022a439 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-010 | cloudgoat | P2 | RED TEAM | abf1ba8f5e47 | master | BSD-3-Clause | NOT_EXTRACTED |
| REF-011 | cloudsploit | P2 | CLOUD SECURITY | 43e27a7e0cee | master | GPL-3.0-or-later | NOT_EXTRACTED |
| REF-012 | codeql | P2 | APPLICATION SECURITY | 05c40eafe6fb | main | MIT | NOT_EXTRACTED |
| REF-013 | codex | P0 | CODING AGENTS | 77e688960196 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-014 | compound-engineering-plugin | P1 | SKILLS / WORKFLOWS | 8b2a7d2dee98 | main | MIT | NOT_EXTRACTED |
| REF-015 | continue | P2 | CODING AGENTS | 5522c6f44ca0 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-016 | ctags | P3 | CODE INTELLIGENCE | b0615e08c941 | master | GPL-2.0-or-later | NOT_EXTRACTED |
| REF-017 | daytona | P2 | EXECUTION / SANDBOX | ec4c21b2d597 | main | UNKNOWN | NOT_EXTRACTED |
| REF-018 | dyad | P1 | DESIGN / APP BUILDERS | 22de43cf0d16 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-019 | garak | P2 | AI SECURITY | 7ff1f2778e4d | main | Apache-2.0 | NOT_EXTRACTED |
| REF-020 | gemini-cli | P0 | CODING AGENTS | 571851b1077a | main | Apache-2.0 | NOT_EXTRACTED |
| REF-021 | gitleaks | P2 | APPLICATION SECURITY | b58d3f102cf3 | master | MIT | NOT_EXTRACTED |
| REF-022 | goose | P1 | CODING AGENTS | 935a37a68482 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-023 | graphiti | P1 | MEMORY / CONTEXT | 10374d6044f9 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-024 | langgraph | P1 | ORCHESTRATION | 644815f9e5bc | main | MIT | NOT_EXTRACTED |
| REF-025 | letta | P3 | MEMORY / CONTEXT | 87fd37aab68c | main | Apache-2.0 | NOT_EXTRACTED |
| REF-026 | letta-code | P0 | MEMORY / CONTEXT | 5786193dd10d | main | Apache-2.0 | NOT_EXTRACTED |
| REF-027 | letta-skills | P3 | SKILLS / WORKFLOWS | 16352df0a3ce | main | MIT | NOT_EXTRACTED |
| REF-028 | mini-swe-agent | P0 | CODING AGENTS | 25941c89cfbc | main | MIT | NOT_EXTRACTED |
| REF-029 | munder-difflin | P3 | SKILLS / WORKFLOWS | 5fb93721030b | main | MIT | NOT_EXTRACTED |
| REF-030 | nuclei | P2 | APPLICATION SECURITY | 34866efc2274 | dev | MIT | NOT_EXTRACTED |
| REF-031 | nuclei-templates | P3 | APPLICATION SECURITY | 55cce9268a5f | main | MIT | NOT_EXTRACTED |
| REF-032 | OmniRoute | P0 | MODEL / PROVIDER ROUTING | df905914154b | release/v3.8.50 | MIT | NOT_EXTRACTED |
| REF-033 | onlook | P1 | DESIGN / APP BUILDERS | 423e2e924366 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-034 | opencode | P1 | CODING AGENTS | 9b0dd36cda0b | dev | MIT | NOT_EXTRACTED |
| REF-035 | openhack | P0 | ORCHESTRATION | 87481e9f374e | main | MIT | NOT_EXTRACTED |
| REF-036 | OpenHands | P0 | CODING AGENTS | b25f9b3969f9 | main | MIT | NOT_EXTRACTED |
| REF-037 | osv-scanner | P2 | APPLICATION SECURITY | c84fa4568f25 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-038 | pacu | P2 | RED TEAM | e597f23ecfb8 | master | BSD-2-Clause | NOT_EXTRACTED |
| REF-039 | playwright | P0 | BROWSER / QA | 5f8e7eac8305 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-040 | promptfoo | P2 | EVALUATION | c149fcf36c2a | main | MIT | NOT_EXTRACTED |
| REF-041 | prowler | P1 | CLOUD SECURITY | 0b9791ffdc6e | master | Apache-2.0 | NOT_EXTRACTED |
| REF-042 | PyRIT | P2 | AI SECURITY | 4598a40aca7d | main | MIT | NOT_EXTRACTED |
| REF-043 | ripgrep | P0 | CODE INTELLIGENCE | 3fce3b5bb023 | master | Unlicense OR MIT | NOT_EXTRACTED |
| REF-044 | Roo-Code | P2 | CODING AGENTS | b867ec914575 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-045 | rtk | P0 | TOKEN EFFICIENCY | ba7a9ce0d92a | develop | Apache-2.0 | NOT_EXTRACTED |
| REF-046 | scip | P1 | CODE INTELLIGENCE | 8b8c4fc0dea6 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-047 | scorecard | P3 | EVALUATION | d1fab88f5463 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-048 | ScoutSuite | P2 | CLOUD SECURITY | 7909f2fc6186 | master | GPL-2.0-or-later | NOT_EXTRACTED |
| REF-049 | semgrep | P2 | APPLICATION SECURITY | 808ee51cda4f | develop | LGPL-2.1-or-later | NOT_EXTRACTED |
| REF-050 | servers | P3 | SKILLS / WORKFLOWS | 599dafc10545 | main | MIT | NOT_EXTRACTED |
| REF-051 | smolagents | P3 | CODING AGENTS | e3a5b8994b30 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-052 | software-agent-sdk | P0 | CODING AGENTS | 98338ff37aea | main | MIT | NOT_EXTRACTED |
| REF-053 | stack-graphs | P3 | CODE INTELLIGENCE | fcb7705d5b38 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-054 | stratus-red-team | P2 | RED TEAM | c12d6654bce9 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-055 | superpowers | P1 | SKILLS / WORKFLOWS | b36e0829c6d0 | main | MIT | NOT_EXTRACTED |
| REF-056 | SWE-bench | P2 | EVALUATION | 87ab1f6ced28 | main | MIT | NOT_EXTRACTED |
| REF-057 | SWE-ReX | P0 | CODING AGENTS | 5c995c365dfb | main | MIT | NOT_EXTRACTED |
| REF-058 | trailofbits-skills | P1 | SKILLS / WORKFLOWS | a551f0b5f76d | main | CC-BY-SA-4.0 | NOT_EXTRACTED |
| REF-059 | tree-sitter | P0 | CODE INTELLIGENCE | f235c2f1c399 | master | MIT | NOT_EXTRACTED |
| REF-060 | trivy | P2 | APPLICATION SECURITY | dcbadb7b1507 | main | Apache-2.0 | NOT_EXTRACTED |
| REF-061 | zaproxy | P2 | APPLICATION SECURITY | e3793d73d04f | main | Apache-2.0 | NOT_EXTRACTED |
| REF-062 | zoekt | P1 | CODE INTELLIGENCE | dcd8c9ca9b84 | main | Apache-2.0 | NOT_EXTRACTED |

## Known License Exceptions

- **daytona** (P2, EXECUTION / SANDBOX): the local clone contains **no license file**
  and no declared license. Recorded as `UNKNOWN`; excluded from any reuse until the
  upstream license is confirmed from the remote. (P1-G7 exception record.)
- **trailofbits-skills** (P1, SKILLS / WORKFLOWS): `LICENSE` is CC BY-SA 4.0
  (Attribution-ShareAlike 4.0 International). Share-alike copyleft: reuse requires
  share-alike compliance review before any material is used.
- **ripgrep** (P0, CODE INTELLIGENCE): dual-licensed `Unlicense OR MIT` (per COPYING).
- **dyad** (P1, DESIGN / APP BUILDERS): top-level LICENSE is Apache-2.0 with a
  "portions" clause — content under `src/pro/` is licensed under that directory's own
  LICENSE.
- **semgrep** (P2), **cloudsploit** (P2), **ScoutSuite** (P2), **ctags** (P3): GPL/
  LGPL family with "or later" clauses — strong-copyleft use is BLOCKED without an
  approved amendment; pattern-study only for now.

## Extraction Status Legend

- `NOT_EXTRACTED` — cataloged only (later campaigns).
- `EXTRACTED` — source-level extraction record exists in `docs/extraction/`.
- `EXTRACTED_FOUNDATION` — extraction performed for the Phase 1 foundation slice.
- `PENDING_REVIEW` — extraction done, independent review outstanding.
- `REJECTED` — recorded as REJECT for AgentCode use (e.g., munder-difflin per Doc 07
  §25–26).

## Rules

1. One canonical catalog; no second independently maintained OSS catalog.
2. Every repo has a recorded SHA before it may be used in extraction.
3. Dirty repositories are used read-only; nothing is written into the reference root.
4. Extraction records pin the exact SHA used (extracted_at_commit).