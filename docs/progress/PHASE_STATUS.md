# Phase Status Tracker

Status model (Doc 09 §6, Doc 11): NOT_STARTED / IN_PROGRESS / COMPLETE (only when all
Doc 10 gates for the phase pass and completion evidence exists).

Phase names are the EXACT canonical names from Doc 09 HC-P00..HC-P29 (no aliases).

Last updated: 2026-08-24 (Phase 26 security/licensing and Phase 27 release engineering closure)

| Phase | Name (Doc 09 HC-*) | Status | Notes |
|-------|--------------------|--------|-------|
| P00 | Project Governance & Architecture Freeze | COMPLETE | all P00 WPs ACCEPTED; gates P0-G1..G7 pass with recorded evidence; failure/understanding checks PASS; evidence: docs/progress/phase-00-completion.md (+ .json), phase-00-governance-validation.md, adr-audit-phase00.md |
| P01 | OSS Extraction & Evidence Collection | COMPLETE | P01-WP01..P01-WP17 ACCEPTED; P1-G1..P1-G8 PASS; evidence: docs/progress/phase-01-completion.md (+ .json), phase-01-gate-review.md, phase-01-architecture-dependency-map.md |
| P02 | Repository Skeleton & Development Infrastructure | COMPLETE | P02-WP01 ACCEPTED; P2-G1..P2-G8 PASS; evidence: docs/progress/phase-02-completion.md (+ .json) |
| P03 | Background Daemon & Persistent Kernel Foundation | COMPLETE | P03-WP01 ACCEPTED for autonomous coding scope; SQLite persists Kernel mission/event state plus AgentSession, WorktreeRecord, ChangeSet, and checkpoint recovery evidence; completion evidence: docs/progress/phase-03-completion.md (+ .json) |
| P04 | Model Broker & OmniRoute Provider Fabric | COMPLETE | P04-WP01..P04-WP12 ACCEPTED; ProviderRegistry owns OmniRoute-style routing, model catalog, provider connections, local Ollama route, health/circuit/fallback, cost policy, and persisted routing evidence; evidence: docs/progress/phase-04-completion.md (+ .json) |
| P05 | Native Tool Runtime Foundation | COMPLETE | P05-WP01..P05-WP11 ACCEPTED; Brokered filesystem/process runtime enforces canonical workspace paths, capability/risk/role policy, secret redaction, structured raw evidence, persistent execution records, timeout/cancellation, and background process control; evidence: docs/progress/phase-05-completion.md (+ .json) |
| P06 | Basic Worker Agent Loop | COMPLETE | P06-WP01..P06-WP09 ACCEPTED; durable worker/task/attempt graph, retry and recovery state, context/provider/tool/verification/repair loop, evidence-backed completion request, and real autonomous benchmark pass; evidence: docs/progress/phase-06-completion.md (+ .json) |
| P07 | Git, Worktrees & Checkpointing | COMPLETE | P07-WP01..P07-WP09 ACCEPTED; durable lease fencing, dirty-base protection, task checkpoints, replacement recovery, safe cleanup/reconciliation, integration branches, and structured conflicts pass P7-G1..G9; evidence: docs/progress/phase-07-completion.md (+ .json) |
| P08 | Code Intelligence Foundation | COMPLETE | P08-WP01..P08-WP13 ACCEPTED; deterministic parser/search fallbacks, durable structural index, import graph, incremental indexing, polling watcher and ContextBuilder retrieval evidence are implemented; evidence: docs/progress/phase-08-completion.md (+ .json) |
| P09 | Semantic Intelligence & Repository Graph | COMPLETE | P09-WP01..P09-WP14 ACCEPTED; LSP lifecycle contracts, language adapters, normalized definitions/references/diagnostics, optional SCIP/Zoekt decisions, unified semantic graph, workspace/test/API/schema edges and durable semantic persistence pass P9-G1..G10; evidence: docs/progress/phase-09-completion.md (+ .json) |
| P10 | Persistent Memory & Knowledge Freshness | COMPLETE | P10-WP01..P10-WP10 ACCEPTED; typed evidence-linked facts, confidence/freshness, source invalidation, conflict sets, decision history, generated non-authoritative CONTEXT.md, snapshots, task memory and replacement handoff pass P10-G1..G11; evidence: docs/progress/phase-10-completion.md (+ .json) |
| P11 | Context Engine & Token Efficiency | COMPLETE | P11-WP01..P11-WP13 ACCEPTED; typed fragments, deterministic relevance, hard includes, scoped rules, sensitivity filtering, profiles, token budgets, pack builder, dedupe, progressive retrieval, RTK-style compression, cache, metrics and benchmark receipts pass P11-G1..G11 plus benchmark gate; evidence: docs/progress/phase-11-completion.md (+ .json) |
| P12 | Full Autonomy Kernel & Multi-Agent Runtime | COMPLETE | P12-WP01..P12-WP24 ACCEPTED; mission contracts, requirement matrix, DAG validation, scheduler, worker registry, leases, heartbeats, progress/stall/loop detection, recovery/retry, Researcher, Verifier, mailbox, blackboard, concurrency, conflict prediction, resource governor, pause/resume/cancel, replanning, dynamic task discovery and escalation pass P12-G1..G20; evidence: docs/progress/phase-12-completion.md (+ .json) |
| P13 | Advanced Edit Engine | COMPLETE | P13-WP01..P13-WP13 ACCEPTED; typed edit strategies, preconditions, transaction journal, rollback/recovery, formatter detection, AST/LSP fallbacks, strategy metrics and durable edit persistence pass P13-G1..G10; evidence: docs/progress/phase-13-completion.md (+ .json) |
| P14 | Verification & Evidence Engine | COMPLETE | P14-WP01..P14-WP14 ACCEPTED; verification profiles, command detection, mechanical gates, test registry/selection, evidence manifests, requirement links, independent verifier, adversarial/tampering checks, freshness invalidation, integration verification, final audit and completion gate pass P14-G1..G14; evidence: docs/progress/phase-14-completion.md (+ .json) |
| P15 | Browser Runtime & Visual Verification | COMPLETE | P15-WP01..P15-WP11 ACCEPTED; adapter abstraction, process/session registry, page actions, DOM/a11y extraction, diagnostics, screenshots, dev-server records, responsive profiles, visual QA and crash recovery pass P15-G1..G11 plus behavioral browser flow; evidence: docs/progress/phase-15-completion.md (+ .json) |
| P16 | Skills, Hooks & MCP | COMPLETE | P16-WP01..P16-WP13 ACCEPTED; skill manifests/registry/discovery/progressive loading/importers, hook registry/dispatch/failure safety, MCP lifecycle/discovery/tool routing/trust pass P16-G1..G15; evidence: docs/progress/phase-16-completion.md (+ .json) |
| P17 | Baseline Security Platform | COMPLETE | P17-WP01..P17-WP14 ACCEPTED; threat model, unified findings, baseline adapters, redaction, grouping, triage, repair/regression and Markdown/JSON/SARIF reports pass P17-G1..G15; evidence: docs/progress/phase-17-completion.md (+ .json) |
| P18 | Advanced AppSec, Cloud & Red-Team | COMPLETE | P18-WP01..P18-WP12 ACCEPTED; scoped active authorization, environment policy, ZAP/Nuclei-compatible native adapters, safe synthetic proof, attack-path graph, Prowler-compatible cloud posture, lab adapter lifecycle, stop conditions, cleanup evidence, reports and persistence pass P18-G1..G13 through deterministic production-facing APIs; evidence: docs/progress/phase-18-completion.md (+ .json) |
| P19 | AI Security | COMPLETE | P19-WP01..P19-WP12 ACCEPTED; AI surface detection, trust-boundary graph, direct/indirect injection, RAG poisoning, tool/MCP abuse, secret leakage, excessive-agency/cross-agent fixtures, optional harness fallback statuses, common finding normalization, mitigation verification and persistence pass P19-G1..G10 through deterministic production-facing APIs; evidence: docs/progress/phase-19-completion.md (+ .json) |
| P20 | Discuss Mode | COMPLETE | P20-WP01..P20-WP08 ACCEPTED; durable discussion sessions/messages/decisions/plans, repo-grounded context answers, read-only policy, routing, research fallback, decision/plan promotion and mission draft pass P20-G1..G7; evidence: docs/progress/phase-20-completion.md (+ .json) |
| P21 | Design Studio | COMPLETE | P21-WP01..P21-WP14 ACCEPTED; product analysis, DesignBrief, DesignGrammar, DESIGN_STATE, anti-slop critic, UI tasks, preview iteration, visual/responsive/a11y/functional QA, reference principles and DOM-source prototype pass P21-G1..G12; evidence: docs/progress/phase-21-completion.md (+ .json) |
| P22 | Minimal Desktop Product Experience | COMPLETE | P22-WP01..P22-WP18 ACCEPTED; desktop/session projection, project open, goal composition, mission status, activity/changes/details, Discuss/Design/Security routing, settings, notifications/sounds, daemon reconnect and accessibility/theme state pass P22-G1..G18; evidence: docs/progress/phase-22-completion.md (+ .json) |
| P23 | Resource, Token & Cost Optimization | COMPLETE | P23-WP01..P23-WP12 ACCEPTED; telemetry, token/cost accounting, resource governor, adaptive concurrency, local model/LSP/index policies, context optimization, compression tracking, cost-aware routing and reports pass P23-G1..G7; evidence: docs/progress/phase-23-completion.md (+ .json) |
| P24 | Chaos Engineering & Recovery Validation | COMPLETE | P24-WP01..P24-WP10 ACCEPTED; deterministic chaos catalog, seeded repetition, provider/model/worker/process/edit/daemon/DB/resource fault scenarios, recovery oracle, state-equivalence validation and durable reliability reports pass P24-G1..G25; evidence: docs/progress/phase-24-completion.md (+ .json) |
| P25 | AgentCode Dogfooding | COMPLETE | P25-WP01..P25-WP12 ACCEPTED; self-repository dogfood mission harness, findings, proposals, normal ChangeSet/verification flow, adversarial dogfood scenarios, metrics and durable feedback reports pass P25-G1..G14; evidence: docs/progress/phase-25-completion.md (+ .json) |
| P26 | Security & Licensing Hardening | COMPLETE | P26-WP01..P26-WP12 ACCEPTED; threat model, boundary campaigns, prompt-injection/secret/privacy/MCP trust campaigns, dependency/license/SBOM/provenance closure, data retention review, signing prerequisites and self-red-team blocker report pass P26-G1..G8 and P26-L1..L6; evidence: docs/progress/phase-26-completion.md (+ .json) |
| P27 | Packaging, Update & Release Engineering | COMPLETE | P27-WP01..P27-WP12 ACCEPTED; release artifact/build/update records, packaging layout, daemon lifecycle, app data paths, migration/upgrade records, managed-tool checksum validation, first-run checklist, diagnostics redaction, reset/rollback and signing pipeline prerequisites pass P27-G1..G11; evidence: docs/progress/phase-27-completion.md (+ .json) |
| P28 | V1 Release Candidate | NOT_STARTED | |
| P29 | V1 Release | NOT_STARTED | |

## Work Package Records

Per-WP records: `docs/progress/workpackages/WP-<phase>-<wp>.md` (canonical
WorkPackageRecord fields per Doc 11 H4) — created for every WP started.

## Phase Completion Rule

A phase becomes COMPLETE only when:
1. all its Work Packages are ACCEPTED,
2. the Doc 10 phase gates pass with recorded evidence,
3. a completion package exists (docs/progress/phase-NN-completion.md + .json),
4. the failure/understanding check passes where Doc 10 requires it.

Evidence wins; file existence alone never marks a phase complete.
