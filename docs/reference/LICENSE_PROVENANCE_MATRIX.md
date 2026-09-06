# License & Provenance Matrix (P01-WP02)

Canonical name: **LICENSE_PROVENANCE_MATRIX**. Data source: actual license files read from each clone at its pinned HEAD SHA (see docs/reference/agentcode_reference_catalog.json and docs/reference/licenses_scan.json); no license is inferred from popularity or README claims.

- Reference root: /Volumes/T7 Shield/GitHub-Repos-dependency (research input only; never a runtime/build dependency)
- Verified: 2026-08-20 recovery run. License files were read directly; SPDX determined from the file text (or-later clauses resolved; dual licenses recorded; CC-BY-SA vs CC0 disambiguated).
- Reuse classification: every cataloged repo is currently PATTERN_STUDY_ONLY (cataloged; no code copied; no extraction decision made). TAKE/ADAPT/WRAP/REJECT decisions are P01-WP03+ outputs and will update this matrix when recorded.
- Review state: INSPECTED = license file text read and classified; BLOCKED_FOR_COPY = reuse blocked until the exception is resolved.
- P1-G3: the matrix covers every candidate direct-dependency/adaptation source currently under consideration (the P0/P1 foundational repos).

## All cataloged repositories (62)

| Catalog ID | Repository | Pinned HEAD SHA | License file | SPDX | Reuse classification | Review state | Exception / blocker |
|---|---|---|---|---|---|---|---|
| REF-001 | agent-framework | `2213ef8493eb` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-002 | aider | `5dc9490bb35f` | LICENSE.txt | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-003 | ast-grep | `0eb08389b6c4` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-004 | bolt.diy | `2e254ac19a69` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-005 | browser-harness | `41108b8676d4` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-006 | browser-use | `898f23f0b672` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-007 | caveman | `ed48ec44b759` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-008 | checkov | `5458c889320a` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-009 | cline | `8a038022a439` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-010 | cloudgoat | `abf1ba8f5e47` | LICENSE | BSD-3-Clause | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-011 | cloudsploit | `43e27a7e0cee` | LICENSE | GPL-3.0-or-later | PATTERN_STUDY_ONLY | INSPECTED (with blockers) | GPL-3.0-or-later; strong copyleft — no reuse without amendment |
| REF-012 | codeql | `05c40eafe6fb` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-013 | codex | `77e688960196` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-014 | compound-engineering-plugin | `8b2a7d2dee98` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-015 | continue | `5522c6f44ca0` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-016 | ctags | `b0615e08c941` | COPYING | GPL-2.0-or-later | PATTERN_STUDY_ONLY | INSPECTED (with blockers) | GPL-2.0-or-later; strong copyleft — no reuse without amendment |
| REF-017 | daytona | `ec4c21b2d597` | none | UNKNOWN | PATTERN_STUDY_ONLY | UNKNOWN / BLOCKED_FOR_COPY | UNKNOWN license; BLOCKED_FOR_COPY until upstream license text confirmed |
| REF-018 | dyad | `22de43cf0d16` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | Apache-2.0 with src/pro/ portions clause (separate license) |
| REF-019 | garak | `7ff1f2778e4d` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-020 | gemini-cli | `571851b1077a` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-021 | gitleaks | `b58d3f102cf3` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-022 | goose | `935a37a68482` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-023 | graphiti | `10374d6044f9` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-024 | langgraph | `644815f9e5bc` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-025 | letta | `87fd37aab68c` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-026 | letta-code | `5786193dd10d` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-027 | letta-skills | `16352df0a3ce` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-028 | mini-swe-agent | `25941c89cfbc` | LICENSE.md | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-029 | munder-difflin | `5fb93721030b` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | REJECTED for AgentCode use per Doc 07 25-26 |
| REF-030 | nuclei | `34866efc2274` | LICENSE.md | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-031 | nuclei-templates | `55cce9268a5f` | LICENSE.md | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-032 | OmniRoute | `df905914154b` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-033 | onlook | `423e2e924366` | LICENSE.md | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-034 | opencode | `9b0dd36cda0b` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-035 | openhack | `87481e9f374e` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-036 | OpenHands | `b25f9b3969f9` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-037 | osv-scanner | `c84fa4568f25` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-038 | pacu | `e597f23ecfb8` | LICENSE | BSD-2-Clause | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-039 | playwright | `5f8e7eac8305` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-040 | promptfoo | `c149fcf36c2a` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-041 | prowler | `0b9791ffdc6e` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-042 | PyRIT | `4598a40aca7d` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-043 | ripgrep | `3fce3b5bb023` | COPYING | Unlicense OR MIT | PATTERN_STUDY_ONLY | INSPECTED | dual Unlicense OR MIT; permissive side available |
| REF-044 | Roo-Code | `b867ec914575` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-045 | rtk | `ba7a9ce0d92a` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-046 | scip | `8b8c4fc0dea6` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-047 | scorecard | `d1fab88f5463` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-048 | ScoutSuite | `7909f2fc6186` | LICENSE | GPL-2.0-or-later | PATTERN_STUDY_ONLY | INSPECTED (with blockers) | GPL-2.0-or-later; strong copyleft — no reuse without amendment |
| REF-049 | semgrep | `808ee51cda4f` | LICENSE | LGPL-2.1-or-later | PATTERN_STUDY_ONLY | INSPECTED (with blockers) | LGPL-2.1-or-later; weak copyleft — boundary review required |
| REF-050 | servers | `599dafc10545` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-051 | smolagents | `e3a5b8994b30` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-052 | software-agent-sdk | `98338ff37aea` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-053 | stack-graphs | `fcb7705d5b38` | LICENSE-APACHE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-054 | stratus-red-team | `c12d6654bce9` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-055 | superpowers | `b36e0829c6d0` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-056 | SWE-bench | `87ab1f6ced28` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-057 | SWE-ReX | `5c995c365dfb` | LICENSE.txt | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-058 | trailofbits-skills | `a551f0b5f76d` | LICENSE | CC-BY-SA-4.0 | PATTERN_STUDY_ONLY | INSPECTED (with blockers) | CC-BY-SA-4.0 share-alike; reuse requires compliance review |
| REF-059 | tree-sitter | `f235c2f1c399` | LICENSE | MIT | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-060 | trivy | `dcbadb7b1507` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-061 | zaproxy | `e3793d73d04f` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |
| REF-062 | zoekt | `dcd8c9ca9b84` | LICENSE | Apache-2.0 | PATTERN_STUDY_ONLY | INSPECTED | — |

## Known exceptions / blockers

1. daytona — no license file in clone: UNKNOWN, BLOCKED_FOR_COPY (P1-G7 record).
2. cloudsploit, ScoutSuite, ctags — GPL family (strong copyleft): BLOCKED for any reuse without an approved amendment.
3. semgrep — LGPL-2.1-or-later (weak copyleft): boundary review required before any reuse decision.
4. trailofbits-skills — CC BY-SA 4.0: share-alike compliance review required.
5. ripgrep — dual Unlicense OR MIT: permissive side (MIT) usable if adopted.
6. dyad — Apache-2.0 except src/pro/ subtree with separate license; avoid or review that subtree.
7. munder-difflin — REJECTED for AgentCode use per Doc 07 §25–26 (recorded).

## Rules

1. No code is copied from any repository in this library without a recorded reuse decision, license-file inspection at the pinned SHA, and attribution (docs/legal/).
2. This matrix is updated by every extraction campaign (P01-WP03+) when a repo moves beyond PATTERN_STUDY_ONLY.
3. Unknowns stay UNKNOWN until primary evidence exists.
