# AgentCode
# 09 — Master Project Roadmap

**Document Status:** V1 — Master Roadmap Locked for Initial Implementation + Hardening Revision 3  
**Revision:** 3 — Implementation-Grade Master Engineering Roadmap  
**Date:** 19 August 2026  
**Project:** AgentCode  
**Document Type:** Master Engineering Roadmap  
**Role:** Primary project-execution roadmap, dependency map, milestone specification and implementation sequencing authority

**Companion Documents:**
- `10 — Success Definition & Acceptance Gates`
- `11 — Phase-Wise Implementation Playbook`

**Depends On:**
- `01 — Model, Provider, Routing & Reliability Architecture`
- `02 — Code Intelligence, Context & Persistent Memory Architecture`
- `03 — Autonomy Kernel & Agent Runtime Architecture`
- `04 — Tool, Edit, Git, Sandbox, Skills & Hooks Architecture`
- `05 — Verification, Security & Red-Team Architecture`
- `06 — Design Studio & Product UX Architecture`
- `07 — Implementation & OSS Extraction Blueprint`
- `08 — Product Requirements Document`

**Primary Development Reference Root:**

```text
/Volumes/T7 Shield/GitHub-Repos-dependency
```

---

# 1. Purpose

This document defines the complete engineering journey from the current architecture-only state to a usable, verified AgentCode V1.

It is intentionally much more than a phase list.

For every major stage, this roadmap defines:

- why the phase exists;
- which earlier capabilities it requires;
- which later capabilities depend upon it;
- what engineering work actually happens inside the phase;
- which open-source implementations must be inspected first;
- which mechanisms should be prototyped before integration;
- what persistent artifacts are created;
- what test fixtures are required;
- what failure scenarios must be tested;
- which work may occur in parallel;
- which interfaces should become stable afterward;
- what evidence proves the phase is real;
- what conditions prevent the phase from being called complete;
- what state is handed to the following phase.

The roadmap must remain usable if:

```text
the original architect is unavailable

a new developer joins the project

a different coding model takes over

development pauses for months

one implementation agent fails

reference repositories change upstream

implementation differs slightly from early assumptions
```

A competent engineer or coding agent reading Docs 01–09 should understand:

```text
what AgentCode is

why it is architected this way

what must be built

what order to build it in

which dependencies are fundamental

what not to reinvent

what constitutes real progress

what must never be faked
```

Doc 10 converts this roadmap into exact success gates.

Doc 11 converts each phase into even more operational implementation procedures, including exact substeps, recommended task decomposition, verification order and phase handoff instructions.

---

# 2. The Central Roadmap Principle

AgentCode must be built **from durable foundations outward**.

A common failure mode in AI developer-tool projects is to start with:

```text
chat UI
   ↓
LLM API
   ↓
shell
   ↓
file editing
```

and then gradually bolt on:

```text
memory

multi-agent features

security

verification

provider routing

recovery
```

afterward.

That produces a visually impressive agent whose fundamental state still lives inside an LLM conversation.

AgentCode must take the opposite route.

The foundational order is:

```text
ARCHITECTURAL TRUTH
        ↓
OSS KNOWLEDGE
        ↓
DURABLE LOCAL STATE
        ↓
RELIABLE MODEL ACCESS
        ↓
SAFE TOOL EXECUTION
        ↓
CAPABLE SINGLE WORKER
        ↓
ISOLATED CODE CHANGES
        ↓
DEEP REPOSITORY UNDERSTANDING
        ↓
PERSISTENT MEMORY
        ↓
TOKEN-EFFICIENT CONTEXT
        ↓
DURABLE MULTI-AGENT AUTONOMY
        ↓
ADVANCED EDITING
        ↓
INDEPENDENT VERIFICATION
        ↓
REAL BROWSER EXECUTION
        ↓
EXTENSIBILITY
        ↓
SECURITY
        ↓
DESIGN STUDIO
        ↓
POLISHED PRODUCT UX
        ↓
OPTIMIZATION
        ↓
CHAOS TESTING
        ↓
DOGFOODING
        ↓
HARDENING
        ↓
RELEASE
```

Every later capability should reuse the same core runtime.

Security must not create a second agent runtime.

Design Studio must not create a second browser/runtime stack.

Discuss Mode must not create a second repository-indexing system.

The desktop UI must not become another source of mission state.

---

# 3. What This Roadmap Optimizes For

The roadmap prioritizes the following sequence:

```text
correctness of foundations
>
durability
>
engineering capability
>
verification
>
specialized features
>
visual polish
```

This does **not** mean UX, Security or Design Studio are unimportant.

It means they become substantially more powerful when they are built on top of:

```text
one durable Kernel

one Tool Broker

one Code Intelligence system

one Context Engine

one verification/evidence system
```

instead of being separate prototypes.

A smaller AgentCode that can reliably:

```text
resume after failure

understand a repository

edit several files correctly

run tests

switch provider

continue across model replacement

refuse false completion
```

is more valuable than a larger product containing many impressive but shallow features.

---

# 4. Source-of-Truth Hierarchy During Development

Implementation decisions follow this order:

```text
1. Current explicit user/project decision

2. Core architecture Docs 01–08

3. Approved ADRs

4. This Master Roadmap

5. Doc 10 Success Gates

6. Doc 11 Phase Playbook

7. Current implementation

8. OSS extraction reports

9. Reference repositories
```

If a donor repository suggests an architecture incompatible with AgentCode:

```text
AgentCode architecture wins.
```

If implementation evidence later proves an architecture decision wrong:

```text
create ADR
→ update architecture
→ update roadmap/playbook
→ continue
```

Do not silently diverge.

---

# 5. Roadmap Macro-Phases

AgentCode V1 development is divided into 30 phases:

```text
PHASE 0   Project Governance & Architecture Freeze

PHASE 1   OSS Extraction & Evidence Collection

PHASE 2   Repository Skeleton & Development Infrastructure

PHASE 3   Background Daemon & Persistent Kernel Foundation

PHASE 4   Model Broker & OmniRoute Provider Fabric

PHASE 5   Native Tool Runtime Foundation

PHASE 6   Basic Worker Agent Loop

PHASE 7   Git, Worktrees & Checkpointing

PHASE 8   Code Intelligence Foundation

PHASE 9   Semantic Intelligence & Repository Graph

PHASE 10  Persistent Memory & Knowledge Freshness

PHASE 11  Context Engine & Token Efficiency

PHASE 12  Full Autonomy Kernel & Multi-Agent Runtime

PHASE 13  Advanced Edit Engine

PHASE 14  Verification & Evidence Engine

PHASE 15  Browser Runtime & Visual Verification

PHASE 16  Skills, Hooks & MCP

PHASE 17  Baseline Security Platform

PHASE 18  Advanced AppSec, Cloud & Red-Team

PHASE 19  AI Security

PHASE 20  Discuss Mode

PHASE 21  Design Studio

PHASE 22  Minimal Desktop Product Experience

PHASE 23  Resource, Token & Cost Optimization

PHASE 24  Chaos Engineering & Recovery Validation

PHASE 25  AgentCode Dogfooding

PHASE 26  Security & Licensing Hardening

PHASE 27  Packaging, Update & Release Engineering

PHASE 28  V1 Release Candidate

PHASE 29  V1 Release
```

This is a dependency-oriented sequence, not an instruction that only one task may exist at a time.

Parallel work is permitted when interfaces and dependencies make it safe.

---

# 6. Phase Status Model

Every phase has exactly one status:

```text
NOT_STARTED

EXTRACTION

PROTOTYPE

IMPLEMENTING

INTEGRATING

VALIDATING

BLOCKED

COMPLETE
```

A phase must not be labeled `COMPLETE` because:

```text
files exist

dependencies compile

one demo works

an agent says it is finished
```

`COMPLETE` means the corresponding Doc 10 acceptance gate is satisfied and evidence has been recorded.

---

# 7. Common Milestone Structure

Most phases should move through:

```text
M0 — Preparation and design confirmation

M1 — Isolated mechanism/prototype

M2 — Real AgentCode integration

M3 — Failure handling and edge cases

M4 — Acceptance tests and benchmarks

M5 — Documentation, interface stabilization and handoff
```

A phase may add additional milestones where necessary.

---

# 8. Development Evidence Standard

Every phase must eventually produce a Phase Completion Report containing:

```text
Phase

Objective

Implemented capabilities

Not implemented / deliberately deferred

Interfaces added or changed

Tests run

Acceptance results

Benchmarks

Known limitations

Architecture deviations

ADRs created

OSS sources actually used

License status

Evidence/artifacts

Next-phase handoff
```

This report prevents development progress from becoming dependent on chat history.

---


# HARDENING REVISION 3 — MASTER ENGINEERING EXECUTION CONTRACT

Revision 2 had the right macro-sequence and broad subsystem coverage, but the roadmap still allowed too much implementation ambiguity. Many phase sections named what should exist without consistently stating what must already be true, what exact artifacts and interfaces leave the phase, how failure is handled, what can proceed in parallel, what is release-critical, and what a later implementation agent may safely assume.

Revision 3 makes those relationships explicit while preserving the 30-phase roadmap.

The core correction is:

> **A phase is not a bucket of features. It is a dependency-producing engineering contract.**

Each phase consumes stable inputs, produces stable outputs, proves failure behavior, records evidence, and creates a handoff that the next phase may depend on.

---

# H1. Roadmap Decision Classes

Every sequencing statement in this roadmap belongs to one of the following classes.

| Class | Meaning |
|---|---|
| `LOCKED_SEQUENCE` | The ordering expresses a real architecture dependency and should not change without ADR + roadmap reconciliation. |
| `LOCKED_DEPENDENCY` | A later capability cannot be trusted before the dependency exists at the required maturity. |
| `CONSTRAINED_IMPLEMENTATION_DECISION` | The roadmap fixes the required properties but an ADR/spike may choose the concrete implementation. |
| `BENCHMARK_PENDING` | Candidate mechanisms exist but the final choice/threshold must come from measured evidence. |
| `PARALLELIZABLE` | Work may overlap after named predecessor interfaces exist. |
| `BLOCKING_GATE` | Failure blocks the dependent phase/release claim. |
| `REQUIRED_V1` | Release obligation inherited from Doc 08. |
| `REQUIRED_IF_APPLICABLE` | Must work when deterministic applicability criteria are met. |
| `OPTIONAL_V1` | May ship if quality is sufficient; absence does not block V1. |
| `POST_V1` | Not part of initial V1 release obligation. |

Phase number does **not** automatically equal product importance. For example, Design Studio is implemented later because it depends on Browser, Context, Editing and Verification; it is still a `REQUIRED_V1` product capability.

---

# H2. Roadmap State Model

Roadmap phase status is a project-management state and must never be confused with:

- Kernel runtime task state from Doc 03;
- a Doc 11 Work Package lifecycle;
- a Git branch state;
- an LLM statement.

Canonical phase states remain:

```text
NOT_STARTED
EXTRACTION
PROTOTYPE
IMPLEMENTING
INTEGRATING
VALIDATING
BLOCKED
COMPLETE
```

Allowed normal progression:

```text
NOT_STARTED
→ EXTRACTION            when donor/mechanism evidence is needed
→ PROTOTYPE             for unresolved architecture/risk questions
→ IMPLEMENTING
→ INTEGRATING
→ VALIDATING
→ COMPLETE
```

A phase may move backward:

```text
VALIDATING → IMPLEMENTING
```

when acceptance exposes a defect.

`BLOCKED` means the phase cannot safely advance because of a real architectural, license, security, platform, resource or external prerequisite blocker. Ordinary compiler/test failures are engineering work, not roadmap blockers.

Only the project control process may set `COMPLETE`, and only after the relevant Doc 10 gate family has current evidence.

---

# H3. Canonical Phase Record

Every phase must have a durable machine-readable representation equivalent to:

```ts
type PhaseRecord = {
  phase_id: string
  number: number
  title: string

  status:
    | "NOT_STARTED"
    | "EXTRACTION"
    | "PROTOTYPE"
    | "IMPLEMENTING"
    | "INTEGRATING"
    | "VALIDATING"
    | "BLOCKED"
    | "COMPLETE"

  objective: string
  owner_subsystems: string[]

  product_scope: Array<
    "REQUIRED_V1"
    | "REQUIRED_IF_APPLICABLE"
    | "OPTIONAL_V1"
    | "POST_V1"
  >

  hard_predecessors: string[]
  soft_predecessors: string[]
  parallelizable_with: string[]

  prd_refs: string[]
  architecture_refs: string[]
  extraction_campaign_refs: string[]
  implementation_packet_refs: string[]

  input_interfaces: string[]
  output_interfaces: string[]
  interface_freeze_effects: string[]

  artifacts: PhaseArtifact[]
  fixtures: string[]
  benchmarks: string[]

  risks: PhaseRisk[]
  blockers: string[]

  doc10_gate_family: string
  doc11_work_package_family: string

  evidence_refs: string[]
  started_at?: string
  completed_at?: string
}
```

---

# H4. Canonical Milestone Record

Milestones exist inside a phase and are finer-grained than the phase status.

```ts
type MilestoneRecord = {
  milestone_id: string
  phase_id: string
  title: string
  objective: string

  prerequisites: string[]
  tasks_or_work_packages: string[]

  produced_artifacts: string[]
  produced_interfaces: string[]

  required_tests: string[]
  failure_tests: string[]
  benchmark_refs: string[]

  exit_conditions: string[]
  evidence_refs: string[]

  status:
    | "NOT_STARTED"
    | "READY"
    | "ACTIVE"
    | "VALIDATING"
    | "BLOCKED"
    | "COMPLETE"
}
```

The familiar M0–M5 pattern remains a default, not a requirement that every phase have exactly six milestones.

---

# H5. Roadmap Workstreams

The 30 phases can be viewed as coordinated workstreams.

```text
GOVERNANCE
  P0–P2

DURABLE RUNTIME
  P3, P6, P12, P24

INFERENCE
  P4, P23

TOOLS / GIT / EDIT
  P5, P7, P13

INTELLIGENCE / MEMORY / CONTEXT
  P8–P11

VERIFICATION
  P14, P24

BROWSER / DESIGN / UX
  P15, P21, P22

EXTENSIBILITY
  P16

SECURITY
  P17–P19, P26

DOGFOOD / RELEASE
  P25–P29
```

These workstreams permit parallel engineering without creating duplicate authorities.

---

# H6. Hard vs Soft Dependencies

### Hard dependency

A phase cannot responsibly implement its main outcome without the predecessor.

Example:

```text
Phase 14 Verification
hard-depends on
Phase 5 Tool execution
Phase 7 Git/commit identity
Phase 8/9 repository intelligence
Phase 12 requirement/task authority
```

### Soft dependency

The predecessor improves quality but need not be globally complete.

Example:

```text
Phase 8 Code Intelligence
may begin while
late Phase 6 Worker benchmarking
continues.
```

### Partial predecessor

A phase may start when a named interface from an earlier phase is frozen, even if the earlier phase still has unrelated validation work.

This is how AgentCode avoids unnecessary waterfall behavior.

---

# H7. Master Dependency Graph

The dependency backbone is:

```text
P0 Governance
 ↓
P1 relevant OSS extraction ─────────────────────────────┐
 ↓                                                      │
P2 Repository skeleton                                  │
 ↓                                                      │
P3 Daemon + durable state                               │
 ├──────────────→ P4 Provider fabric                    │
 └──────────────→ P5 Tool runtime                       │
                    ↓                                   │
                  P6 Worker                             │
                    ↓                                   │
                  P7 Git/worktrees                      │
                    ↓                                   │
                  P8 Code intelligence                  │
                    ↓                                   │
                  P9 Semantic graph                     │
                    ↓                                   │
                 P10 Knowledge                          │
                    ↓                                   │
                 P11 Context                            │
                    ↓                                   │
                 P12 Full Kernel/autonomy ←──────── P4  │
                    ↓                                   │
                 P13 Edit Engine                        │
                    ↓                                   │
                 P14 Verification                       │
                    ├────────→ P15 Browser               │
                    ├────────→ P16 Skills/Hooks/MCP      │
                    └────────→ P17 Security baseline     │
                                  ↓                      │
                                P18 Advanced security    │
                                  ↓                      │
                                P19 AI security          │
                                                         │
P9 + P11 + P12 ───────────────→ P20 Discuss             │
P13 + P14 + P15 ──────────────→ P21 Design Studio       │
P3 + P12 + P14 + P20 + P21 + P17 ─→ P22 Product UX     │
                                                         │
P22 + all required V1 subsystems ─→ P23 Optimization    │
                                      ↓                  │
                                    P24 Chaos            │
                                      ↓                  │
                                    P25 Dogfood          │
                                      ↓                  │
                                    P26 Security/license │
                                      ↓                  │
                                    P27 Packaging        │
                                      ↓                  │
                                    P28 RC               │
                                      ↓                  │
                                    P29 V1               │
```

This graph is intentionally simplified. Phase contracts later in this revision define the exact hard/soft edges.

---

# H8. Critical Path to First Real Autonomous Mission

The **first internally credible autonomous end-to-end mission** does not require every V1 feature.

Critical path:

```text
P0
Governance / contracts
  ↓
P1
approved extraction packets for provider/runtime/tools/Git/code-intel
  ↓
P2
buildable repository + fixtures
  ↓
P3
durable daemon/mission state
  ↓
P4
at least one real + one alternate inference route
  ↓
P5
safe filesystem/search/shell/test tools
  ↓
P6
single capable Worker
  ↓
P7
worktree + checkpoint
  ↓
P8
basic persistent code intelligence
  ↓
P10
minimal durable project/task handoff state
  ↓
P11
focused context packs
  ↓
P12
Kernel scheduler/recovery
  ↓
P14
independent evidence-backed completion
  ↓
thin internal mission UI / harness
```

Phase 9 semantics, Phase 13 advanced editing and Phase 15 browser improve mission quality substantially, but the project should deliberately prove the core autonomy loop before trying to finish every specialization.

---

# H9. Product Milestone Map

## MVP-0 — Runtime Mechanism Proof

Purpose:

> Prove that AgentCode is more than a UI around an LLM.

Requires sufficient completion of:

```text
P0 Governance
P1 relevant provider/runtime/tool extraction
P2 repository skeleton
P3 durable daemon
P4 provider fabric minimum
P5 tool runtime minimum
P6 single Worker
```

MVP-0 demonstration:

```text
fixture bug
→ Worker edits
→ real test
→ provider route usable
→ mission/task state stored outside transcript
```

MVP-0 is not user release scope.

---

## Internal Alpha — Autonomous Core

Requires:

```text
P7 worktrees/checkpoints
P8 code intelligence foundation
P9 core LSP/graph where supported
P10 persistent knowledge
P11 context engine
P12 full Kernel/recovery
P13 transactional editing core
P14 verification/final gate
thin mission UI
```

Demonstrates:

```text
longer mission
worker replacement
provider failure
persistent context
verification repair
UI/daemon separation
```

---

## Feature-Complete Alpha — V1 Capability Surface

Requires the required V1 capability families:

```text
P15 browser
P16 skills/hooks/MCP
P17 Security baseline
P20 Discuss
P21 Design Studio core
P22 product UI
```

and applicable baseline work from P18/P19 when the capability is part of the V1 claim.

At this point all required product modes exist through one runtime.

---

## Beta — Reliability and Dogfood Quality

Requires:

```text
P23 optimization
P24 chaos
P25 dogfood
substantial P26 hardening
packaging preview from P27
```

Goal:

```text
repeated usefulness on AgentCode itself
+
8 GB viability
+
failure recovery
```

---

## Release Candidate

Requires:

```text
P26 complete
P27 packaging complete
P28 RC gates
```

No unresolved release blocker.

---

## V1

Requires:

```text
Doc 10 global release gate
+
P29 release procedure
```

A feature present in Alpha but failing its V1 required gate is still a release blocker.

---

# H10. Interface Freeze Points

An interface freeze does not mean “never change.” It means changes now require explicit versioning/migration and dependent test updates.

| Freeze point | Interface family |
|---|---|
| after P3 | IDs, core event envelope, mission/project identity, local protocol versioning |
| after P4 | Broker ↔ provider-fabric candidate/request/result contracts |
| after P5 | ToolDescriptor, ToolRequest, ToolResult, ToolError, process identity |
| after P7 | Worktree, checkpoint, Git evidence and ownership/fencing contracts |
| after P9 | Repository-intelligence query/graph provenance contracts |
| after P11 | Knowledge/context manifest and context-budget contracts |
| after P12 | Mission/task/attempt/lease/scheduler/human-request contracts |
| after P13 | ChangeSet/EditOperation transaction contract |
| after P14 | Verification profile/evidence/requirement-verification/final-audit contracts |
| after P17 | Threat model/attack surface/finding/security-policy contracts |
| after P21 | DesignBrief/DesignGrammar/DesignIteration contracts |
| after P22 | Desktop ↔ daemon product protocol and route/view models |

A post-freeze breaking change requires:

```text
schema/API version bump
migration
affected phase regression tests
updated Docs
```

---

# H11. Database and Persistent-Schema Roadmap

Do not attempt to create the entire final database in Phase 3.

### P3 — Control-plane core

```text
projects/repositories
missions
mission_contract_versions
requirements
plans
tasks
task_dependencies
events
human_requests
settings metadata
daemon metadata
```

### P6/P7/P12 — Execution/runtime

```text
agent_sessions
task_attempts
workers
leases
checkpoints
worktrees
resource_locks
messages/mailboxes
blackboard entries
budget reservations
```

### P8/P9 — Derived repository intelligence

Prefer separate logical ownership even if SQLite files are shared:

```text
files
symbols
symbol_edges
imports
packages
test mappings
index generations
language/LSP capability state
```

### P10/P11 — Knowledge/context

```text
knowledge facts
fact evidence refs
decisions
context snapshots
context manifests
context-cache metadata
```

### P13 — Edit journal

```text
change_sets
edit_operations
preimages
edit journal/events
```

### P14 — Verification/evidence

```text
verification_profiles
verification_runs
requirement_verifications
evidence_records
evidence_manifests
test_registry
test_runs
verification_findings
```

### P17–P19 — Security

```text
security_policy_versions
threat_models
attack_surfaces
scanner_invocations
finding_instances
security_findings
attack_paths
validation_sessions
redteam_sessions
suppressions
risk_acceptances
security_reports
```

### P21 — Design

```text
design_briefs
design_grammars
design_constraints
design_iterations
visual_findings
preview metadata
```

Every migration family requires:

```text
upgrade fixture
downgrade/rollback policy where practical
backup-before-risky-migration
schema-version assertion
crash/interruption test
```

---

# H12. Build and Repository Boundary Direction

Exact package layout is an ADR, but the likely architecture direction remains:

```text
Rust
  durable daemon
  Kernel
  process/tool execution
  Git/worktree critical path
  index-critical native services where justified

TypeScript
  Tauri/React desktop
  UI state projection
  provider/OmniRoute layer where upstream ecosystem fits
  browser orchestration where appropriate

Python / external runtimes
  only for optional tools or ecosystems that require them

External binaries
  ripgrep/scanners/tooling where mature upstream engines should be wrapped
```

Cross-layer rule:

```text
UI may depend on protocol contracts.
Kernel must not import UI.
Security adapters may depend on Tool Broker interfaces.
Tool Broker must not depend on Security UI.
Provider fabric must not depend on Kernel mission semantics.
```

Architecture-boundary linting should enforce these rules where languages/build systems allow.

---

# H13. Fixture and Benchmark Registry

Canonical fixture IDs should be stable.

| Fixture | Introduced | Primary use |
|---|---:|---|
| `FIX-TS-SMALL` | P2 | tools, Worker, code intelligence |
| `FIX-NEXT-FULLSTACK` | P2 | full-stack graph, browser, Design |
| `FIX-PY` | P2 | poly-language Worker/LSP |
| `FIX-RUST` | P2 | Rust parser/LSP/build |
| `FIX-GO` | P2 | Go parser/LSP/build |
| `FIX-POLYGLOT` | P2 | mixed-language graph/context |
| `FIX-MONOREPO` | P2/P8 | package graph, large indexing |
| `FIX-EDIT-CONFLICT` | P7/P13 | worktree/edit conflict/recovery |
| `FIX-BROWSER-DYNAMIC` | P15 | dev server/browser/recovery |
| `FIX-VERIFY-FALSE-DONE` | P14 | missing wiring/test tampering |
| `FIX-SEC-VULNERABLE` | P17 | AppSec baseline |
| `FIX-SEC-WEB` | P18 | DAST/validation |
| `FIX-CLOUD-LAB` | P18 | authorized cloud lab |
| `FIX-AI-SEC` | P19 | prompt/tool/RAG/MCP abuse |
| `FIX-DESIGN-SLOP` | P21 | anti-slop/responsive/accessibility |
| `FIX-AGENTCODE-SELF` | P25 | real dogfood |

Each fixture must record:

```text
fixture version
ground truth
expected commands
expected defects
expected successful behavior
seed data
network assumptions
cleanup
```

---

# H14. Test Environment Matrix

### `ENV-LOCAL-DETERMINISTIC`

Used in CI and normal local development.

```text
mock providers
fixture repos
no production credentials
deterministic clocks where useful
```

### `ENV-MAC8-REAL`

Required real 8 GB Apple Silicon machine.

Used for:

```text
memory
UI
browser
local models
real process lifecycle
packaging
```

### `ENV-LIVE-PROVIDER`

Small controlled smoke tests.

Used for:

```text
real auth
catalog
streaming
tool call
provider failover compatibility
```

Live-provider success never replaces deterministic mock tests.

### `ENV-BROWSER`

Pinned browser/runtime version for deterministic browser evidence.

### `ENV-NETWORK-RESTRICTED`

Tests:

```text
offline
DNS/network failure
provider outage
degraded operation
```

### `ENV-SECURITY-LAB`

Disposable authorized targets only.

### `ENV-CLOUD-LAB`

Optional/required-if-applicable validation environment with explicit cost/scope/cleanup.

---

# H15. Phase Completion Report

Canonical human path:

```text
docs/progress/phase-XX-completion.md
```

Machine-readable companion:

```text
docs/progress/phase-XX-completion.json
```

Logical schema:

```ts
type PhaseCompletionReport = {
  phase_id: string
  phase_version: number

  validated_commit: string
  environment_fingerprints: string[]
  tool_versions: Record<string, string>

  objectives: string[]
  implemented: string[]
  deliberately_deferred: string[]

  prd_refs: string[]
  architecture_refs: string[]
  extraction_refs: string[]

  interfaces_added: string[]
  migrations_added: string[]

  tests: string[]
  failure_tests: string[]
  benchmarks: string[]

  evidence_manifest_ref: string

  known_limitations: string[]
  accepted_deviations: string[]
  adr_refs: string[]

  license_status: string[]
  security_status: string[]

  blockers_remaining: string[]

  next_phase_handoff: string[]

  status:
    | "PASS"
    | "FAIL"
    | "BLOCKED"
}
```

A prose completion report without commit/environment/evidence identity is insufficient.

---

# H16. Roadmap Change Control

### Requires ADR + Docs 08–11 reconciliation

```text
phase sequence changes that alter dependencies
new foundational framework
new major fork
new source-of-truth database
new release-critical mode
removal/deferment of REQUIRED_V1 capability
major trust/sandbox relaxation
new production mutation authority
```

### May change inside a phase without cross-doc rewrite

```text
internal helper function
module naming
minor library choice behind stable interface
benchmark-tuned threshold
non-breaking refactor
```

provided phase contracts and acceptance remain true.

---

# H17. Roadmap Risk Register

| Risk | Failure mode | Primary mitigation phases |
|---|---|---|
| architecture drift | autonomous agents invent incompatible cores | P0, P1, all interface freezes |
| extraction paralysis | months spent reading donors | P1 implementation packets + stop-collecting rule |
| premature UI | polished shell simulates nonexistent runtime | thin dev harness early, product UX P22 |
| scope explosion | every donor feature becomes V1 | Doc08 scope matrix, P0/P28 |
| provider volatility | one provider quietly becomes mandatory | P4, P24, P28 |
| resource pressure | browser/LSP/local model/workers thrash 8 GB Mac | P12, P23, P28 |
| verification theater | many green checks miss original goal | P14, P24, P25 |
| flaky-test contamination | rerun-until-green hides regression | P14 |
| dependency/license blocker | late release discovers incompatible source | P0/P1/P26 |
| sandbox weakness | unattended agent writes outside allowed scope | P5/P24/P26 |
| prompt injection | repo/web/MCP content changes policy | P5/P16/P19/P26 |
| security scope risk | active validation crosses authorized target | P18/P26 |
| generic Design output | Design mode becomes CSS generator | P21/P25 |
| packaging delay | dev build works but packaged daemon/tooling fails | P27/P28 |
| stale knowledge | persistent memory becomes confidently wrong | P10/P14 |
| false progress | roadmap/UI reports done before evidence | P14/P22/P28 |

---

# H18. What Can Begin Immediately

After the hardened Docs 01–09 exist:

### Start immediately

```text
Phase 0 governance repository setup
Phase 1 donor catalog/license/source extraction
fixture planning
benchmark environment capture
ADRs for language split / IPC / OmniRoute fork / data layout
```

### Phase 2 may start when

```text
critical language/build/IPC/package-boundary ADRs are sufficiently decided
+
foundational extraction packets for chosen primitives exist
```

Do **not** wait for all optional scanner, cloud, AI-security or direct-visual-edit extraction work.

### Individual subsystems may start early

Example:

```text
Tree-sitter adapter implementation
```

may begin once the Code Intelligence implementation packet and license decision are approved, while unrelated cloud-security extraction continues.

The roadmap is dependency-driven, not a demand for serial idle time.

---

# H19. Post-V1 Boundary

Do not expand V1 indefinitely to include:

```text
Windows release if not explicitly elevated
remote daemon hosting
team collaboration
shared organizational workspaces
plugin marketplace
learned/bandit routing
continuous enterprise monitoring
universal direct visual manipulation
enterprise governance/policy fleet
mobile companion
```

These remain post-V1 backlog themes unless a product-scope amendment explicitly changes the release.

---

# H20. Phase-to-Product Scope Traceability

| Product capability | First serious implementation | Integration maturity | Release verification |
|---|---:|---:|---:|
| durable Goal mission | P3/P6/P12 | P14/P22 | P28/P29 |
| provider independence | P4 | P12/P23 | P24/P28 |
| safe tools | P5 | P6/P12 | P24/P26/P28 |
| worktrees/recovery | P7 | P12/P13 | P24/P28 |
| code intelligence | P8/P9 | P11/P12 | P25/P28 |
| persistent memory | P10 | P11/P12 | P24/P25 |
| context efficiency | P11 | P12/P23 | P25/P28 |
| multi-worker autonomy | P12 | P14 | P24/P25/P28 |
| transactional editing | P13 | P14 | P24/P28 |
| independent verification | P14 | P17/P21 | P24/P25/P28 |
| browser automation | P15 | P21/P22 | P25/P28 |
| skills/hooks/MCP | P16 | P20/P21/P17 | P25/P26/P28 |
| Security baseline | P17 | P18/P19 | P25/P26/P28 |
| applicable Web/cloud validation | P18 | P26 | P28 when claimed/applicable |
| applicable AI security | P19 | P26 | P28 when applicable |
| Discuss Mode | P20 | P22 | P25/P28 |
| Design Studio core | P21 | P22 | P25/P28 |
| minimal desktop UX | thin harness earlier; product P22 | P25 | P28 |
| 8 GB viability | P12 early controls, P23 deep optimization | P24/P25 | P28 |
| packaging | P27 | P28 | P29 |

---

# H21. Phase-to-Extraction Dependencies

Doc 07 extraction reports are consumed selectively.

| Phase | Required extraction before implementation |
|---|---|
| P3 | `02_AGENT_RUNTIME_EXTRACTION`, relevant persistence/daemon donor findings |
| P4 | `01_PROVIDER_FABRIC_EXTRACTION` |
| P5 | `03_TOOLS_EDITING_EXTRACTION`, `07_SANDBOX_PERMISSIONS_EXTRACTION` |
| P6 | `02_AGENT_RUNTIME_EXTRACTION`, `03_TOOLS_EDITING_EXTRACTION` |
| P7 | `06_GIT_WORKTREES_EXTRACTION` |
| P8/P9 | `04_CODE_INTELLIGENCE_EXTRACTION` |
| P10/P11 | `05_CONTEXT_MEMORY_EXTRACTION` |
| P13 | `03_TOOLS_EDITING_EXTRACTION` |
| P15 | `09_BROWSER_QA_EXTRACTION` |
| P16 | `08_SKILLS_HOOKS_MCP_EXTRACTION` |
| P17 | `11_APPSEC_EXTRACTION` |
| P18 | `12_CLOUD_SECURITY_EXTRACTION` plus AppSec where web DAST used |
| P19 | `13_AI_SECURITY_EXTRACTION` |
| P20 | `14_PRODUCT_UX_EXTRACTION` plus context/runtime packets |
| P21 | `10_DESIGN_STUDIO_EXTRACTION`, browser packet |
| P22 | `14_PRODUCT_UX_EXTRACTION` |
| P26/P27 | `00_REFERENCE_CATALOG_AND_LICENSES` current production-dependency review |

A phase does not need unrelated extraction campaigns to be complete.

---


# HARDENED PHASE CONTRACTS

The following contracts are authoritative overlays for Phases 0–29. They do not replace the detailed subphase descriptions already present later in this roadmap; they specify what each phase must consume, produce and prove.

---

# HC-P00 — PHASE 0 CONTRACT: PROJECT GOVERNANCE & ARCHITECTURE FREEZE

## Purpose and Why It Exists Now

Phase 0 prevents the first implementation agents from turning unresolved architectural questions into accidental permanent structure.

Its goal is not to freeze every helper function. It freezes **authority, product scope, compatibility boundaries and evidence rules**.

The central output is a development environment in which a future Worker can determine:

```text
which document is authoritative
which PRD requirements are V1
which ADRs are active
which source reuse is legal/approved
which interface it is allowed to change
which tests prove a claim
```

without relying on conversation memory.

## Hard Prerequisites

```text
hardened Docs 01–09 available
project repository location chosen
reference-library location known
primary 8 GB Mac benchmark environment identified
```

## Engineering Slices

### Governance registry

Create and validate:

```text
docs/registry/documents.*
docs/registry/requirements.*
docs/adr/
docs/progress/
docs/reference/
docs/extraction/
```

The registry must detect:

```text
duplicate document IDs
superseded doc used as current
duplicate PRD IDs
missing dependency references
```

### Product-scope ingestion

Translate Doc 08's V1 scope matrix into machine-readable data.

At minimum each capability records:

```text
REQUIRED_V1
REQUIRED_IF_APPLICABLE
OPTIONAL_V1
POST_V1
```

No roadmap implementation phase may override the scope field.

### ADR framework

Initial ADR set must resolve or explicitly schedule:

```text
core language/package split
SQLite ownership/layout
local IPC
OmniRoute fork
worktree placement
AgentCode state directories
LSP process model
Tauri/desktop stack
external binary management
sandbox implementation
logging/event encoding
local protocol security
```

### Engineering conventions

Lock conventions for:

```text
stable ID format
error code namespaces
structured logging
event correlation
configuration precedence
feature flags
test naming
fixture naming
migration naming
benchmark environment fingerprints
artifact retention
secret handling
```

### AgentCode self-threat model seed

Before building an unattended shell-capable agent, document early threats:

```text
workspace escape
secret leakage
prompt injection
malicious dependency/tool
MCP/skill privilege escalation
remote mutation
provider-data exposure
```

This seed is later expanded in P26.

## Canonical Artifacts

```text
docs/registry/documents.json
docs/requirements/v1_requirements.yaml
docs/adr/ADR-*.md
docs/reference/AGENTCODE_REFERENCE_CATALOG.md
docs/progress/roadmap_state.json
docs/security/agentcode-threat-model-seed.md
docs/testing/fixture-registry.yaml
docs/testing/environment-registry.yaml
```

## Failure / Edge Cases to Prove

```text
two active docs claim same ID
PRD requirement removed without supersession
migration generated with duplicate version
unknown secret appears in fixture
ADR refers to superseded architecture
feature flag changes release scope without PRD update
```

## Parallel Work

`PARALLELIZABLE`:

```text
reference catalog verification
fixture design
CI skeleton
initial ADR drafting
```

after ownership conventions exist.

## No-Go Conditions

Phase may not complete while:

```text
release scope is ambiguous
major source-of-truth ownership is unresolved
no ADR process exists
direct OSS copying can occur without license review
no deterministic fixture strategy exists
```

## Exit Evidence

Phase completion report must prove the registries validate, the initial ADR set exists, PRD scope can be queried, development secret/fixture rules are active, and architecture deviations have a defined process.

## Handoff

P1 receives governance/extraction rules.

P2 receives language/build/package/IPC decisions sufficient to create repository structure.

---

# HC-P01 — PHASE 1 CONTRACT: OSS EXTRACTION & EVIDENCE COLLECTION

## Purpose

Turn the flat donor library into implementation packets, not a pile of Git clones.

The phase succeeds when implementation teams no longer need to ask:

```text
Which repo should I read?
Where is the actual mechanism?
Can we reuse it?
What must we not inherit?
```

for foundational V1 primitives.

## Hard Prerequisites

```text
P0 extraction governance
canonical catalog schema
license-review process
reference root accessible for actual source verification
```

## Canonical Campaigns

Use hardened Doc 07 names:

```text
00_REFERENCE_CATALOG_AND_LICENSES
01_PROVIDER_FABRIC
02_AGENT_RUNTIME
03_TOOLS_EDITING
04_CODE_INTELLIGENCE
05_CONTEXT_MEMORY
06_GIT_WORKTREES
07_SANDBOX_PERMISSIONS
08_SKILLS_HOOKS_MCP
09_BROWSER_QA
10_DESIGN_STUDIO
11_APPSEC
12_CLOUD_SECURITY
13_AI_SECURITY
14_PRODUCT_UX
```

### Critical name cleanup

The canonical catalog is:

```text
AGENTCODE_REFERENCE_CATALOG.md
```

not `OSS_REFERENCE_CATALOG.md`.

The ambiguous local `skills` clone must be reverified. If it is Trail of Bits Skills, normalize catalog identity/path and keep Letta Skills separately as `letta-skills`.

## Engineering Slices

### P0/P1 donor verification

For every selected foundational donor:

```text
remote
HEAD SHA
branch
dirty state
license files
maintenance status
```

must be recorded.

### Source-level extraction

For a mechanism to become implementation-ready, record:

```text
entrypoint
exact source paths/symbols
control flow
data structures
failure paths
upstream tests
platform assumptions
license status
TAKE/ADAPT/WRAP/STUDY/REJECT
AgentCode destination
```

### Adoption decisions

Separate:

```text
what upstream does
```

from:

```text
what AgentCode will do.
```

### Implementation packets

P2+ Work Packages consume implementation packets containing only the chosen mechanism, exact source refs, normalized AgentCode interface, prohibited inheritance, tests and license obligations.

## Canonical Artifacts

```text
docs/reference/AGENTCODE_REFERENCE_CATALOG.md
docs/reference/agentcode_reference_catalog.json
docs/reference/OSS_LICENSE_MATRIX.md
docs/extraction/00_... through 14_...
docs/extraction/decisions/
docs/extraction/packets/
THIRD_PARTY_NOTICES.md skeleton
third_party_manifest.json
```

## Phase Exit Logic

Global Phase 1 may remain `IN_PROGRESS` while P2/P3 work begins for a subsystem whose extraction packet is already `IMPLEMENTATION_READY`.

This avoids extraction paralysis.

## No-Go

Do not let an implementation agent use:

```text
README summary
memory from previous chat
repository name alone
```

as source-level proof for a foundational reuse decision.

---

# HC-P02 — PHASE 2 CONTRACT: REPOSITORY SKELETON & DEVELOPMENT INFRASTRUCTURE

## Purpose

Build the development substrate that lets multiple subsystems evolve for years without becoming one application package with circular ownership.

## Hard Prerequisites

Required decisions sufficiently resolved:

```text
language/package split
local IPC direction
database/migration framework
OmniRoute placement
desktop stack
external binary policy
```

Relevant P1 implementation packets must exist for primitives entering the skeleton.

## Engineering Slices

### Workspace/package layout

Recommended shape remains:

```text
apps/
core/
services/
adapters/
skills/
tests/
fixtures/
benchmarks/
docs/
```

The exact path may differ by language ecosystem, but architecture linting must prevent:

```text
desktop UI → direct DB writes
provider gateway → Kernel task state
security adapter → separate mission scheduler
```

### Build orchestration

Create reproducible root commands for:

```text
format
lint
typecheck/compile
unit tests
workspace tests
fixture tests
```

A single `validate` command should eventually run the fast baseline.

### Dependency locking

Every ecosystem uses a committed lockfile where its package manager supports one.

CI verifies manifest/lock consistency.

### Structured error/log foundations

Define:

```text
component namespace
error code
severity
retryability
correlation ID
mission/task IDs where applicable
redaction
```

### Configuration layering

One configuration subsystem resolves:

```text
compiled defaults
user config
project config
environment refs
test overrides
runtime flags
```

Individual packages must not read arbitrary environment variables throughout the codebase.

### Fixture framework

Instantiate versioned fixtures from H13.

Fixtures must be:

```text
small
deterministic
self-describing
resettable
safe to run repeatedly
```

### CI jobs

At minimum:

```text
format
lint
compile/typecheck
unit tests
migration validation
architecture-boundary lint
secret scan for repository
fixture smoke
```

## Canonical Artifacts

```text
workspace manifests/lockfiles
core package skeletons
apps/desktop skeleton
services/provider_gateway skeleton
migrations/
fixtures/
benchmarks/
docs/development/
CI workflows
architecture-boundary config
```

## Failure Tests

```text
clean clone build
missing optional tool
invalid config
duplicate migration version
fixture reset after failed test
secret injected into test fixture
cross-boundary import violation
```

## Exit Evidence

A clean checkout on the primary Mac builds and tests without requiring the donor-reference directory.

---

# HC-P03 — PHASE 3 CONTRACT: BACKGROUND DAEMON & PERSISTENT KERNEL FOUNDATION

## Purpose

Create the first non-negotiable AgentCode property:

> Mission truth exists independently of the renderer and independently of any LLM session.

## Hard Prerequisites

```text
P2 build/migrations/logging
IPC ADR
SQLite ownership ADR
stable ID/event conventions
```

## Engineering Slices

### Daemon singleton ownership

Only one daemon instance may own the active control-plane database.

Use an OS-appropriate singleton lock and stale-lock recovery.

### Core persistence

Implement only the foundational control-plane tables needed now. Do not pre-invent every later schema.

Durable entities include:

```text
project/repository
mission
mission_contract_version
requirement skeleton
task skeleton
event
human_request
daemon metadata
```

### Mission contract

Persist:

```text
immutable original_goal
active contract version
scope
policy snapshot
created/updated timestamps
```

Original goal is never overwritten by later clarification.

### SQLite policy

Lock:

```text
schema versioning
WAL if benchmark/safety supports it
single Kernel writer logical authority
busy-timeout strategy
foreign keys
transaction boundaries
migration backup
integrity checks
```

### Local protocol

Implement:

```text
version handshake
request IDs
idempotent commands
health
project registration
mission create/read/list
pause/resume/cancel skeleton
snapshot
event subscription
```

### Snapshot-then-stream

A reconnecting client obtains snapshot revision N and then events after N.

Revision gaps trigger fresh snapshot.

### Startup reconciliation

On unclean restart inspect:

```text
database version/integrity
missions not terminal
future task/lease placeholders
unclean-shutdown marker
```

P12 later adds full runtime reconciliation.

## Failure Matrix

Must test:

```text
double daemon launch
renderer disconnect
renderer reconnect
duplicate create-mission request
daemon killed after transaction commit
daemon killed before commit
migration interrupted
database busy
invalid protocol version
stale snapshot/event gap
power-loss simulation around durable state
```

## Interface Freeze

After P3, changing core IDs/event envelope/local protocol semantics requires versioning/migration.

## Exit Evidence

Mission and original goal survive:

```text
UI close
UI crash
daemon restart
```

without any model call.

---

# HC-P04 — PHASE 4 CONTRACT: MODEL BROKER & OMNIROUTE PROVIDER FABRIC

## Purpose

Make inference a replaceable resource before autonomous work depends on it.

## Hard Prerequisites

```text
P3 durable daemon/event identity
01_PROVIDER_FABRIC implementation packet
OmniRoute fork ADR/license clearance
secret-reference policy
```

## Engineering Slices

### Fork discipline

Record:

```text
upstream remote
AgentCode origin
base SHA
patch list
compatibility tests
upstream update policy
```

### Provider fabric entities

Implement normalized:

```text
Provider
ProviderConnection
QuotaDomain
ModelIdentity
CapabilityProfile
HealthObservation
CircuitBreaker
```

### Kernel/Broker boundary

Model Broker consumes:

```text
role
task type
risk
complexity
context need
tool/vision/structured-output requirement
privacy
budget
```

OmniRoute consumes normalized candidate/provider requirements and resolves real routes.

Mission semantics do not leak into provider adapters.

### Credential handling

Only secret references persist.

Provider adapters resolve secret values at execution boundary through approved secret path.

### Deterministic provider harness

Build mock providers capable of returning:

```text
success
429
quota exhausted
auth failure
timeout
stream interruption
malformed tool call
context limit
provider 5xx
```

### Live smoke

Small tests validate:

```text
authentication
catalog/model identity
streaming
tool calling where supported
usage/cost metadata
```

Live tests are never the only provider test.

### Local models

Register local runtime dynamically.

Do not hardcode one model as mandatory; the architecture supports lightweight local roles if available.

### Spend/budget

Implement:

```text
mission spend
reservation/reconciliation
daily/user limit
paid reserve
budget denial
```

## Failure / Edge Cases

```text
two keys share same quota domain
provider metadata lies about model identity
model disappears
stream dies after partial output
all free routes unavailable
paid route denied
local model OOM
catalog stale
```

## Exit Evidence

A caller issues one normalized inference request and survives at least one injected provider/connection failure without redesigning caller logic.

---

# HC-P05 — PHASE 5 CONTRACT: NATIVE TOOL RUNTIME FOUNDATION

## Purpose

Give future Workers real engineering capability through one safe, typed authority.

## Hard Prerequisites

```text
P2 process/build substrate
P3 IDs/events
03_TOOLS_EDITING packet
07_SANDBOX_PERMISSIONS packet
```

## Engineering Slices

### Canonical contracts

Freeze:

```text
ToolDescriptor
ToolRequest
ToolResult
ToolError
ToolEvent
CapabilityDescriptor
ExecutionManifest
```

### Policy intersection

Effective permission is the intersection of:

```text
role
mission policy
task scope
workspace
tool capability
sandbox profile
risk class
human approvals
```

No extension can add rights beyond that intersection.

### Filesystem

Implement:

```text
canonical path resolution
symlink resolution
workspace boundaries
encoding/BOM/newline preservation
binary/large file handling
generated/vendor classification
```

### Search

Wrap ripgrep behind structured exact-search API.

### Shell/process

Implement:

```text
argv-first execution
cwd
sanitized env
timeout
streaming stdout/stderr
process registry
process-tree cancellation
background process
readiness
port ownership
```

### Test/build runner

Detect repository-native commands.

Normalize:

```text
PASS
FAIL
NO_TESTS
SKIPPED
UNAVAILABLE
INDETERMINATE
```

### Package operations

Detect package manager and classify install/update operations separately from ordinary build/test.

Review scripts/postinstall risk.

### Secret Broker skeleton

Resolve secret references only for approved process execution and redact logs/events.

### Sandbox profiles

Implement meaningful V1 profiles appropriate to macOS capabilities without claiming perfect containment.

## Security Fixtures

```text
../ traversal
symlink -> ~/.ssh
nested shell writing outside workspace
malicious package postinstall
secret echoed to stdout
hung child process
child process survives parent
git clean/reset destructive attempt
```

## Exit Evidence

Deterministic harness can inspect, search, edit a fixture, run commands/tests, capture structured/raw evidence, cancel a process tree and deny workspace escape.

---

# HC-P06 — PHASE 6 CONTRACT: BASIC WORKER AGENT LOOP

## Purpose

Build one genuinely useful coding Worker before scaling orchestration.

## Hard Prerequisites

```text
P4 usable inference
P5 Tool Broker
P3 durable task/session identity
```

## Engineering Slices

### Session vs task

`AgentSession` is ephemeral.

`Task` and `TaskAttempt` are durable.

Model transcript never becomes task truth.

### Worker loop

Implement:

```text
context request
→ model turn
→ structured tool calls
→ Tool Broker
→ results/evidence refs
→ next turn
```

### Response handling

Normalize:

```text
tool call
plain response
structured completion request
malformed response
premature stop
refusal
```

No dependence on hidden chain-of-thought.

### Initial editing

Use one robust validated method until P13.

### Completion request

Worker emits evidence-backed request; Kernel may reject.

### Checkpoint triggers

After meaningful changes or before risky model/session replacement, persist enough task/worktree/test state for another Worker.

### Anti-loop

Track:

```text
repeated command/error
same edit/revert
no meaningful diff
no new evidence
```

### Attempt accounting

Store:

```text
model/provider
context manifest
tool calls
tokens
duration
result
failure classification
```

## Benchmark Matrix

At least:

```text
single-file bug
3-file bug
test repair
small refactor
config/build failure
```

across more than one model class where available.

## Exit Evidence

Worker autonomously explores, edits, tests and requests completion on a real fixture without the user typing `continue`.

---

# HC-P07 — PHASE 7 CONTRACT: GIT, WORKTREES & CHECKPOINTING

## Purpose

Make implementation isolated, restartable and auditable.

## Hard Prerequisites

```text
P5 Git/process tools
P6 Worker
06_GIT_WORKTREES implementation packet
```

## Engineering Slices

### Worktree identity

Persist:

```text
worktree_id
mission/task
path
branch
base commit
owner Worker/lease epoch
status
```

### Dirty base handling

Before creating a task worktree, determine whether user base contains uncommitted changes.

Never hide or destroy them.

### Fencing

A Worker that loses the current lease/fencing epoch must not commit/write as active owner.

### Checkpoints

Checkpoint includes:

```text
commit/diff identity
task attempt
test state
context ref
blocker
```

### Replacement

New Worker receives same durable task/worktree lineage.

### Integration

Use a mission integration branch or equivalent isolated integration view.

Conflict becomes structured repair input.

### Cleanup/reconciliation

After crash or cancel:

```text
find orphan worktrees
find orphan branches
find active processes using worktree
preserve evidence
clean only when safe
```

### External drive

Test worktree behavior when project/reference location is on external volume and temporarily disconnects.

## Failure Fixtures

```text
two independent Workers
intentional same-file conflict
manual user edit
Worker killed after checkpoint
zombie Worker attempts commit
worktree directory deleted
crash during checkpoint
```

## Exit Evidence

Two Workers can safely create isolated work, one can be replaced, integration succeeds for independent work, and conflict is detected without blanket ours/theirs.

---

# HC-P08 — PHASE 8 CONTRACT: CODE INTELLIGENCE FOUNDATION

## Purpose

Move foundational repository facts out of model-led exploration.

## Hard Prerequisites

```text
P5 filesystem/search
P7 worktree identity
04_CODE_INTELLIGENCE packet
```

## Engineering Slices

### Repository identity/inventory

Track canonical root, Git identity and file metadata.

### Ignore engine

Merge:

```text
.gitignore
.git/info/exclude
.agentcodeignore
known generated/vendor rules
user override
```

### Content identity

Use hashes/content-addressed cache to avoid full rebuild.

### Parse manager

Tree-sitter adapters normalize:

```text
language
symbol
range
signature
parent
imports/exports
parse errors
```

### Repo map

AgentCode-owned compact map uses deterministic relevance.

### Structural search

Expose ast-grep where available through typed adapter.

### Atomic generations

Index readers see complete generation N or N+1, never half-written state.

### Watchers

Filesystem events are hints; consistency scans repair missed events.

### Readiness

Expose:

```text
BASE_READY
STRUCTURAL_READY
DEGRADED
REBUILDING
```

### Worktree overlays

Unchanged base facts are shared; modified files receive isolated overlay facts.

## Benchmarks

```text
1k files
10k files
monorepo
reopen unchanged
single-file change
3-file change
```

Measure bootstrap/reopen/update/RAM/disk.

## Exit Evidence

AgentCode answers file/symbol/import/change questions deterministically without broad LLM reading.

---

# HC-P09 — PHASE 9 CONTRACT: SEMANTIC INTELLIGENCE & REPOSITORY GRAPH

## Purpose

Add project-aware semantic relationships while preserving structural fallbacks.

## Hard Prerequisites

```text
P8 structural intelligence
LSP process policy from P5
```

## Engineering Slices

### LSP lifecycle

Implement:

```text
server discovery
allowed install policy
spawn
workspace roots
initialize
health
restart
shutdown
```

### Initial language depth

Prioritize TS/JS, Python, Rust and Go because they cover major AgentCode fixture families.

### Capability normalization

Not every LSP supports:

```text
definition
references
rename
call hierarchy
diagnostics
```

Expose per-server capability state.

### Unified graph

Merge evidence from:

```text
Tree-sitter
LSP
SCIP optional
Git
tests
API/schema/config detectors
```

Edges always carry provenance/confidence/freshness/repository view.

### Package/build graph

Detect monorepo/workspace boundaries.

### API/database/config

Add high-value route/schema/migration/ORM/config edges.

### Test relationships

Infer from imports/references/naming/runtime/coverage where available.

### Impact analysis

Given changed symbols/files, produce conservative affected set with confidence.

### SCIP/Zoekt

Remain benchmark-gated optional accelerators, not mandatory for every repository.

## Failure Behavior

LSP crash/broken project must degrade to P8 capabilities.

No semantic edge may be treated as certain without provenance.

## Exit Evidence

Representative full-stack flows can be traced and impact analysis selects relevant dependent files/tests.

---

# HC-P10 — PHASE 10 CONTRACT: PERSISTENT MEMORY & KNOWLEDGE FRESHNESS

## Purpose

Make cross-session project understanding useful without turning summaries into stale truth.

## Hard Prerequisites

```text
P8/P9 evidence sources
P3 durable state
```

## Engineering Slices

### Authoritative vs derived

Kernel mission state and Git/repository evidence remain authoritative.

Knowledge facts are structured derived/project state.

### KnowledgeFact

Record:

```text
statement/type
scope
source
evidence refs
confidence
observed commit/view
freshness dependencies
last validation
```

### Conflict

Contradictory facts coexist as conflict until fresh evidence resolves them.

### Decision records

User/approved architecture/product decisions are durable and versioned.

### Failed approaches

Store important failed approach summaries with evidence so replacement Workers do not blindly repeat them.

### Generated state files

Generate:

```text
CONTEXT.md
MISSION.md
REQUIREMENTS.md
DECISIONS.md
TEST_STATE.md
SECURITY_STATE.md
DESIGN_STATE.md
```

from structured truth where applicable.

These files are handoff/readability views, not canonical databases.

### History/retention

Archive meaningful context snapshots; garbage-collect obsolete derived caches without deleting authoritative evidence.

## Tests

```text
fact stale after relevant file change
unrelated file change does not stale fact
conflicting fact
deleted symbol invalidates fact
new model resumes from state
stale CONTEXT.md cannot override source
```

## Exit Evidence

A different model/session can understand current project/task state without transcript replay, and stale knowledge is visibly invalidated.

---

# HC-P11 — PHASE 11 CONTRACT: CONTEXT ENGINE & TOKEN EFFICIENCY

## Purpose

Convert repository intelligence + memory + mission/task state into minimal sufficient role-specific model input.

## Hard Prerequisites

```text
P8/P9 intelligence
P10 knowledge
P4 model capabilities
P5 raw tool evidence
```

## Engineering Slices

### ContextPack

Include typed sections and a manifest.

### Profiles

```text
TINY
NORMAL
DEEP
AUDIT
EXTREME
```

are budget/selection strategies, not exact token numbers.

### Role-specific packs

Planner, Worker, Researcher and Verifier receive different information.

Verifier pack avoids excessive Worker narrative.

### Relevance

Combine deterministic signals:

```text
direct target
requirements
symbol graph
package
tests
runtime error
Git diff
recent evidence
user references
failed approach
```

### Hard inclusions / hard exclusions

Explicit requirement, direct compile error and project rules may bypass ranking.

Secrets and provider-incompatible sensitive content are excluded/redacted.

### Stable prefix/cache

Order stable project content for provider prompt-cache reuse.

### Raw/tool budget

Large logs remain raw locally with bounded compressed model view.

### RTK-style benchmark

Measure actual task success in addition to compression.

### Immutable cache

Cache source fragments/context sections by content hash/commit/symbol fingerprint.

## Benchmarks

Compare targeted vs broad context on:

```text
cross-module bug
API migration
architecture explanation
test selection
verification
```

Metrics:

```text
verified success
tokens
retries
latency
raw refetch
```

## Exit Evidence

Token/context reduction does not reduce verified task success on the benchmark corpus.

---

# HC-P12 — PHASE 12 CONTRACT: FULL AUTONOMY KERNEL & MULTI-AGENT RUNTIME

## Purpose

Turn isolated Worker capability into a durable mission system that can continue through model, Worker and provider failure.

## Hard Prerequisites

```text
P3 durable core
P4 inference
P6 Worker
P7 worktrees
P11 context
P8/P9 impact intelligence
```

## Engineering Slices

### Goal Interpretation / Mission Contract

Create versioned requirement set from immutable goal.

### Planner contract

Planner produces structured task proposals, not mission truth.

### DAG

Validate IDs/dependencies/cycles/supersession.

### Scheduler

READY-set algorithm considers:

```text
dependencies
priority
critical path
fairness/starvation
resource availability
conflict risk
provider availability
```

Use deterministic tie-breaking where practical.

### Worker Registry / Attempts

Track Worker, session, task attempt and current lease separately.

### Leases / Fencing

Every assignment has lease epoch; expired/zombie Worker cannot mutate current task authority.

### Heartbeats

Runtime-generated liveness must not rely only on model text.

### Progress watermarks

Meaningful progress is:

```text
new accepted diff
new evidence
test improvement
requirement advancement
new resolved blocker
```

Repeated activity alone is insufficient.

### Recovery Engine

Canonical sequence:

```text
DETECT
→ QUIESCE
→ SNAPSHOT REALITY
→ CLASSIFY
→ RECONCILE
→ SELECT RECOVERY
→ APPLY
→ VALIDATE
```

### Retry escalation

Every retry must change a meaningful variable.

### Replanning

Preserve old task/plan lineage and completed valid evidence.

### Resource locks / concurrency

Adaptive concurrency replaces hard-coded Worker count.

### Durable communication

Mailboxes/blackboard carry typed facts/requests, not free-form hidden coordination only.

### Human escalation

Deduplicated durable request with reason/options.

### Pause/resume/cancel

Safe checkpoint + reconcile, not blind process kill.

### Budget

Mission/task budget is enforced but cannot weaken correctness gates.

## Failure Fixtures

```text
provider loss
Worker death
Planner death
lease expiry
zombie Worker
context exhaustion
two conflicting tasks
resource pressure
replan after new requirement
duplicate scheduler tick
daemon restart
human pause during edit
```

## Exit Evidence

Long mission survives at least provider failover, Worker replacement and replan with no routine babysitting and with coherent durable state.

---

# HC-P13 — PHASE 13 CONTRACT: ADVANCED EDIT ENGINE

## Purpose

Make edits reliable enough that the model can express intent without being trusted to manipulate files unsafely.

## Hard Prerequisites

```text
P5 tool/filesystem
P7 Git/worktrees
P8/P9 symbol intelligence
P12 task/lease authority
```

## Engineering Slices

### Edit strategies

```text
search/replace
unified diff
whole-file replacement
structured symbol edit
AST transform
LSP operation
```

are adapters producing one ChangeSet model.

### Preconditions

Require expected:

```text
content hash
base revision
symbol fingerprint where applicable
```

### Transaction journal

ChangeSet lifecycle:

```text
PREPARED
APPLYING
APPLIED
VALIDATING
ACCEPTED
ROLLING_BACK
ROLLED_BACK
CONFLICT
UNKNOWN_EFFECT
```

### Metadata preservation

Preserve:

```text
encoding
BOM
newline
executable bit
permissions
```

where appropriate.

### Scope guards

Generated/vendor/lockfile/binary/secret-bearing files receive explicit policy.

### Validation ladder

```text
parse
format
lint
typecheck/compile
targeted tests
```

based on project and change type.

### Crash recovery

On daemon restart, journal determines whether to finish, repair or roll back.

### Concurrent human edit

Never overwrite silently. Convert to conflict/rebase/repair.

### Diff quality

Detect accidental broad formatting/noise.

## Benchmarks

By model/task type measure:

```text
first apply
syntax failure
retry count
unintended diff
Verifier rejection
```

## Exit Evidence

Multi-file edit survives crash, concurrent user edit and rollback without corrupting unrelated work.

---

# HC-P14 — PHASE 14 CONTRACT: VERIFICATION & EVIDENCE ENGINE

## Purpose

Make “done” an evidence state instead of a conversational state.

## Hard Prerequisites

```text
P12 requirements/tasks
P13 reliable changes
P8/P9 impact analysis
P5 test/build execution
P7 commit/worktree identity
```

## Engineering Slices

### VerificationProfile

Derive required V0–V7 layers from:

```text
requirement type
task risk
project capability
security/design applicability
```

Worker cannot weaken profile.

### Evidence record

Every proof ties to:

```text
repo view/commit
environment fingerprint
tool/model version
command/request
raw artifact
normalized result
requirement refs
freshness dependencies
```

### Baseline vs regression

Capture pre-existing failures so new work is not blamed for unrelated old defects, while preventing new regressions from hiding in noise.

### Test registry / selection manifest

Persist selected tests, why they were selected, what was excluded and fallback broad suite.

### Flaky policy

Bounded reruns only.

No rerun-until-green.

### Mechanical gates

Normalize unavailable/no-tests/skipped/partial states.

### Independent Verifier

Read-only by default, evidence-first, anti-anchored context.

### False-done

Challenge:

```text
missing wiring
mock-only behavior
unreachable code
stale evidence
skipped test
uncommitted/unintegrated change
```

### Test tampering

Detect weakened assertions, new skip, fixture manipulation, snapshot rewrite and mocking changes.

### Integration verification

After merge, invalidate/re-run affected proof.

### Final Audit

Mission-level audit compares final integrated state with immutable goal + accepted contract amendments.

Kernel alone transitions mission COMPLETE.

## Acceptance Fixtures

Expand `FIX-VERIFY-FALSE-DONE` with:

```text
unwired route
weak test
stale pass
mock implementation
deleted assertion
pre-existing failing test
flaky test
integration regression
```

## Exit Evidence

A persuasive Worker and even a superficially agreeable Verifier cannot bypass deterministic completion.

---

# HC-P15 — PHASE 15 CONTRACT: BROWSER RUNTIME & VISUAL VERIFICATION

## Purpose

Prove user-visible/runtime behavior in a real browser while keeping browser execution deterministic and isolated.

## Hard Prerequisites

```text
P5 process/port manager
P14 evidence
09_BROWSER_QA packet
```

## Engineering Slices

### Playwright core

Detect/install approved browser runtime and expose stable AgentCode actions.

### Session/profile registry

Track isolated contexts by task/mission.

Do not reuse user's personal browser profile.

### Dev-server readiness

A server is ready only when:

```text
process alive
+
HTTP readiness
+
target route loadable
```

### Evidence

Capture:

```text
DOM assertions
accessibility tree
console
page errors
network errors
screenshots
traces
```

tied to commit/environment.

### Sensitive browser state

Cookies/storage/screenshots receive sensitivity classification and retention policy.

### Crash recovery

Browser crash restarts session and invalidates stale browser evidence; mission state survives.

### Agentic exploration

Browser Use/Harness-style reasoning remains optional for unknown navigation. Deterministic Playwright remains core.

## Resource Rules

Default one active interactive preview/browser session per relevant task unless Resource Governor permits more.

## Exit Evidence

Fixture flow is executed, asserted and evidenced in browser; deliberate browser crash recovers.

---

# HC-P16 — PHASE 16 CONTRACT: SKILLS, HOOKS & MCP

## Purpose

Add extensibility without creating alternate policy or execution authorities.

## Hard Prerequisites

```text
P5 Tool Broker
P11 context
P12 Kernel events
08_SKILLS_HOOKS_MCP packet
```

## Engineering Slices

### Skill manifest

Track:

```text
id/version/source
scope
trust
triggers
required capabilities
context cost
executable assets
```

### Progressive loading

Model receives skill summary first, full instructions only when selected.

### Project instructions

Normalize trusted instruction files and nested scope with provenance.

### Hook engine

Typed registration:

```text
event
priority/order
timeout
idempotency
failure policy
recursion guard
```

### MCP lifecycle

Track:

```text
server identity
transport
capabilities
trust
health
version
```

MCP metadata/output is untrusted model input.

### Policy

Every extension action still goes through Tool Broker.

Core file/edit/Git/test remains native.

## Security Fixtures

```text
skill requests secret exfiltration
hook recursively triggers itself
MCP description says ignore policy
MCP tool attempts external write
server disappears mid-task
```

## Exit Evidence

A skill, hook and MCP server can be added/removed without changing Kernel code or bypassing policy.

---

# HC-P17 — PHASE 17 CONTRACT: BASELINE SECURITY PLATFORM

## Release Scope

`REQUIRED_V1`.

This is not optional simply because advanced red-team tooling is later.

## Purpose

Create a normalized security reasoning and remediation workflow over repository evidence.

## Hard Prerequisites

```text
P14 evidence/findings
P16 extension/tool adapter patterns
P8/P9 attack-surface intelligence
11_APPSEC packet
```

## Engineering Slices

### Security policy

Versioned project policy defines:

```text
blocking severities
scan applicability
active-test permission
suppression/risk rules
retention
```

### Threat Model / Attack Surface

Build evidence-backed structured records.

### Finding store

Separate scanner-specific `FindingInstance` from normalized root `SecurityFinding`.

### Core baseline adapters

Realistic baseline:

```text
Gitleaks
Semgrep CE
OSV-Scanner and/or Trivy
Checkov and/or Trivy IaC when applicable
```

### Health honesty

Distinguish:

```text
scanner ran with zero findings
```

from:

```text
scanner missing/failed/stale ruleset.
```

### Triage / false-positive reduction

Evaluate reachability, project context and duplicate root causes.

### Remediation / regression

Confirmed finding produces Kernel repair task and later regression evidence.

### Reporting

Generate Markdown + JSON + SARIF where appropriate.

### AgentCode self-security seed

Run baseline on AgentCode itself.

## Exit Evidence

Seeded vulnerable fixture produces normalized candidates, confirmed/dismissed outcomes, repair task, rescan/regression and redacted report.

---

# HC-P18 — PHASE 18 CONTRACT: ADVANCED APPSEC, CLOUD & RED-TEAM

## Release Scope

Mixed:

```text
Web DAST / cloud posture / authorized validation
→ REQUIRED_IF_APPLICABLE

deep Red-Team Lab breadth
→ OPTIONAL_V1
```

## Hard Prerequisites

```text
P17 finding/security policy
P15 browser
P5 scope/sandbox/process
12_CLOUD_SECURITY packet
```

## Engineering Slices

### Authorization record

Before active testing:

```text
target
environment
allowed domains/IPs/accounts
credentials
rate/concurrency
forbidden actions
expiry
cleanup
```

### Web DAST

ZAP/Nuclei adapters enforce scope before invocation.

Pin engine and rules/template provenance.

### Business-logic hypotheses

Models may propose abuse hypotheses; proof uses synthetic users/data where possible.

### Attack Path Graph

Combine entry point, identity, permission, finding, resource, control and impact with evidence.

### Cloud posture

Prowler primary broad adapter where applicable; complementary scanners only for additional signal.

Read-only is default.

### Active validation

Minimum proof only.

Mandatory stop on:

```text
proof achieved
scope boundary
unexpected production data
destructive side-effect risk
rate/error threshold
authorization expiry
```

### Lab lifecycle

Provision/snapshot/seed/run/cleanup/verify.

Pacu/Stratus/CloudGoat remain lab/advanced tools, not normal production defaults.

## Exit Evidence

Authorized disposable fixture demonstrates one multi-step path and cleanup. Production-style policy rejects destructive proof.

---

# HC-P19 — PHASE 19 CONTRACT: AI SECURITY

## Release Scope

Baseline checks are `REQUIRED_IF_APPLICABLE` to AI-enabled repositories. Advanced external orchestration is optional V1.

## Hard Prerequisites

```text
P17 security model
P16 MCP/tool trust
P18 validation framework where active testing is used
13_AI_SECURITY packet
```

## Engineering Slices

### AI surface detection

Identify:

```text
model gateways
user prompts
indirect content
RAG
memory
MCP
tools
external actions
secret sources
output sinks
```

### Test taxonomy

```text
direct injection
indirect injection
prompt/system leakage
secret leakage
RAG poisoning
tool abuse
MCP abuse
cross-agent manipulation
excessive agency
```

### Safe fixtures

Use synthetic secrets and dummy destructive tools.

### External harnesses

Promptfoo/Garak may be wrapped where useful.

PyRIT remains advanced optional depth.

### AgentCode regression

Permanent self-tests for:

```text
malicious README/comment
scanner-output injection
MCP description injection
skill injection
tool argument abuse
```

## Exit Evidence

AI fixture detects at least one direct/indirect trust failure, prevents unsafe tool action and produces remediation/regression evidence.

---

# HC-P20 — PHASE 20 CONTRACT: DISCUSS MODE

## Release Scope

`REQUIRED_V1`.

## Purpose

Expose shared Code Intelligence/Context as a high-quality engineering conversation without creating a second runtime.

## Hard Prerequisites

```text
P9 repository semantics
P10 knowledge
P11 context
P4 routing
P12 project/decision state
```

## Engineering Slices

### DiscussSession

Persist session/message/context-manifest references separately from mission state.

### Read-only profile

Default tools:

```text
read
search
code intelligence
non-mutating diagnostics
research where allowed
```

No write tools.

### Context profiles

Use lighter question-oriented retrieval than Goal mission.

### Local fallback

Small local model may answer basic explanations/summaries during cloud outage with degraded-quality indication.

### Decision candidates

Extract structured candidate; user acceptance creates durable decision record.

### Plan promotion

Turn discussion into:

```text
requirements
constraints
open questions
accepted decisions
```

then standard Kernel mission.

No transcript replay dependency.

## Exit Evidence

A real repository architecture discussion becomes a mission without copying context manually and without any silent source edit during Discuss.

---

# HC-P21 — PHASE 21 CONTRACT: DESIGN STUDIO

## Release Scope

Core workflow is `REQUIRED_V1`.

Universal direct DOM-to-source manipulation is `OPTIONAL_V1`.

## Hard Prerequisites

```text
P13 editing
P14 verification
P15 browser
P11 context
P16 skills
10_DESIGN_STUDIO packet
```

## Engineering Slices

### Existing product analysis

Detect:

```text
framework
routes
layouts
styles
tokens
component libraries
fonts/assets
navigation
baseline screens
```

### DesignBrief / DesignGrammar

Persist structured product-specific design intent and provenance.

### Constraints

Support hard user constraints:

```text
preserve navigation
do not touch checkout
keep typography
```

### Mission types

```text
DESIGN_ONLY
DESIGN_AND_IMPLEMENT
VISUAL_REPAIR
DESIGN_SYSTEM_EVOLUTION
```

### Preview

Use isolated dev server + browser profile.

### Iteration

Record meaningful:

```text
change set
screenshots
browser checks
visual findings
functional/accessibility results
```

### Visual critique

Structured findings, not “looks good.”

### Anti-slop

Combine deterministic candidate heuristics and design-brief-aware visual reasoning.

### Responsive/accessibility

Use viewport matrix, keyboard/focus/semantics/contrast and interaction-state evidence.

### Functional preservation

Run pre-identified product workflows after redesign.

### Performance

Flag unjustified bundle/dependency/asset cost.

### Screenshot privacy

Redact/classify before remote visual routing.

### Direct visual selection

Prototype only where source mapping confidence is real. Low confidence must be shown honestly.

## Exit Evidence

A real fixture/app goes through at least two meaningful autonomous design iterations, passes visual/responsive/accessibility/functional criteria, and preserves explicit user constraints.

---

# HC-P22 — PHASE 22 CONTRACT: MINIMAL DESKTOP PRODUCT EXPERIENCE

## Release Scope

`REQUIRED_V1`.

## Purpose

Expose runtime truth through a calm product shell.

A thin developer mission client may exist much earlier for P3/P6 testing. Phase 22 owns the **polished user-facing product**, not the first debug UI.

## Hard Prerequisites

```text
P3 daemon protocol
P12 mission model
P14 verification
P20 Discuss
P21 Design core
P17 Security baseline
```

## Engineering Slices

### Tauri shell / routing

Primary areas:

```text
Projects
Goal
Discuss
Design
Security
Mission
Settings
Diagnostics
```

Mission subviews:

```text
Summary
Changes
Activity
Verification
Security
Details
```

### Snapshot-then-stream

Renderer reconnects to authoritative daemon state.

### Truthful progress

Do not invent percentages before plan/requirement state is stable.

### Activity compression

Group raw events into meaningful engineering actions while preserving errors/failures.

### Changes

Virtualized grouped diff, task/change-set provenance and verification state.

### Needs You

Deduplicated blocker with explanation/options/recommendation.

### Controls

Pause/resume/cancel must wait for Kernel confirmation.

### Notifications

Only completion, genuine human action and meaningful blocker/failure.

Privacy-safe lock-screen text.

### Settings

Provider/budget/autonomy/appearance/privacy/developer settings with secrets outside renderer config.

### Accessibility

Keyboard core flows, focus restoration, semantic status, reduced motion, contrast.

### Performance

Virtualize long histories/diffs and bound terminal buffers.

## Exit Evidence

New user can open project, run Goal mission, close/reopen UI, inspect Changes/Activity, answer Needs You, use Discuss/Design/Security and receive truthful completion notification without understanding internal architecture.

---

# HC-P23 — PHASE 23 CONTRACT: RESOURCE, TOKEN & COST OPTIMIZATION

## Purpose

Make the proven architecture practical on an 8 GB Mac and under limited/free inference.

Optimization may tune thresholds and strategies but may not weaken correctness/safety gates.

## Hard Prerequisites

Functional versions of all resource-heavy subsystems.

## Engineering Slices

### Resource telemetry

Measure per component:

```text
RSS
CPU
disk
process count
browser
LSP
local model
indexes
Worker count
```

### Admission

Resource Governor decides whether new Worker/browser/scanner/local model can start.

### Adaptive Worker concurrency

No fixed “2 Workers” rule.

### Local model lifecycle

Lazy load / unload / OOM recovery.

### LSP lifecycle

Close idle/restart unhealthy.

### Index policy

Benchmark SCIP/Zoekt activation.

### Context tuning

Tune relevance/context profile budgets based on verified outcomes.

### Compression

Benchmark raw vs RTK-style view.

### Routing history

Use empirical outcomes but keep deterministic hard filters.

### Spend

Enforce configured paid reserve and denial-of-wallet protections.

### Desktop

Measure renderer/daemon memory and responsiveness under active mission.

## Exit Evidence

Representative workload on real 8 GB Mac avoids severe sustained memory pressure and maintains verified task success while reducing wasted tokens/cost.

---

# HC-P24 — PHASE 24 CONTRACT: CHAOS ENGINEERING & RECOVERY VALIDATION

## Purpose

Deliberately attack every critical durability promise.

## Hard Prerequisites

All core V1 recovery paths implemented.

## Fault Families

### Provider/model

```text
429
timeout
DNS/network drop
stream interruption
model removed
malformed tool call
context exhaustion
premature stop
```

### Worker/scheduler

```text
Worker kill
lease expiry
zombie Worker
Planner kill
duplicate scheduler tick
duplicate IPC command
```

### Process/tool

```text
browser crash
LSP crash
scanner crash
hung command
child process leak
dev-server death
```

### Editing/Git

```text
crash after file N of ChangeSet
manual user edit
worktree missing
checkpoint interruption
disk full
```

### Daemon/database

```text
daemon kill
power-loss simulation
DB busy
transaction interruption
migration failure
corrupt derived cache
event replay gap
```

### Resource

```text
low memory
filesystem full
network restricted
external drive disconnect
```

## Expected Result Categories

Every chaos case defines one of:

```text
RECOVER_AUTOMATICALLY
DEGRADE_AND_CONTINUE
BLOCK_SAFELY
NEEDS_USER
FAIL_SAFE_WITH_DURABLE_STATE
```

“No crash” is not the only acceptable outcome; coherent state is mandatory.

## Repetition

Critical recovery scenarios run repeatedly, not once.

## Exit Evidence

Doc 10 chaos gate family passes with stable outcomes and no data-loss/secrecy/scope regression.

---

# HC-P25 — PHASE 25 CONTRACT: AGENTCODE DOGFOODING

## Purpose

Use normal AgentCode workflows to improve AgentCode itself.

No privileged hidden execution path is allowed.

## Required Mission Set

At minimum:

```text
real bug fix
multi-file feature
refactor
test/coverage repair
dependency update
provider-failure mission
restart-during-mission
Discuss → Mission
Design Studio UI improvement
Security audit + repair
long unattended mission
```

### Security-specific

Include:

```text
prompt injection fixture
malicious MCP
malicious skill
```

### Metrics

Record:

```text
verified completion
human interventions
provider/model switches
Worker replacements
context compactions
repair cycles
Verifier rejections
tokens
paid cost
wall time
```

### Failure Review

Every human intervention asks:

```text
Was this truly unavoidable?
Could durable state/policy/tooling remove it?
```

Every false completion/recovery failure becomes P0/P1 repair before RC.

## Exit Evidence

AgentCode produces useful merged improvements to itself across several mission classes with the same permissions, Tool Broker and verification rules as external repositories.

---

# HC-P26 — PHASE 26 CONTRACT: SECURITY & LICENSING HARDENING

## Purpose

Treat AgentCode as a privileged developer tool before distribution.

## Hard Prerequisites

P25 dogfood evidence and final production dependency set.

## Engineering Slices

### Final threat model

Cover:

```text
source repositories
provider/Git/cloud credentials
Tool Broker
sandbox
MCP
skills/hooks
browser
managed binaries
updates
logs/evidence
```

### Self-red-team

Attack:

```text
workspace escape
prompt injection
MCP/skill privilege abuse
secret leakage
remote Git boundary
malicious package install
scanner-output injection
```

### Privacy/redaction

Verify secrets/private source do not leak into:

```text
model requests
logs
diagnostics
notifications
reports
screenshots
```

outside policy.

### Dependency/license

Finalize:

```text
license matrix
third-party manifest
THIRD_PARTY_NOTICES
asset audit
bundled/managed binary provenance
SBOM
```

No shipped `UNKNOWN`/`INCOMPATIBLE` license.

### Release-security process

Prepare:

```text
SECURITY.md
vulnerability reporting
update/signing prerequisites
artifact retention
data deletion/uninstall expectations
```

## Exit Evidence

No unresolved critical security blocker, no unresolved incompatible/unknown shipped license, and final self-security suite passes.

---

# HC-P27 — PHASE 27 CONTRACT: PACKAGING, UPDATE & RELEASE ENGINEERING

## Purpose

Turn the development workspace into a reproducible macOS Apple Silicon desktop product.

## Release Scope

macOS ARM64 is V1.

Windows remains later unless scope is explicitly amended.

## Engineering Slices

### Bundle

Package:

```text
desktop
daemon
migrations
required native libraries
approved bundled tools
```

Optional managed tools may install on first use.

### Daemon lifecycle

Define:

```text
start
background continuation
upgrade coordination
safe quit
crash restart
version compatibility
```

### Data directories

Separate:

```text
config
DB
indexes
cache
logs
managed binaries
browser profiles
artifacts
```

### First run

Minimal setup:

```text
open project
verify Git
detect model route
optional provider configuration
detect local model/browser
```

Do not force optional scanners.

### Updates

If V1 includes update mechanism:

```text
signed/provenanced artifact
version check
migration plan
rollback
daemon/desktop compatibility
```

Otherwise document manual update path honestly.

### Signing/notarization

Required for public macOS release when release credentials/process are available and distribution channel demands it.

Do not pretend signing has occurred in developer builds.

### Diagnostics/uninstall

Diagnostics redacts secrets.

Uninstall behavior distinguishes app removal from optional user data removal.

## Packaging Tests

```text
clean Mac install
spaces in paths
external SSD repo
offline startup
upgrade from previous dev build
migration failure
missing optional tool
daemon/UI version mismatch
uninstall
```

## Exit Evidence

A clean representative Mac can install/configure/run AgentCode without the donor reference library or developer checkout.

---

# HC-P28 — PHASE 28 CONTRACT: V1 RELEASE CANDIDATE

## Purpose

Freeze product scope and evaluate the packaged product, not the development environment.

## Change Policy

Allowed:

```text
bug/security/reliability/performance/documentation/UX correctness
```

Requires exceptional approval:

```text
new framework
new mode
new DB authority
new major dependency
scope expansion
```

## RC Blocker Classes

### RC0 — release impossible

```text
data loss
workspace escape
secret leak
false completion
daemon state corruption
critical license blocker
critical security issue
```

### RC1 — required V1 capability broken

```text
routine provider failover fails
Goal mission unreliable
Discuss unavailable
Design core cannot pass its quality gate
Security baseline broken
browser verification broken
background mission stops on UI close
```

### RC2 — quality/performance blocker

```text
8 GB target regularly unusable
packaged update/migration unsafe
major accessibility blocker in primary flow
```

Optional V1 capability defects do not block if capability is removed/disabled/documented and the PRD scope allows that.

## RC Matrix

Run packaged builds across:

```text
fixture languages
mission types
provider profiles
offline/degraded mode
8 GB hardware
browser
Security baseline
Design
Discuss
recovery
```

Cloud lab/advanced red-team tests are required at RC only if those optional/conditional claims are shipped and applicable.

## Evidence

RC produces a frozen acceptance manifest tied to exact release candidate commit/artifact hashes.

---

# HC-P29 — PHASE 29 CONTRACT: V1 RELEASE

## Purpose

Publish only the artifact that passed the global release gate.

## Preconditions

```text
P28 RC accepted
Doc 10 global release gate PASS
no unresolved RC0/RC1
required docs/notices/SBOM
artifact provenance
known limitations approved
```

## Release Procedure

Record:

```text
version
Git tag/commit
desktop artifact hashes
daemon version
database schema version
bundled/managed tool manifest
release notes
known limitations
optional capability list
third-party notices
security policy/contact
```

### Claim honesty

Do not imply:

```text
Windows support
unlimited free inference
universal source mapping
deep cloud red-team breadth
advanced PyRIT automation
```

unless actually shipped and verified.

### Rollback/incident plan

Before publication define:

```text
how to withdraw bad artifact
how to disable compromised managed tool/update
how users recover DB/missions
how release issue is communicated
```

## Final Exit

V1 exists only when the released packaged artifact—not merely repository code—satisfies the required Doc 08 product scope through Doc 10 evidence.

---

# END OF HARDENED PHASE CONTRACTS

# PHASE 0 — PROJECT GOVERNANCE & ARCHITECTURE FREEZE

# 9. Objective

Phase 0 establishes the rules under which the rest of AgentCode will be built.

No significant subsystem should begin while:

```text
core architecture is ambiguous

dependency ownership is unclear

licensing procedure does not exist

there is no ADR system

there is no acceptance-gate system
```

The purpose is not bureaucracy.

The purpose is to stop autonomous implementation agents from independently making incompatible architectural decisions.

---

# 10. Phase 0 Inputs

Required:

```text
Docs 01–09

reference repository library

project directory

initial architectural decisions
```

Docs 10 and 11 may still be under preparation during this phase, but must exist before large implementation begins.

---

# 11. Phase 0.1 — Core Document Registry

Create a machine-readable or strongly structured registry containing:

```text
document_id

filename

title

version

status

last_updated

depends_on

supersedes
```

Example:

```text
DOC-03
Autonomy Kernel & Agent Runtime
V1
LOCKED
```

Implementation agents should always be able to determine the current authoritative documents.

---

# 12. Phase 0.2 — ADR System

Create:

```text
docs/adr/
```

with a consistent template.

An ADR must include:

```text
ID

Title

Date

Status

Context

Decision

Alternatives Considered

Why Rejected

Consequences

Migration Impact

Affected Docs
```

Initial ADR candidates:

```text
ADR-001 Core daemon implementation language

ADR-002 SQLite database ownership

ADR-003 UI ↔ daemon communication mechanism

ADR-004 OmniRoute fork strategy

ADR-005 worktree storage location

ADR-006 AgentCode project-state storage

ADR-007 LSP process management

ADR-008 desktop runtime technology

ADR-009 external binary management

ADR-010 security tool deployment

ADR-011 logging format

ADR-012 local IPC authentication/security
```

---

# 13. Phase 0.3 — Development Governance Rules

Lock rules such as:

```text
No new core framework without ADR.

No direct OSS source copying before license review.

No second mission-state authority.

No second repository-knowledge database.

No second security finding database.

No broad provider-specific assumptions in Kernel.

No phase-complete claims without evidence.

No silent architecture deviation.

No secrets committed to repository.

No generated credentials inside fixtures.
```

---

# 14. Phase 0.4 — Branch and Integration Policy

Recommended development model:

```text
main
  stable integrated development

dev
  optional broader integration stream

feature/<subsystem>

fix/<issue>

agentcode/mission-*/...
```

The exact branch layout may change via ADR.

The important requirement is:

```text
implementation experiments
must not repeatedly corrupt the primary development branch.
```

---

# 15. Phase 0.5 — License Review Process

Before any direct source reuse, record:

```text
upstream repo

commit

license

file-level exceptions

asset license

intended reuse type

distribution implications
```

Possible outcomes:

```text
PASS_DIRECT_REUSE

PASS_WRAPPER_ONLY

PASS_CONCEPT_ONLY

REIMPLEMENT

BLOCKED
```

---

# 16. Phase 0.6 — Development Tracking

Every roadmap phase and milestone should have persistent status.

Conceptual fields:

```text
phase_id

milestone_id

status

owner

dependencies

started_at

completed_at

blockers

evidence
```

This can begin as Markdown/YAML/SQLite and later move into AgentCode itself.

---

# 17. Phase 0 Exit Evidence

Before moving forward:

```text
core docs registered

ADR mechanism exists

branch strategy recorded

license procedure recorded

phase tracking exists

architecture ownership rules documented
```

Failure to establish these means implementation agents may begin diverging before meaningful coding even starts.

---

# PHASE 1 — OSS EXTRACTION & EVIDENCE COLLECTION

# 18. Objective

Phase 1 turns the cloned repository collection into usable engineering knowledge.

The current local library is valuable only if AgentCode can answer questions like:

```text
How exactly does Codex enforce command permissions?

Where exactly does Aider build its repository map?

How does Gemini CLI persist or compress context?

How does Letta Code maintain durable memory?

How does OpenHands represent actions and observations?

How does Onlook resolve rendered UI back to source code?

Which security scanners expose reliable JSON/SARIF output?
```

The objective is not to "study everything."

It is to eliminate unnecessary reinvention before implementation.

---

# 19. Phase 1.1 — Repository Catalog

Create:

```text
AGENTCODE_REFERENCE_CATALOG.md
```

and preferably machine-readable equivalent.

For every local repo capture:

```text
name

local path

upstream URL

HEAD SHA

branch

license

category

priority

maintenance status

primary AgentCode relevance

extraction status
```

Do not rely on folder names alone.

---

# 20. Phase 1.2 — Verify Reference Integrity

For every P0/P1 reference:

```text
git status

git rev-parse HEAD

git remote -v
```

must be recorded.

This catches:

```text
incomplete clones

wrong branch

stale fork

unexpected repository

accidental local modifications
```

---

# 21. Phase 1.3 — License Matrix

Create:

```text
OSS_LICENSE_MATRIX.md
```

Particularly inspect:

```text
Dyad / src/pro

CodeQL query repo vs CLI/engine

Trail of Bits skill content

Bolt/WebContainer boundaries

Munder artwork

Universal Ctags

Daytona

archived/source-available projects
```

If uncertain:

```text
do not copy.
```

---

# 22. Phase 1.4 — Extraction Template

Every subsystem investigation uses the same format:

```text
Question being answered

Relevant Doc requirement

Reference repositories

Commit SHAs

Exact source paths

Observed mechanism

Control flow

Important data structures

Failure behavior

Reusable API ideas

Architecture conflicts

TAKE / ADAPT / WRAP / STUDY / REJECT

License status

Prototype recommendation

Final AgentCode recommendation
```

---

# 23. Phase 1.5 — Provider Fabric Extraction

Deep sources:

```text
OmniRoute

OpenCode

Codex

Goose
```

Must determine:

```text
provider registration

model catalog

credential representation

request normalization

streaming abstraction

error abstraction

usage reporting

rate-limit handling

fallback behavior

model capability metadata
```

AgentCode-specific questions:

```text
Can OmniRoute cleanly return top-K?

Where can quota domains be represented?

How should provider health be exposed?

How can local Ollama appear beside remote models?

What must remain in Model Broker rather than OmniRoute?
```

Deliverable:

```text
01_PROVIDER_FABRIC_EXTRACTION.md
```

---

# 24. Phase 1.6 — Agent Runtime Extraction

Deep sources:

```text
Codex

mini-SWE-agent

OpenHands / Software Agent SDK

Gemini CLI
```

Supporting:

```text
LangGraph

Microsoft Agent Framework

Munder Difflin
```

Investigate:

```text
model loop

tool loop

session object

interruptions

resume

checkpoint

worker state

error recovery

event representation

agent termination

subagent spawning
```

Deliverable:

```text
02_AGENT_RUNTIME_EXTRACTION.md
```

---

# 25. Phase 1.7 — Tools & Editing Extraction

Deep:

```text
Codex

Aider

Cline
```

Supporting:

```text
Gemini CLI

OpenHands

OpenCode
```

Trace:

```text
file read

file write

patch formats

search/replace

whole-file changes

shell execution

process handling

approval checks

checkpoint behavior

rollback behavior

diff construction
```

The report must determine which edit forms different reference agents prefer and why.

Deliverable:

```text
03_TOOLS_EDITING_EXTRACTION.md
```

---

# 26. Phase 1.8 — Code Intelligence Extraction

Deep:

```text
Aider

Tree-sitter

OpenCode

SCIP

Zoekt
```

Supporting:

```text
Codex

ast-grep

ctags

Stack Graphs
```

Investigate:

```text
symbol extraction

repo-map ranking

graph construction

LSP lifecycle

semantic search

incremental reindex

large-repository search

cache invalidation
```

Deliverable:

```text
04_CODE_INTELLIGENCE_EXTRACTION.md
```

---

# 27. Phase 1.9 — Context & Memory Extraction

Deep:

```text
Letta Code

Gemini CLI

Aider

RTK
```

Supporting:

```text
Letta

Graphiti

Cline

Caveman
```

Investigate:

```text
memory representation

cross-session state

context compaction

context selection

raw history

summaries

Git-backed memory

tool-output reduction

retrieval
```

Deliverable:

```text
05_CONTEXT_MEMORY_EXTRACTION.md
```

---

# 28. Phase 1.10 — Git & Worktree Extraction

Deep:

```text
Codex

Cline

Aider

Superpowers
```

Supporting:

```text
Munder Difflin
```

Investigate:

```text
worktree creation

branch naming

checkpoint commit

rollback

integration

conflict behavior

cleanup
```

Deliverable:

```text
06_GIT_WORKTREES_EXTRACTION.md
```

---

# 29. Phase 1.11 — Sandbox & Permissions Extraction

Deep:

```text
Codex

OpenHands

SWE-ReX

Gemini CLI
```

Supporting:

```text
OpenCode

Cline
```

Trace:

```text
command interception

filesystem scope

approval model

network policy

sandbox profile

dangerous command handling

process spawning
```

Deliverable:

```text
07_SANDBOX_PERMISSIONS_EXTRACTION.md
```

---

# 30. Phase 1.12 — Skills, Hooks & MCP Extraction

Inspect:

```text
Gemini CLI

Superpowers

Compound Engineering

Letta Skills

Trail of Bits Skills

OpenHands

MCP Servers
```

Determine:

```text
skill format

metadata

progressive loading

scope

trust

script execution

hook lifecycle

MCP capability discovery
```

Deliverable:

```text
08_SKILLS_HOOKS_MCP_EXTRACTION.md
```

---

# 31. Phase 1.13 — Browser Extraction

Deep:

```text
Playwright
```

Supporting:

```text
Browser Use

Browser Harness

Cline
```

Investigate:

```text
browser process management

persistent sessions

screenshots

DOM tree

console

network logs

agentic browser state

recovery
```

Deliverable:

```text
09_BROWSER_QA_EXTRACTION.md
```

---

# 32. Phase 1.14 — Design Studio Extraction

Deep:

```text
Onlook

Dyad

Bolt.diy
```

Trace:

```text
prompt-to-code pipeline

preview lifecycle

source mapping

browser refresh

visual selection

project state

generated-file ownership

provider integration
```

Deliverable:

```text
10_DESIGN_STUDIO_EXTRACTION.md
```

---

# 33. Phase 1.15 — AppSec Extraction

Inspect:

```text
OpenHack

Trail of Bits Skills

Semgrep

CodeQL

Gitleaks

OSV-Scanner

Trivy

Checkov

ZAP

Nuclei

Scorecard
```

Deliverable:

```text
11_APPSEC_EXTRACTION.md
```

Must define:

```text
scanner invocation

structured outputs

installation models

runtime cost

licensing

normalization strategy
```

---

# 34. Phase 1.16 — Cloud Security Extraction

Inspect:

```text
Prowler

ScoutSuite

CloudSploit

Stratus Red Team

Pacu

CloudGoat
```

Deliverable:

```text
12_CLOUD_SECURITY_EXTRACTION.md
```

---

# 35. Phase 1.17 — AI Security Extraction

Inspect:

```text
Promptfoo

Garak

PyRIT
```

Deliverable:

```text
13_AI_SECURITY_EXTRACTION.md
```

---

# 36. Phase 1.18 — Product UX Extraction

Study interaction mechanics from:

```text
Codex

OpenCode

Cline

Goose

Dyad

Onlook
```

Identify:

```text
mission entry

activity presentation

diff review

permission UX

background execution

model details

preview UX
```

Deliverable:

```text
14_PRODUCT_UX_EXTRACTION.md
```

---

# 37. Phase 1.19 — Extraction Verification

A separate reviewer must verify each foundational extraction report.

Questions:

```text
Did the researcher actually inspect code?

Are exact paths present?

Does observed behavior match source?

Was licensing guessed?

Was a stronger mechanism overlooked?

Does the recommendation violate Docs 01–08?
```

---

# 38. Phase 1 Exit Gate

Phase 1 is not complete until:

```text
reference catalog exists

license matrix exists

all foundational extraction reports exist

major source paths are recorded

core reuse decisions are classified

foundational license blockers are understood

prototype questions are identified
```

Implementation may begin for an individual subsystem once its extraction report is approved; Phase 1 does not have to block unrelated subsystem preparation unnecessarily.

---

# PHASE 2 — REPOSITORY SKELETON & DEVELOPMENT INFRASTRUCTURE

# 39. Objective

Create a source repository capable of supporting years of modular development.

This phase should prevent future architectural pain caused by:

```text
one giant application package

mixed UI/runtime state

unstructured tests

unversioned database schemas

cross-subsystem circular dependencies
```

---

# 40. Phase 2.1 — Select Core Languages and Boundaries

Through ADR, decide the practical language split.

Expected direction:

```text
Desktop:
Tauri + React + TypeScript

Kernel/runtime:
Rust strongly preferred candidate

Provider gateway:
adapt according to OmniRoute implementation ecosystem

Special security adapters:
process-based integration, often Python/Go binaries
```

Do not convert every upstream tool into the Kernel language.

---

# 41. Phase 2.2 — Source Structure

Establish clean boundaries such as:

```text
apps/
    desktop/

core/
    kernel/
    runtime/
    model_broker/
    intelligence/
    context/
    tools/
    edit/
    git/
    verification/
    security/

services/
    provider_gateway/

adapters/
    lsp/
    browser/
    mcp/
    security/

skills/

tests/

fixtures/

benchmarks/

docs/
```

Exact layout depends on language ecosystem.

The invariant is separation of ownership.

---

# 42. Phase 2.3 — Build Tooling

Set up:

```text
dependency locks

formatters

lint

compiler/typecheck

test runners

workspace build

development scripts
```

One command should eventually run the basic project validation suite.

---

# 43. Phase 2.4 — CI Foundation

CI should initially verify:

```text
format

lint

build

unit tests

license checks later
```

CI is not final verification; it is foundational engineering hygiene.

---

# 44. Phase 2.5 — Fixture Repository Suite

Create deliberately small repositories representing:

```text
TypeScript

Next.js full-stack

Python

Rust

Go

polyglot

large-ish monorepo

vulnerable web application

AI agent application
```

Fixtures must be version-controlled and deterministic enough for benchmarks.

---

# 45. Phase 2.6 — Logging

Create structured logging library/interface.

Fields should support:

```text
timestamp

component

severity

mission_id

task_id

worker_id

event

metadata
```

Sensitive fields must be redactable.

---

# 46. Phase 2.7 — Configuration

Define:

```text
application config

developer config

provider config references

feature flags

test config
```

Avoid spreading environment-variable reads across every subsystem.

---

# 47. Phase 2.8 — Schema Migration Infrastructure

Even before real Kernel state exists, prepare versioned SQLite migrations.

Do not create:

```text
if table missing, create random current schema
```

without version migration strategy.

---

# 48. Phase 2 Exit Gate

Evidence:

```text
clean clone builds

tests execute

CI passes

fixture repositories exist

structured logs work

empty daemon process can start

empty desktop shell can start
```

No meaningful agent feature is required yet.

---

# PHASE 3 — BACKGROUND DAEMON & PERSISTENT KERNEL FOUNDATION

# 49. Objective

Build the first capability that truly differentiates AgentCode from chat-style coding wrappers:

> Mission state exists outside the UI and outside the model.

At the end of this phase, even without a sophisticated agent, AgentCode should already demonstrate durable local mission state.

---

# 50. Phase 3.1 — Kernel Database

Implement initial tables for:

```text
repositories

missions

requirements

plans

tasks

dependencies

workers

leases

events

checkpoints

evidence

human_requests

settings
```

All mutations should use explicit transactions.

---

# 51. Phase 3.2 — Repository Registration

The daemon can register:

```text
repo_id

path

Git root

remote

current HEAD
```

No deep indexing yet.

---

# 52. Phase 3.3 — Mission Object

Implement:

```text
mission_id

original_goal

status

repo scope

created_at

updated_at

policy references
```

The original goal must be immutable.

---

# 53. Phase 3.4 — Event Store

Every important lifecycle change emits a typed event.

Implement:

```text
event_id

mission_id

task_id optional

event_type

timestamp

payload

causation_id

correlation_id
```

---

# 54. Phase 3.5 — Daemon Lifecycle

Daemon responsibilities:

```text
singleton ownership

startup

database initialization

migration

IPC server

background scheduler placeholder

safe shutdown

recovery scan
```

---

# 55. Phase 3.6 — IPC

Select and implement a local protocol.

Required V1 operations initially:

```text
health

register project

create mission

get mission

list missions

pause

resume

cancel

subscribe events
```

Later interfaces extend the protocol.

---

# 56. Phase 3.7 — UI Independence

Demonstrate:

```text
start daemon

create mission

close client/UI

mission remains persisted

reopen client

mission visible
```

---

# 57. Phase 3.8 — Restart Reconciliation

Daemon startup scans for:

```text
RUNNING missions

RUNNING tasks

expired leases

unclean shutdown marker
```

At this early phase no real Worker exists, but the reconciliation architecture must be present.

---

# 58. Phase 3.9 — Database Backup/Integrity

Implement minimum:

```text
WAL mode if appropriate

busy timeout/concurrency policy

integrity check

periodic safe backup strategy

migration backup
```

Final corruption recovery becomes deeper in Phase 24.

---

# 59. Phase 3 Exit Gate

Prove:

```text
mission survives UI closure

mission survives daemon restart

original goal remains unchanged

state transitions persist

event history persists

schema migrations work
```

No model needs to be involved.

---

# PHASE 4 — MODEL BROKER & OMNIROUTE PROVIDER FABRIC

# 60. Objective

Make inference a resilient commodity before relying on it for autonomous missions.

AgentCode should never reach later phases while one hardcoded model/API is the only execution route.

---

# 61. Phase 4.1 — Fork OmniRoute

Create the controlled AgentCode fork.

Record:

```text
upstream repository

base SHA

AgentCode fork SHA

expected upstream update strategy
```

---

# 62. Phase 4.2 — Provider Abstraction

Normalize providers into:

```text
provider_id

connection_id

account/project

credential reference

quota domain

supported models

health

latency

cost data
```

Credentials are references, not raw database plaintext if avoidable.

---

# 63. Phase 4.3 — Initial Providers

Implement enough diverse providers to prove architecture.

The goal is not maximum provider count yet.

A good initial set should include multiple failure domains, for example:

```text
ModelScope

Groq

Cerebras

NVIDIA

Gemini/API route

OpenRouter or equivalent aggregator

DeepSeek paid reserve

Ollama local
```

Actual availability is validated at implementation time.

---

# 64. Phase 4.4 — Model Catalog

Represent:

```text
model family

context window

coding score

reasoning score

vision

tool use

structured output

price

provider routes

privacy/trust class
```

Initial quality scores are hand-curated.

Historical scores come later.

---

# 65. Phase 4.5 — Model Broker

Model Broker inputs:

```text
role

task type

complexity

risk

required context

vision requirement

tool requirement

privacy

budget
```

Output:

```text
ranked candidate model families
```

OmniRoute then resolves actual provider connections.

---

# 66. Phase 4.6 — Routing Profiles

Implement:

```text
FREE_ONLY

FREE_FIRST

LOCAL_FIRST

QUALITY_FIRST

PAID_ALLOWED

OFFLINE
```

These are policies, not separate architectures.

---

# 67. Phase 4.7 — Failure Normalization

All provider/model failures map to stable categories:

```text
RATE_LIMIT

QUOTA_EXHAUSTED

AUTH_FAILED

TIMEOUT

MODEL_UNAVAILABLE

PROVIDER_UNAVAILABLE

CONTEXT_LIMIT

BAD_RESPONSE

TOOL_CALL_FAILURE
```

---

# 68. Phase 4.8 — Circuit Breakers

Track failure history.

Example:

```text
NVIDIA
5 transient failures
→ cooldown 10 min
```

Do not repeatedly hammer a dead provider.

---

# 69. Phase 4.9 — Local Models

Register available Ollama models.

V1 baseline roles:

```text
llama3.2:1b
sentinel / summarization

qwen3:4b
routing/local critic

qwen2.5-coder:3b
small mechanical coding

gemma3:4b
visual QA
```

Resource loading is optimized later.

---

# 70. Phase 4.10 — Routing Evidence

Every decision stores:

```text
task

candidate list

selected model

provider

reason

fallback reason

latency

tokens

cost
```

This data later feeds empirical routing.

---

# 71. Phase 4.11 — Failure Simulation

Tests must deliberately trigger:

```text
429

timeout

invalid credentials

model removed

context overflow

all-free unavailable
```

Expected behavior:

```text
fallback or controlled BLOCKED state
```

not corrupted mission state.

---

# 72. Phase 4 Exit Gate

A test request can survive at least one provider failure without caller-side redesign.

The caller sees:

```text
inference request
→ usable result
```

regardless of which provider ultimately served it.

---

# PHASE 5 — NATIVE TOOL RUNTIME FOUNDATION

# 73. Objective

Build deterministic capabilities that let models perform real engineering while remaining inside AgentCode policy.

This phase must be strong enough that later Worker quality is not limited by weak tools.

---

# 74. Phase 5.1 — Tool Registry

Every tool records:

```text
identifier

schema

read/write behavior

workspace scope

network requirement

risk

reversibility

role permissions
```

---

# 75. Phase 5.2 — Tool Broker

Central flow:

```text
tool request
    ↓
schema validation
    ↓
role capability
    ↓
workspace/policy check
    ↓
risk classification
    ↓
sandbox
    ↓
execution
    ↓
structured result
```

All tools, including MCP later, must ultimately obey this authority.

---

# 76. Phase 5.3 — Filesystem Primitives

Implement reliable:

```text
list_directory

stat

read_file

read_range

glob

create_file

write_file

move

rename

copy

delete
```

Requirements include:

```text
path normalization

symlink resolution

encoding handling

permission preservation

binary detection
```

---

# 77. Phase 5.4 — Workspace Boundaries

Test:

```text
../ traversal

absolute external paths

symlink escape

nested symlink escape
```

Read access and write access may have different allowed roots.

---

# 78. Phase 5.5 — Exact Search

Wrap ripgrep behind:

```text
search_text()
```

Support:

```text
literal

regex

glob scope

file filters

context lines

result limit
```

Raw CLI output should not be the Agent API.

---

# 79. Phase 5.6 — Shell Execution

Implement structured execution:

```text
argv

cwd

env refs

timeout

network policy

background flag
```

Prefer direct argv execution over arbitrary shell strings where possible.

---

# 80. Phase 5.7 — Process Manager

Maintain:

```text
process_id

task

command

PID

state

logs

ports

start time
```

Support:

```text
inspect

wait

cancel

terminate

background
```

---

# 81. Phase 5.8 — Risk Policy

Implement R0–R4.

Examples:

```text
R0
read/search/status

R1
worktree edits/tests/build

R2
bulk local changes/dependency changes

R3
remote mutation

R4
destructive production/high-impact
```

---

# 82. Phase 5.9 — Role Profiles

Even before complete agents exist:

```text
Planner
read/search

Worker
read/write/shell/test

Researcher
read/web later

Verifier
read/test
```

---

# 83. Phase 5.10 — Secret Broker Skeleton

Implement:

```text
secret references

runtime environment injection

redaction
```

The initial system may support environment-backed secrets before OS keychain integration.

---

# 84. Phase 5.11 — Tool Result Model

Every invocation produces:

```text
tool_call_id

status

structured result

raw output reference

compressed summary optional

duration

artifacts

warnings
```

---

# 85. Phase 5 Exit Gate

A deterministic harness can safely:

```text
inspect repository

search

modify temporary workspace

run command

run test

capture output
```

while unable to escape assigned workspace without explicit policy.

---

# PHASE 6 — BASIC WORKER AGENT LOOP

# 86. Objective

Build one excellent coding Worker before attempting sophisticated multi-agent orchestration.

AgentCode must prove that:

```text
model + tools + task
```

can perform useful coding work reliably.

---

# 87. Phase 6.1 — Agent Session Model

A session stores:

```text
agent_session_id

role

task

model

provider

context

tools

started_at

status
```

Sessions are temporary; task state is not.

---

# 88. Phase 6.2 — Tool Loop

Implement:

```text
model request
    ↓
assistant reasoning/output abstraction
    ↓
tool calls
    ↓
tool results
    ↓
next model request
```

Support multiple tool calls where provider/model protocol permits.

---

# 89. Phase 6.3 — Worker Instructions

Worker receives:

```text
task

acceptance criteria

workspace

allowed tools

project rules

completion expectations
```

It must not be told to run a fixed script of actions.

It should be able to explore.

---

# 90. Phase 6.4 — Initial Editing

Before the advanced Edit Engine exists, implement at least one robust editing method.

Prefer:

```text
validated patch/search-replace
```

over naive complete-file rewriting.

---

# 91. Phase 6.5 — Test Loop

Worker should autonomously:

```text
inspect

edit

run targeted command

observe failure

repair

rerun
```

without the user typing "continue."

---

# 92. Phase 6.6 — Completion Claim Protocol

Worker cannot directly transition task state.

Instead it emits:

```text
TASK_COMPLETION_REQUEST

summary

evidence references
```

Kernel decides whether to accept.

Full verifier arrives later.

---

# 93. Phase 6.7 — Basic Anti-Loop

Track identical tool/action patterns.

Initial implementation can be rule-based.

---

# 94. Phase 6.8 — Worker Benchmark

Fixture task should require:

```text
at least 3 relevant files

existing test failure

real repair

test pass
```

Measure:

```text
calls

tokens

tool count

retries

final diff quality
```

---

# 95. Phase 6 Exit Gate

AgentCode can autonomously solve a nontrivial repository fixture task using real tools.

A single chat response that generates code without applying/testing it does not satisfy this phase.

---

# PHASE 7 — GIT, WORKTREES & CHECKPOINTING

# 96. Objective

Turn Worker modifications into isolated, recoverable engineering work.

---

# 97. Phase 7.1 — Git Service

Normalize:

```text
status

diff

log

show

branch

commit

restore

worktree

merge

cherry-pick
```

---

# 98. Phase 7.2 — Worktree Registry

Kernel records:

```text
worktree_id

mission

task

branch

path

base_commit

owner

status
```

---

# 99. Phase 7.3 — Task Worktree Creation

Task assignment:

```text
select approved base

create task branch

create worktree

register ownership

initialize repository intelligence overlay later
```

---

# 100. Phase 7.4 — Checkpointing

Worker can checkpoint after meaningful state.

A checkpoint includes:

```text
Git SHA or diff snapshot

task state

test summary

current blocker

context snapshot reference
```

---

# 101. Phase 7.5 — Worker Replacement

Test:

```text
Worker A
edits + checkpoint

Worker A killed

Worker B
opens same task/worktree
continues
```

No manual user intervention.

---

# 102. Phase 7.6 — Rollback

Support task-level restore to:

```text
last checkpoint

base

selected prior checkpoint
```

without affecting unrelated worktrees.

---

# 103. Phase 7.7 — Integration Branch

For multi-task missions:

```text
agentcode/mission-<id>/integration
```

is a strong default.

Verified changes later merge into it.

---

# 104. Phase 7.8 — Conflict Detection

Never resolve conflicts with blanket "ours/theirs."

Create explicit conflict result containing:

```text
files

base

ours

theirs

related tasks
```

---

# 105. Phase 7 Exit Gate

Demonstrate:

```text
two task worktrees

isolated edits

checkpoint

Worker replacement

one successful integration

one intentional conflict detected safely
```

---

# PHASE 8 — CODE INTELLIGENCE FOUNDATION

# 106. Objective

Stop relying on LLM-led repository exploration for basic structural facts.

The repository should become a persistent indexed system.

---

# 107. Phase 8.1 — Repository Identity

Implement stable ID derived from appropriate combination of:

```text
canonical root path

remote identity

repository metadata
```

Do not key state only by folder name.

---

# 108. Phase 8.2 — Inventory Scanner

For each file:

```text
path

relative path

extension

language

size

hash

line count

binary

generated

test

config

docs

Git tracked/status
```

---

# 109. Phase 8.3 — Ignore Engine

Merge:

```text
.gitignore

.git/info/exclude

.agentcodeignore

known generated/vendor paths
```

with explicit user override.

---

# 110. Phase 8.4 — Language Detection

Initial adapters:

```text
TS/JS

Python

Rust

Go

SQL

Shell

Terraform

JSON

YAML

TOML

Markdown
```

---

# 111. Phase 8.5 — Tree-sitter Parser Manager

Handle:

```text
grammar registration

parse

incremental update

parse failure

language fallback
```

---

# 112. Phase 8.6 — Symbol Store

Store:

```text
qualified name

kind

file

range

signature

parent

visibility

fingerprint
```

---

# 113. Phase 8.7 — Import/Export Extraction

Create first graph edges.

---

# 114. Phase 8.8 — ast-grep

Expose:

```text
search_structural()
```

with safe limits.

---

# 115. Phase 8.9 — Repo Map

Build compact structural summary inspired by Aider but AgentCode-owned.

Initial relevance signals:

```text
task terms

imports

references available

Git recency

changed files

symbol visibility
```

---

# 116. Phase 8.10 — Incremental Index

On reopen:

```text
hash unchanged
→ reuse index
```

On modification:

```text
invalidate only file/symbol records
```

---

# 117. Phase 8.11 — File Watcher

Detect external changes.

Watch events are hints; periodic consistency checks prevent missed events.

---

# 118. Phase 8.12 — Performance Benchmark

Representative:

```text
1K files

10K files

large monorepo fixture
```

Measure:

```text
bootstrap

reopen

single-file update

symbol lookup
```

---

# 119. Phase 8 Exit Gate

AgentCode can deterministically answer:

```text
What files exist?

Which language is this file?

Where is AuthService?

Which symbols are in file X?

What files import Y?

Which files changed?
```

without asking an LLM to scan everything.

---

# PHASE 9 — SEMANTIC INTELLIGENCE & REPOSITORY GRAPH

# 120. Objective

Upgrade syntax understanding into project-aware semantics.

---

# 121. Phase 9.1 — LSP Manager

Responsibilities:

```text
detect server

install/detect availability

spawn

initialize workspace

route requests

health check

restart

shutdown
```

---

# 122. Phase 9.2 — Initial Language Servers

Prioritize:

```text
TypeScript/JavaScript

Python

Rust

Go
```

based on AgentCode target repositories.

---

# 123. Phase 9.3 — Semantic API

Normalize:

```text
definition

references

implementations

workspace symbols

document symbols

hover/types

diagnostics

rename

call hierarchy
```

Not all servers must support every feature.

---

# 124. Phase 9.4 — LSP Failure Handling

If project is broken:

```text
LSP unavailable
```

must still leave:

```text
Tree-sitter

ripgrep

repo map

Git
```

working.

---

# 125. Phase 9.5 — SCIP Prototype

Evaluate actual value on representative languages.

Measure:

```text
semantic accuracy

indexing complexity

runtime cost

coverage vs LSP
```

SCIP becomes optional if benefit is limited.

---

# 126. Phase 9.6 — Zoekt Prototype

Benchmark:

```text
index size

RAM

initial indexing

query latency
```

against ripgrep.

Define activation threshold based on evidence.

---

# 127. Phase 9.7 — Unified Repository Graph

Normalize:

```text
Tree-sitter edges

LSP edges

SCIP edges

Git

test mappings

API/schema relationships
```

into one graph.

---

# 128. Phase 9.8 — Confidence

Every edge records:

```text
source

confidence

freshness

commit/worktree
```

An LLM inference does not receive the same confidence as an LSP reference.

---

# 129. Phase 9.9 — Workspace/Monorepo Detection

Detect:

```text
npm/pnpm/yarn workspace

Turborepo

Nx

Cargo workspace

Go workspace

Python package roots
```

Represent package boundaries explicitly.

---

# 130. Phase 9.10 — API & Schema Intelligence

Add adapters for common patterns:

```text
REST routes

OpenAPI

GraphQL

ORM schema

SQL migrations

environment templates
```

This does not need perfect universal static analysis.

The goal is high-value system relationships.

---

# 131. Phase 9.11 — Test Relationships

Map tests using:

```text
imports

symbol references

naming

runtime stack traces

coverage if available
```

---

# 132. Phase 9 Exit Gate

AgentCode can trace at least representative flows such as:

```text
UI component
→ client function
→ API route
→ service
→ database
```

on supported fixture repositories.

---

# PHASE 10 — PERSISTENT MEMORY & KNOWLEDGE FRESHNESS

# 133. Objective

Make repository understanding and project knowledge survive sessions without becoming stale hallucinated documentation.

---

# 134. Phase 10.1 — Knowledge Model

Implement:

```text
fact_id

statement/type

source

evidence

confidence

observed_commit

freshness

scope

last_validation
```

---

# 135. Phase 10.2 — Fact Sources

Normalize:

```text
USER_REQUIREMENT

ARCHITECTURE_DECISION

TREE_SITTER

LSP

SCIP

TEST

RUNTIME

GIT

LLM_INFERENCE
```

---

# 136. Phase 10.3 — Freshness Engine

When evidence changes:

```text
identify facts referencing affected files/symbols

mark POSSIBLY_STALE

revalidate selectively
```

Deletion of required symbol may mark:

```text
INVALID
```

---

# 137. Phase 10.4 — Conflict Handling

If:

```text
Fact A:
Service X owns authentication.

Fact B:
Service Y owns authentication.
```

do not silently choose.

Mark conflict and resolve using fresh evidence.

---

# 138. Phase 10.5 — Memory Classes

Implement:

```text
IMMUTABLE_MISSION

DECISION

LONG_LIVED_REPO

TASK_SCOPED

EPHEMERAL
```

Different retention/retrieval behavior applies.

---

# 139. Phase 10.6 — `CONTEXT.md`

Generate compact readable state containing:

```text
repository

mission

requirements

architecture summary

important modules

current task

completed work

remaining work

failed approaches

test state

security state

blockers

next action

evidence pointers
```

---

# 140. Phase 10.7 — `DECISIONS.md`

Maintain append-only/ADR-like project decisions.

---

# 141. Phase 10.8 — Other State Files

Prepare support for:

```text
MISSION.md

REQUIREMENTS.md

TEST_STATE.md

SECURITY_STATE.md

DESIGN_STATE.md
```

Some become fully populated later.

---

# 142. Phase 10.9 — Context History

Before:

```text
major compaction

provider/model replacement

phase transition

mission pause
```

archive snapshot.

---

# 143. Phase 10.10 — Restart Continuity Test

Work substantially on a fixture.

Terminate entire agent session.

Start a different model.

It should reconstruct current state from:

```text
structured state

CONTEXT.md

targeted retrieval
```

rather than transcript replay.

---

# 144. Phase 10 Exit Gate

Knowledge survives restart and properly becomes stale after source changes.

A stale summary must never override current repository evidence.

---

# PHASE 11 — CONTEXT ENGINE & TOKEN EFFICIENCY

# 145. Objective

Deliver deep relevant context without indiscriminate long prompts.

---

# 146. Phase 11.1 — Context Pack Schema

A context pack contains typed sections:

```text
role

task

acceptance criteria

mission subset

project rules

repo map slice

target source

related source

tests

diff

errors

decisions

failed approaches

tool state
```

---

# 147. Phase 11.2 — Relevance Engine

Initial score combines deterministic features.

Example classes:

```text
direct target

user mentioned

symbol relationship

dependency distance

test relationship

runtime error

Git recency

same package

historical successful retrieval
```

Avoid ML ranking initially.

---

# 148. Phase 11.3 — Hard Inclusions

Certain data bypass ranking:

```text
acceptance criteria

explicit user files

current edited files

compile-error locations

direct target definition

project rules
```

---

# 149. Phase 11.4 — Sensitive Exclusion

Before provider transmission:

```text
classify sensitivity

remove/redact secret values

apply provider trust policy
```

---

# 150. Phase 11.5 — Context Profiles

Implement ranges dynamically rather than rigid fixed counts.

Typical:

```text
TINY
simple operation

NORMAL
standard coding

DEEP
cross-module

AUDIT
broad repository

EXTREME
exceptional large-context task
```

---

# 151. Phase 11.6 — Token Budget Manager

Track:

```text
target input

hard ceiling

reserved output

tool result allowance

history allowance
```

A Worker can request expansion if necessary.

---

# 152. Phase 11.7 — Deduplication

If full function source is present:

```text
repo map should reference it
```

rather than duplicate it.

---

# 153. Phase 11.8 — Progressive Retrieval

Expose agent operations:

```text
need_definition

need_references

need_related_tests

need_broader_scope

need_raw_output

need_history
```

These may map to Tool/Context APIs.

---

# 154. Phase 11.9 — RTK

Benchmark and integrate deterministic compression for:

```text
Git

tests

build output

directory listing

logs
```

Raw output remains persisted.

---

# 155. Phase 11.10 — Caveman Comparison

Use a fixed command corpus.

Measure:

```text
compressed size

information loss

model success

debugging recovery
```

Select useful techniques rather than stacking systems.

---

# 156. Phase 11.11 — Provider Prompt Caching

Where supported, order stable context consistently:

```text
system

project rules

stable architecture

task

volatile evidence
```

---

# 157. Phase 11.12 — Context Caching

Cache immutable content keyed by:

```text
commit

file hash

symbol fingerprint

context-pack parameters
```

---

# 158. Phase 11.13 — Context Benchmark

Tasks:

```text
cross-module bug

API migration

architecture explanation

test selection
```

Compare:

```text
broad context
```

against:

```text
targeted retrieval.
```

Evaluate:

```text
success

tokens

retries

latency
```

---

# 159. Phase 11 Exit Gate

AgentCode demonstrates meaningful token reduction without decreasing verified task success.

---

# PHASE 12 — FULL AUTONOMY KERNEL & MULTI-AGENT RUNTIME

# 160. Objective

Transform the Worker from a useful coding agent into one component of a durable autonomous mission system.

---

# 161. Phase 12.1 — Requirement Extraction

Original goal:

```text
immutable
```

Planner/interpreter derives requirements.

Each requirement receives:

```text
ID

description

type

priority

source

verification strategy

blocking flag
```

---

# 162. Phase 12.2 — Requirement Matrix

Track independently:

```text
implementation

verification

evidence

status
```

---

# 163. Phase 12.3 — Planner

Planner outputs structured:

```text
tasks

dependencies

task type

risk

acceptance criteria

skills

context profile
```

Planner cannot directly declare mission complete.

---

# 164. Phase 12.4 — DAG Manager

Validate:

```text
dependencies exist

no cycles

states coherent

superseded tasks preserved
```

---

# 165. Phase 12.5 — Scheduler

Scheduler uses:

```text
readiness

priority

critical path

provider availability

resource availability

conflict risk
```

---

# 166. Phase 12.6 — Worker Registry

Maintain real-time durable worker records.

---

# 167. Phase 12.7 — Leases & Heartbeats

Every Worker assignment has:

```text
lease

expiry

heartbeat interval
```

Expired lease triggers recovery.

---

# 168. Phase 12.8 — Progress Detection

Progress may include:

```text
meaningful diff

new evidence

test improvement

task subgoal completed

requirement advancement
```

Repeated reads do not equal progress.

---

# 169. Phase 12.9 — Stall/Loop Detection

Detect:

```text
same command repeatedly

same error

same files without new insight

edit/revert loop
```

---

# 170. Phase 12.10 — Recovery Engine

Failure classification determines recovery.

Examples:

```text
provider failure
→ switch route

context full
→ compact

Worker crash
→ lease recovery

tool crash
→ restart tool

repeated logic failure
→ alternate model/replan
```

---

# 171. Phase 12.11 — Retry Escalation

Retries must change something.

Suggested ladder:

```text
same Worker with explicit failure evidence

expanded context

different model

different provider/family

Researcher task

Planner reassessment

new repair Worker

human escalation
```

---

# 172. Phase 12.12 — Researcher

Research results contain:

```text
question

sources

findings

recommendation

risks

version/date
```

Persist independently.

---

# 173. Phase 12.13 — Verifier Skeleton

Verifier receives independent context.

It can:

```text
read

search

run tests

inspect diff

raise findings
```

Full verification rules arrive Phase 14.

---

# 174. Phase 12.14 — Mailboxes

Durable messages:

```text
BLOCKER

RESEARCH_RESULT

FINDING

TASK_REQUEST

REPLAN_REQUEST
```

No important agent coordination should exist only in transient model text.

---

# 175. Phase 12.15 — Shared Blackboard

Store mission-level:

```text
blockers

integration constraints

high-risk findings
```

Agents retrieve relevant entries only.

---

# 176. Phase 12.16 — Concurrency

Two tasks may run concurrently only if:

```text
dependency independent

resource independent

conflict risk acceptable
```

Worktrees provide code isolation.

---

# 177. Phase 12.17 — Conflict Prediction

Use repository graph to estimate overlap.

Signals:

```text
same files

same symbols

same package

same schema

same lockfile

dependency proximity
```

---

# 178. Phase 12.18 — Resource Governor

Track actual load.

At minimum:

```text
RAM pressure

CPU

active builds

browser sessions

LSPs

local model
```

Scheduler delays tasks when necessary.

---

# 179. Phase 12.19 — Pause/Resume

Pause:

```text
stop scheduling

checkpoint Workers

persist state

terminate/retain processes safely
```

Resume:

```text
reconcile

refresh indexes/provider health

continue.
```

---

# 180. Phase 12.20 — Human Escalation

Only for:

```text
missing credentials

irreversible decision

production action

budget limit

persistent blocker
```

---

# 181. Phase 12.21 — Replanning

New information may update plan.

Preserve:

```text
old version

reason

superseded tasks

completed evidence
```

---

# 182. Phase 12 Acceptance Mission

Construct mission requiring:

```text
Planner

two parallel Workers

Researcher

Verifier skeleton

provider failover

Worker kill

recovery

replan
```

All state must survive.

---

# 183. Phase 12 Exit Gate

AgentCode can run a long simulated mission without routine user babysitting.

---

# PHASE 13 — ADVANCED EDIT ENGINE

# 184. Objective

Reach editing quality comparable to mature coding agents.

---

# 185. Phase 13.1 — Model Edit Adapters

Map model capabilities to:

```text
search/replace

unified diff

structured edit

whole file

AST

LSP
```

---

# 186. Phase 13.2 — Preconditions

Every edit operates against expected:

```text
file hash

symbol fingerprint

base revision
```

Stale input causes conflict, not overwrite.

---

# 187. Phase 13.3 — Search/Replace

Validate:

```text
match count

expected text

file state
```

Ambiguity requires repair.

---

# 188. Phase 13.4 — Unified Diff

Apply hunks with strict context validation.

Record rejected hunks.

---

# 189. Phase 13.5 — Structured Symbol Edits

Target symbol identity rather than hardcoded line positions where possible.

---

# 190. Phase 13.6 — ChangeSet

Multi-file logical transaction:

```text
prepared

applied

validating

accepted

rolled back
```

---

# 191. Phase 13.7 — Crash Recovery

If daemon dies mid-change:

```text
manifest identifies partial state

reconcile

rollback or finish safely
```

---

# 192. Phase 13.8 — Formatting

Detect project-native formatter.

Run only affected scope when practical.

---

# 193. Phase 13.9 — AST Transformations

Use ast-grep for safe repetitive migration patterns.

---

# 194. Phase 13.10 — LSP Rename

Semantic rename where server quality supports it.

---

# 195. Phase 13.11 — Edit Quality Metrics

Measure by:

```text
first-apply success

syntax failures

retries

unintended diff

verification rejection
```

Store by model/task language.

---

# 196. Phase 13 Exit Gate

Demonstrate:

```text
multi-file transactional edit

external concurrent human edit

crash mid-edit

rollback

AST migration

semantic rename
```

without corrupting repository state.

---

# PHASE 14 — VERIFICATION & EVIDENCE ENGINE

# 197. Objective

Change AgentCode's definition of "done" from model judgment to evidence-backed completion.

---

# 198. Phase 14.1 — Mechanical Gate API

Normalize:

```text
format

lint

typecheck

compile

build
```

---

# 199. Phase 14.2 — Test Runner

Use repository-detected commands.

Persist:

```text
test identities

results

duration

commit

raw output
```

---

# 200. Phase 14.3 — Targeted Test Selection

Doc 02 mappings choose likely relevant tests.

Fallback to broader scope if confidence is low.

---

# 201. Phase 14.4 — Requirement Evidence Links

Requirement:

```text
R17
```

links to:

```text
implementation

test

runtime

Verifier
```

---

# 202. Phase 14.5 — Independent Verifier

Context intentionally excludes excessive Worker self-narrative.

Verifier examines:

```text
actual requirement

actual diff

actual repository

actual tests

actual failures
```

---

# 203. Phase 14.6 — Adversarial Review

Explicitly search for:

```text
placeholder

mock implementation

dead wiring

unregistered route

weak assertions

ignored exception

hidden fallback

missing migration

unhandled edge
```

---

# 204. Phase 14.7 — Test Tampering

Flag suspicious:

```text
test deleted

assertion weakened

skip introduced

fixture changed to avoid failure
```

Require explanation and review.

---

# 205. Phase 14.8 — Evidence Freshness

When source changes:

```text
impact analysis
→ invalidate affected evidence
```

---

# 206. Phase 14.9 — Integration Verification

After merging parallel work:

```text
build integrated state

run affected broad tests

check cross-module contract
```

---

# 207. Phase 14.10 — Incremental Verification

Do not re-review unaffected code.

Use:

```text
verified baseline

new diff

impact graph
```

---

# 208. Phase 14.11 — Final Audit

Final Verifier receives:

```text
original goal

requirements

final repository state

test state

security state

limitations

accepted risk
```

---

# 209. Phase 14.12 — Completion Gate

Kernel allows mission `COMPLETE` only after required conditions pass.

---

# 210. Phase 14 Acceptance Fixtures

Seed:

```text
false "done"

unwired backend

weak test

test skip

stale previous pass

integration regression
```

AgentCode must catch them.

---

# 211. Phase 14 Exit Gate

A Worker cannot trick the system into mission completion through persuasive text.

---

# PHASE 15 — BROWSER RUNTIME & VISUAL VERIFICATION

# 212. Objective

Give AgentCode the ability to test applications the way a user actually experiences them.

---

# 213. Phase 15.1 — Playwright Service

Wrap:

```text
browser install/detection

launch

contexts

pages

close
```

---

# 214. Phase 15.2 — Browser Sessions

Track:

```text
browser_session_id

task

process

URL

profile

storage state
```

---

# 215. Phase 15.3 — Actions

Expose:

```text
open

click

type

select

scroll

wait

inspect DOM

accessibility tree
```

---

# 216. Phase 15.4 — Developer Evidence

Capture:

```text
console errors

page errors

network failures

HTTP status

screenshots
```

---

# 217. Phase 15.5 — Dev Server Integration

Worker can:

```text
start server

wait until ready

browser test

retain process

clean up
```

---

# 218. Phase 15.6 — Agentic Browser Evaluation

Use Browser Use/Harness only for:

```text
unknown navigation

dynamic UI discovery

semantic exploration
```

Deterministic Playwright remains default.

---

# 219. Phase 15.7 — Visual QA

Pass screenshots to visual model.

Use an available low-footprint local visual route where it passes benchmark and resource gates (for example a Gemma3-class 4B route if available and suitable).

The specific local model is dynamic runtime data rather than a roadmap invariant.

Escalate to a stronger trusted route when the design is high-impact or the local result is uncertain.

---

# 220. Phase 15.8 — Responsive Test Harness

Support named viewport groups.

Store screenshots/evidence per viewport.

---

# 221. Phase 15 Exit Gate

AgentCode can launch a fixture web application and independently verify a meaningful user flow.

---

# PHASE 16 — SKILLS, HOOKS & MCP

# 222. Objective

Make AgentCode extensible without making every Worker prompt enormous.

---

# 223. Phase 16.1 — Skill Registry

Each skill:

```text
id

version

source

scope

trust

trigger

required tools

context cost
```

---

# 224. Phase 16.2 — `SKILL.md`

Define AgentCode-native portable structure.

---

# 225. Phase 16.3 — Progressive Loading

Models initially see only:

```text
skill name + purpose
```

Full instructions loaded only when selected.

---

# 226. Phase 16.4 — Core Skills

Implement high-value built-ins:

```text
debugging

testing

Git

React

Next.js

Rust

Python

browser QA

security

design
```

---

# 227. Phase 16.5 — Importers

Normalize compatible external formats.

Never allow imported skills to supersede Kernel/security authority.

---

# 228. Phase 16.6 — Hook Engine

Implement typed lifecycle events.

---

# 229. Phase 16.7 — Initial Hooks

Examples:

```text
BeforeTool

AfterTool

AfterEdit

BeforeTest

AfterTest

BeforeTaskComplete

BeforeMissionComplete
```

---

# 230. Phase 16.8 — Hook Safety

Hook:

```text
→ Tool Broker
→ policy
→ sandbox
```

No privileged bypass.

---

# 231. Phase 16.9 — MCP Client

Support:

```text
stdio

stream transports supported by standard

tool discovery

resource discovery

invocation
```

---

# 232. Phase 16.10 — MCP Trust

New server:

```text
UNTRUSTED/LIMITED
```

until configured.

Capabilities sent to agents only when task relevant.

---

# 233. Phase 16 Exit Gate

AgentCode can add a skill, lifecycle hook and MCP server without modifying Kernel code.

---

# PHASE 17 — BASELINE SECURITY PLATFORM

# 234. Objective

Build an integrated security workflow rather than a scanner-launcher UI.

---

# 235. Phase 17.1 — Security Finding Database

Normalize:

```text
severity

confidence

exploitability

source

affected code

evidence

state

remediation
```

---

# 236. Phase 17.2 — Security Orchestrator

Determine relevant checks from:

```text
languages

framework

changed files

mission

infrastructure
```

---

# 237. Phase 17.3 — Threat Model V1

Use Code Intelligence to detect:

```text
entry points

auth boundaries

data stores

admin operations

cloud configuration

sensitive assets
```

---

# 238. Phase 17.4 — Gitleaks

Scan:

```text
working tree

Git history
```

Redact actual values.

---

# 239. Phase 17.5 — OSV/Trivy

Normalize dependency vulnerabilities.

Avoid duplicate execution where not useful.

---

# 240. Phase 17.6 — Semgrep

Run relevant rule packs.

Store raw report.

---

# 241. Phase 17.7 — IaC

Use Checkov/Trivy based on detected infrastructure.

---

# 242. Phase 17.8 — Triage

LLM Security Verifier examines:

```text
reachability

configuration

false positive likelihood

business context
```

---

# 243. Phase 17.9 — Root Cause Grouping

Multiple findings with same flaw should become one remediation strategy where appropriate.

---

# 244. Phase 17.10 — Reports

Generate:

```text
Markdown

JSON

SARIF
```

---

# 245. Phase 17.11 — Repair Workflow

Confirmed finding:

```text
Security task
→ Worker
→ test
→ rescan
→ Verifier
```

---

# 246. Phase 17 Exit Gate

Seeded vulnerable fixture produces:

```text
candidate findings

triage

confirmed findings

report

repair

reverification
```

---

# PHASE 18 — ADVANCED APPSEC, CLOUD & RED-TEAM

# 247. Objective

Add controlled adversarial validation and attack-chain reasoning.

---

# 248. Phase 18.1 — Environment Classification

Before active testing:

```text
LOCAL

TEST

STAGING

AUTHORIZED_LAB

PRODUCTION_READ_ONLY

PRODUCTION_ACTIVE_APPROVED
```

---

# 249. Phase 18.2 — ZAP

Integrate browser/web DAST.

Apply:

```text
scope

rate limit

authentication profile
```

---

# 250. Phase 18.3 — Nuclei

Pin:

```text
engine version

template commit
```

Apply scope before execution.

---

# 251. Phase 18.4 — Safe Exploit Validator

Goal:

```text
prove minimum necessary effect
```

using:

```text
synthetic account

canary record

test resource
```

---

# 252. Phase 18.5 — Attack Graph

Combine:

```text
entry point

finding

permission

asset

impact
```

into chained paths.

---

# 253. Phase 18.6 — Cloud Posture

Prowler becomes primary broad cloud adapter.

Use read-only credentials where possible.

---

# 254. Phase 18.7 — Complementary Cloud Tools

ScoutSuite/CloudSploit run only when complementary signal justifies overhead.

---

# 255. Phase 18.8 — Cloud Attack Reasoning

Model:

```text
principal

permissions

trust

resource

escalation
```

---

# 256. Phase 18.9 — Stratus

Use controlled techniques only against authorized lab/test environments.

---

# 257. Phase 18.10 — CloudGoat

Use as repeatable AgentCode benchmark.

---

# 258. Phase 18.11 — Pacu

Integrate only inside explicit authorized/lab policy.

Never default production tool.

---

# 259. Phase 18.12 — Cleanup

After red-team session:

```text
stop processes

delete temporary test resources

revoke temporary credentials

verify teardown
```

---

# 260. Phase 18 Exit Gate

AgentCode identifies a seeded multi-step attack path, safely proves it in a disposable environment and cleans up afterward.

---

# PHASE 19 — AI SECURITY

# 261. Objective

Secure repositories containing LLM/RAG/agent/MCP functionality and test AgentCode itself.

---

# 262. Phase 19.1 — AI Surface Detection

Identify:

```text
LLM SDK

RAG

vector retrieval

MCP

tool calling

agent frameworks
```

---

# 263. Phase 19.2 — Promptfoo Adapter

Use for structured AI test matrices and red-team suites.

---

# 264. Phase 19.3 — Garak Adapter

Use broad vulnerability probes.

---

# 265. Phase 19.4 — PyRIT Adapter

Use more sophisticated adversarial orchestration where justified.

---

# 266. Phase 19.5 — Attack Categories

Cover:

```text
direct injection

indirect injection

system leakage

secret leakage

RAG poisoning

tool misuse

cross-agent manipulation

MCP trust abuse

excessive agency
```

---

# 267. Phase 19.6 — Synthetic Tools

Build safe malicious-test tool fixtures.

Example:

```text
delete_all_test_records
```

should be rejected by policy in protected scenario.

---

# 268. Phase 19 Exit Gate

AI security fixture demonstrates at least:

```text
attack detection

evidence

finding

repair or mitigation

regression
```

---

# PHASE 20 — DISCUSS MODE

# 269. Objective

Expose repository understanding as a high-quality engineering conversation without making it another independent architecture.

---

# 270. Phase 20.1 — Discussion Session

Persist:

```text
project

messages

selected context

decisions
```

Conversation itself may compact.

---

# 271. Phase 20.2 — Repository-Aware Answers

Context Engine provides:

```text
actual source

repo map

decisions

tests

Git
```

based on question.

---

# 272. Phase 20.3 — Read-Only Policy

Discuss cannot mutate source by default.

It may propose patches/plans.

---

# 273. Phase 20.4 — Research

When public/current information is needed:

```text
Researcher task
```

can augment discussion.

---

# 274. Phase 20.5 — Decision Promotion

User-approved discussion output becomes:

```text
decision record
```

instead of remaining buried in transcript.

---

# 275. Phase 20.6 — Plan Promotion

Convert discussion into:

```text
requirements

tasks

constraints
```

---

# 276. Phase 20.7 — Execute

Create standard Kernel mission.

No separate Discuss executor.

---

# 277. Phase 20 Exit Gate

A repository-aware architecture discussion can become an autonomous mission without restating context manually.

---

# PHASE 21 — DESIGN STUDIO

# 278. Objective

Create a design system capable of autonomous high-quality UI generation and repair.

---

# 279. Phase 21.1 — Existing Product Analysis

Inspect:

```text
routes

screens

components

global CSS

tokens

fonts

Tailwind/theme

screenshots
```

---

# 280. Phase 21.2 — Design Brief

Generate structured:

```text
product

audience

personality

density

primary workflow

visual goals

patterns to avoid
```

---

# 281. Phase 21.3 — Design Grammar

Persist:

```text
type scale

spacing

radii

surfaces

color roles

navigation

motion

iconography

component principles
```

---

# 282. Phase 21.4 — Anti-Slop Critic

Explicitly score/flag:

```text
generic card grids

gratuitous gradients

huge empty heroes

generic AI copy

unmodified default components

unjustified glass effects

everything rounded
```

The critic must explain why something is weak, not ban patterns mechanically.

---

# 283. Phase 21.5 — Implementation Worker

Use normal Worker plus:

```text
design

frontend

accessibility

browser
```

skills.

---

# 284. Phase 21.6 — Preview Loop

```text
implement
→ launch
→ screenshot
→ inspect
→ critique
→ repair
```

automatically repeated.

---

# 285. Phase 21.7 — Visual Model

Use cheap local visual QA first.

Escalate for high-value design comparison.

---

# 286. Phase 21.8 — Accessibility

Verify:

```text
keyboard

labels

focus

semantics

contrast

responsive behavior
```

---

# 287. Phase 21.9 — Functional Preservation

Existing important flows must still pass.

---

# 288. Phase 21.10 — Design State

Persist:

```text
DESIGN_STATE.md

structured design facts
```

---

# 289. Phase 21.11 — Reference Images

Extract design principles.

Do not clone proprietary assets.

---

# 290. Phase 21.12 — Direct Visual Selection Prototype

For supported React/Next.js setups:

```text
DOM element
→ source mapping
→ component/file
```

Inspired by Onlook concepts.

This may ship as limited V1 capability if robust enough.

---

# 291. Phase 21 Exit Gate

AgentCode redesigns a real fixture/application through at least two autonomous visual iterations and passes responsive + functional QA.

---

# PHASE 22 — MINIMAL DESKTOP PRODUCT EXPERIENCE

# 292. Objective

Expose the mature runtime through a calm, minimal interface.

This phase is deliberately late because UI must reflect real Kernel state rather than simulate it.

---

# 293. Phase 22.1 — Tauri Shell

Implement:

```text
window lifecycle

daemon connection

events

theme

navigation
```

---

# 294. Phase 22.2 — Projects

Support:

```text
open project

recent projects

repository status
```

---

# 295. Phase 22.3 — Goal Composer

Input:

```text
goal

attachments/references

optional advanced settings
```

---

# 296. Phase 22.4 — Mission View

Default surface:

```text
goal

status

real progress

current action

blocker

no-action-required indicator
```

---

# 297. Phase 22.5 — Activity

Compress noisy events.

Example:

```text
Inspected 17 authentication files
```

expandable to raw events.

---

# 298. Phase 22.6 — Changes

Grouped diff with:

```text
task

file

verification state

added/deleted lines
```

---

# 299. Phase 22.7 — Details

Advanced panels:

```text
requirements

task DAG

models

providers

context

worktrees

tool calls

evidence
```

Hidden by default.

---

# 300. Phase 22.8 — Discuss UI

Repository-aware conversation surface.

---

# 301. Phase 22.9 — Design UI

Preview-centric.

---

# 302. Phase 22.10 — Security UI

Show:

```text
scope

security mode

findings

attack paths

remediation state
```

---

# 303. Phase 22.11 — Settings

Include:

```text
provider accounts

model preferences

local models

budgets

autonomy profile

appearance

notifications
```

---

# 304. Phase 22.12 — Notifications

Only:

```text
Mission complete

Needs user

Blocked/failure when meaningful
```

---

# 305. Phase 22.13 — Completion Sound

Short and subtle.

Separate attention sound.

---

# 306. Phase 22.14 — Window vs Daemon Lifecycle

Closing window:

```text
daemon continues.
```

Explicit Quit:

```text
may stop daemon after safe checkpoint.
```

---

# 307. Phase 22.15 — Menu Bar/Tray

Allow:

```text
status

open

pause

resume

quit
```

---

# 308. Phase 22 Exit Gate

A new user can run a mission without understanding internal architecture, while an advanced user can inspect everything when needed.

---

# PHASE 23 — RESOURCE, TOKEN & COST OPTIMIZATION

# 309. Objective

Make the entire system viable on an 8 GB Mac and under free/limited inference.

Optimization occurs after functional architecture exists so measurements reflect real workloads.

---

# 310. Phase 23.1 — Runtime Telemetry

Measure:

```text
RSS memory

CPU

LSP memory

browser memory

local model memory

index memory

process count

disk
```

---

# 311. Phase 23.2 — Local Model Lifecycle

Implement:

```text
lazy load

idle unload

single-model preference under pressure
```

---

# 312. Phase 23.3 — LSP Lifecycle

Unload inactive workspaces.

Restart unhealthy servers.

---

# 313. Phase 23.4 — Index Policy

Zoekt/SCIP only active where benefit exceeds cost.

---

# 314. Phase 23.5 — Context Relevance Tuning

Use benchmark history to adjust ranking weights.

---

# 315. Phase 23.6 — Tool Compression

Measure:

```text
bytes

estimated tokens

loss

model success
```

---

# 316. Phase 23.7 — Agent Fanout

Tune based on task risk.

Do not spawn Planner+3 Reviewers for trivial one-line changes.

---

# 317. Phase 23.8 — Model Escalation

Historical evidence should answer:

```text
Is one stronger call cheaper than repeated cheap failures?
```

---

# 318. Phase 23.9 — Prompt Cache

Exploit stable provider caching where supported.

---

# 319. Phase 23.10 — Cost Dashboard

Track:

```text
free calls

paid spend

tokens per task

tokens per verified task

cost per verified task
```

---

# 320. Phase 23.11 — Hardware Stress Mission

On real target 8 GB Mac:

```text
repository index

Worker

LSP

tests

browser
```

must coexist under controlled resource policy.

---

# 321. Phase 23 Exit Gate

No normal AgentCode mission should regularly force severe memory pressure on the target hardware.

---

# PHASE 24 — CHAOS ENGINEERING & RECOVERY VALIDATION

# 322. Objective

Prove the AgentCode promise:

> Failures occur, but the mission survives.

Every critical recovery path is deliberately attacked.

---

# 323. Provider Chaos

Test:

```text
429

timeout

connection drop

provider outage

bad auth

model removal

free-provider cascade failure
```

Expected:

```text
fallback

cooldown

paid policy or BLOCKED
```

---

# 324. Model Chaos

Test:

```text
invalid tool call

malformed structured result

premature done

context exhaustion

repeated ineffective behavior
```

---

# 325. Worker Chaos

Kill process mid-task.

Expected:

```text
heartbeat stops

lease expires

recovery

new Worker
```

---

# 326. Planner Chaos

Terminate Planner mid-planning.

New Planner should reconstruct state.

---

# 327. Tool Chaos

Test:

```text
browser crash

LSP crash

scanner crash

command hang

dev server death
```

---

# 328. Edit Chaos

Test:

```text
crash after file 2 of 5

disk error

concurrent human edit

formatter modification

deleted worktree
```

---

# 329. Daemon Chaos

Kill daemon during active mission.

Restart.

Reconcile:

```text
database

worktrees

processes

leases

pending changes
```

---

# 330. Machine Restart

Test actual application/system restart path.

---

# 331. Disk Pressure

Simulate disk nearly/full.

AgentCode must fail safely rather than corrupt state.

---

# 332. SQLite Failure

Test:

```text
transaction interruption

migration failure

integrity failure
```

Recovery procedure must be defined.

---

# 333. Infinite Loop

Deliberately create an agent that repeats actions.

Loop detector must escalate/terminate.

---

# 334. False Completion

Worker + Verifier simulations must not bypass deterministic final gate.

---

# 335. Phase 24 Exit Gate

All critical recovery scenarios defined by Doc 10 pass repeatedly.

A one-time lucky recovery is insufficient.

---

# PHASE 25 — AGENTCODE DOGFOODING

# 336. Objective

Use AgentCode to build AgentCode.

This is the strongest integration test because every subsystem participates.

---

# 337. Dogfood Mission A — Small Bug

AgentCode fixes a real contained bug.

Requirements:

```text
normal Worker

normal worktree

normal verification
```

---

# 338. Dogfood Mission B — Multi-File Feature

AgentCode implements an actual AgentCode feature spanning multiple modules.

---

# 339. Dogfood Mission C — Refactor

Cross-module change tests repository intelligence and context.

---

# 340. Dogfood Mission D — Tests

Ask AgentCode to identify missing coverage for a real subsystem.

---

# 341. Dogfood Mission E — UI

Design Studio improves a real AgentCode interface.

---

# 342. Dogfood Mission F — Security

Run Security Mode on AgentCode.

---

# 343. Dogfood Mission G — Prompt Injection

Place malicious instructions inside:

```text
source comment

README

fixture

scanner output
```

Ensure they do not override policy.

---

# 344. Dogfood Mission H — Malicious MCP

Test overprivileged or malicious server.

---

# 345. Dogfood Mission I — Malicious Skill

Test secret-exfiltration instructions/scripts.

---

# 346. Dogfood Mission J — Long Mission

Run genuinely long unattended mission.

Measure:

```text
human interventions

provider switches

context compactions

Worker replacements

verification failures

total tokens

paid cost

final correctness
```

---

# 347. Dogfood Exit Gate

AgentCode must complete useful changes to itself with no privileged internal shortcuts.

---

# PHASE 26 — SECURITY & LICENSING HARDENING

# 348. Objective

Treat AgentCode itself as a security-sensitive developer tool with access to source code, credentials and shell execution.

---

# 349. Phase 26.1 — Threat Model AgentCode

Assets:

```text
source repositories

provider credentials

Git credentials

cloud credentials

filesystem

MCP integrations

skills
```

---

# 350. Phase 26.2 — Prompt Injection

Attack:

```text
repository

web content

scanner output

MCP

skills

tool logs
```

---

# 351. Phase 26.3 — Workspace Escape

Test:

```text
path traversal

symlinks

nested shell

script indirection

downloaded script
```

---

# 352. Phase 26.4 — Secret Safety

Verify no secret appears in:

```text
LLM payload

event log

tool summary

UI

report
```

unless explicitly permitted.

---

# 353. Phase 26.5 — Dependency Audit

Run AgentCode's own security stack.

---

# 354. Phase 26.6 — External Mutation Audit

Verify Git/cloud/production boundaries.

---

# 355. Phase 26.7 — License Audit

Finalize:

```text
license matrix

THIRD_PARTY_NOTICES

asset review

bundled binary review
```

---

# 356. Phase 26.8 — SBOM

Generate release SBOM.

---

# 357. Phase 26.9 — Security Policy

Prepare `SECURITY.md` before public release.

---

# 358. Phase 26 Exit Gate

No unresolved critical security or licensing blocker remains.

---

# PHASE 27 — PACKAGING, UPDATE & RELEASE ENGINEERING

# 359. Objective

Turn the development system into an installable desktop product.

---

# 360. Phase 27.1 — macOS ARM64 Package

Primary target.

Validate:

```text
installation

launch

permissions

updates if implemented

uninstall
```

---

# 361. Phase 27.2 — Daemon Lifecycle

Define:

```text
launch with app

background continuation

quit

crash restart policy
```

---

# 362. Phase 27.3 — Application Data

Separate:

```text
config

SQLite

index

cache

managed binaries

logs

browser data
```

---

# 363. Phase 27.4 — External SSD Projects

Validate:

```text
spaces in path

external drive reconnect

watcher behavior

Git worktrees

indexes
```

---

# 364. Phase 27.5 — Managed Tool Installer

For selected tools:

```text
discover

install

version

checksum

upgrade

remove
```

---

# 365. Phase 27.6 — First Run

Guide through:

```text
project open

provider connection

Ollama detection

Git availability

browser runtime
```

without forcing configuration of every optional security tool.

---

# 366. Phase 27.7 — Diagnostics

Provide one diagnostics report containing:

```text
platform

AgentCode version

daemon

database

providers

local models

LSPs

tool availability

browser

disk paths
```

with secrets removed.

---

# 367. Phase 27.8 — Reset

Allow:

```text
rebuild indexes

clear cache

reset settings

reset application state
```

without deleting repositories.

---

# 368. Phase 27 Exit Gate

A clean Mac installation can install, configure and run AgentCode without developer tooling beyond documented prerequisites.

---

# PHASE 28 — V1 RELEASE CANDIDATE

# 369. Objective

Freeze feature scope and evaluate the system as a whole.

No new ambitious architecture belongs here.

---

# 370. RC Feature Freeze

Allowed:

```text
bug fix

security fix

performance

reliability

documentation

UX correctness
```

Not:

```text
new orchestration framework

new major mode

new database architecture
```

---

# 371. RC Repository Suite

Run standardized tasks across:

```text
TS

Next.js

Python

Rust

Go

polyglot

monorepo
```

---

# 372. RC Mission Types

Include:

```text
bug fix

feature

refactor

test repair

production-readiness audit

security audit

UI redesign
```

---

# 373. RC Provider Matrix

Test:

```text
preferred free provider

free fallback

local degraded

paid allowed

paid denied

provider cascade failure
```

---

# 374. RC Recovery

Repeat Phase 24 in packaged product.

Development environment success is insufficient.

---

# 375. RC Security

Run:

```text
vulnerable web fixture

cloud lab

AI fixture

AgentCode self-audit
```

---

# 376. RC Design

Run:

```text
greenfield UI

existing UI polish

responsive repair

anti-slop scenario
```

---

# 377. RC Privacy

Capture outbound model requests and verify:

```text
provider trust

secret filtering

context scope
```

---

# 378. RC Hardware

Mandatory:

```text
8 GB Apple Silicon Mac
```

Also test stronger hardware to verify scaling.

---

# 379. RC Documentation

All user-facing flows documented.

---

# 380. RC Release Blockers

Any of these block V1:

```text
mission state loss

routine provider failover failure

false completion

workspace escape

secret leak

half-applied edit corruption

UI closure kills mission

unrecoverable daemon restart

basic Worker unreliable on real repos

critical license violation

critical security issue
```

---

# PHASE 29 — V1 RELEASE

# 381. Objective

Ship only when the product demonstrates the core AgentCode promise in reality.

V1 should not be branded "complete" because:

```text
all roadmap phases have code
```

but because:

```text
all required Doc 10 gates pass.
```

---


# HARDENING NOTE — V1 CHECKLIST INTERPRETATION

The following original V1 checklists remain useful, but release obligation is governed by the hardened Doc 08 capability matrix and the Revision 3 contracts above.

In particular:

```text
Design Studio core workflow
→ REQUIRED_V1

Security baseline
→ REQUIRED_V1

Web DAST / cloud posture / authorized validation
→ REQUIRED_IF_APPLICABLE

baseline AI security
→ REQUIRED_IF_APPLICABLE for AI-enabled repositories

deep red-team lab breadth / advanced PyRIT / universal visual source mapping
→ OPTIONAL_V1
```

P0/P1/P2 sequencing labels do not override those release-scope classes.

---

# 382. V1 Foundation Requirements

Mandatory working:

```text
daemon

Kernel

SQLite

requirements

DAG

Planner

Worker

Researcher

Verifier

leases

heartbeats

recovery
```

---

# 383. V1 Provider Requirements

Mandatory:

```text
multiple providers

Model Broker

OmniRoute

health/fallback

local model support

paid policy

routing logs
```

---

# 384. V1 Repository Requirements

Mandatory:

```text
inventory

ripgrep

Tree-sitter

symbols

repo map

initial LSP

repository graph

Git

tests

incremental indexing

worktree awareness
```

---

# 385. V1 Context Requirements

Mandatory:

```text
purpose-built packs

token budgets

progressive retrieval

persistent knowledge

freshness

CONTEXT.md

compaction

raw evidence

tool compression
```

---

# 386. V1 Engineering Requirements

Mandatory:

```text
filesystem

shell

process manager

Git

worktrees

transactional multi-file editing

tests

builds

browser
```

---

# 387. V1 Verification Requirements

Mandatory:

```text
requirement trace

mechanical gates

tests

Verifier

integration verification

evidence freshness

Final Audit

Kernel completion gate
```

---

# 388. V1 Extensibility Requirements

Mandatory:

```text
skills

hooks

MCP client

role-scoped tools
```

---

# 389. V1 Security Requirements

Mandatory minimum:

```text
threat model

secret scan

dependency scan

SAST

IaC

finding DB

triage

repair workflow

reports
```

Required-if-applicable V1 depth:

```text
web DAST for authorized runnable web targets

cloud posture for explicitly scoped supported cloud environments

attack-path reasoning when multiple security findings/prerequisites compose

authorized validation for local/test/staging/lab targets when the user requests it

baseline AI-security checks when AI/LLM/RAG/MCP surfaces are detected
```

Optional V1 depth:

```text
deep red-team lab automation

broad Pacu/Stratus technique coverage

advanced multi-turn PyRIT orchestration

continuous cloud/security monitoring
```

Release obligation follows the Doc 08 scope matrix. Doc 10 proves the applicable requirement; it does not silently downgrade a Doc 08 `REQUIRED_V1` or `REQUIRED_IF_APPLICABLE` capability.

---

# 390. V1 Product Requirements

Mandatory:

```text
Projects

Goal

Mission

Activity

Changes

Discuss

Security

background operation

notifications

settings
```

Design Studio core workflow is mandatory for V1 and must clear its quality gate.

If the core Design Studio cannot meet its required brief → implementation → real preview → critique → responsive/accessibility/functional verification contract, V1 is not feature-complete. Advanced direct visual element-to-source manipulation remains optional V1 depth; a weak placeholder for that advanced capability should be omitted rather than mislabeled as reliable.

---

# 391. Cross-Phase Parallelization Rules

Parallel work is valuable only where ownership remains clear.

Good examples:

```text
Provider extraction
parallel with
Code Intelligence extraction

Desktop shell skeleton
parallel with
Kernel state

Security adapter research
parallel with
Browser infrastructure

Design skill research
parallel with
Security implementation
```

Bad:

```text
two teams independently implementing Kernel

two context databases

two security finding stores

two Tool Brokers

two Git/worktree managers
```

---

# 392. Single-Owner Responsibility Map

```text
Mission truth
→ Kernel

Requirements
→ Kernel Requirement Manager

Task scheduling
→ Kernel Scheduler

Model selection
→ Model Broker

Provider connections
→ OmniRoute

Repository knowledge
→ Code Intelligence

Persistent project facts
→ Knowledge Store

Prompt contents
→ Context Engine

Actions
→ Tool Broker

Edits
→ Edit Engine

Work isolation
→ Git/Worktree Manager

Verification evidence
→ Verification Engine

Security findings
→ Security Engine

Design workflow
→ Design Studio

UI presentation
→ Desktop App
```

---

# 393. Interface Stabilization Milestones

After these phases, major interface changes should require stronger review:

```text
Phase 4
Model Broker ↔ Provider Fabric

Phase 5
Tool API

Phase 9
Repository Graph API

Phase 11
Context Pack API

Phase 12
Kernel/Agent protocol

Phase 13
Edit ChangeSet API

Phase 14
Evidence API

Phase 17
Security Adapter API

Phase 22
Daemon ↔ Desktop API
```

---

# 394. Continuous Benchmark Program

Benchmarking is not postponed until release.

Every subsystem should produce measurements as soon as meaningful.

---

# 395. Repository Understanding Benchmarks

Tasks:

```text
find target definition

find callers

trace full-stack flow

identify tests

explain package architecture
```

Measure:

```text
correct retrieval

irrelevant retrieval

latency

tokens
```

---

# 396. Editing Benchmarks

Tasks:

```text
small fix

multi-file fix

rename

API migration

structured repetitive migration
```

Measure:

```text
first-pass apply

syntax breakage

retries

unintended diff

Verifier rejection
```

---

# 397. Autonomy Benchmarks

Measure:

```text
mission completion

human interventions

recovery rate

stalls

model replacements

provider replacements
```

---

# 398. Context Benchmarks

Measure:

```text
tokens per verified task

context pack size

duplicate ratio

retrieval count

first-attempt success
```

---

# 399. Security Benchmarks

Use controlled fixtures containing:

```text
true vulnerability

scanner false positive

business logic vulnerability

multi-step attack chain
```

---

# 400. Design Benchmarks

Evaluate:

```text
product fit

generic-pattern frequency

responsive correctness

accessibility

functional preservation
```

Human review can supplement deterministic/model evaluation.

---

# 401. Historical Model Intelligence

For every meaningful agent attempt, store:

```text
model

provider

task type

role

language

framework

context size

success

attempts

Verifier outcome

latency

tokens

cost
```

Over time this data should improve routing.

---

# 402. Initial Routing vs Future Routing

V1:

```text
rules
+
hand-authored capability scores
+
observed health
+
historical statistics
```

Future:

```text
learned routing/bandit optimization
```

Do not overengineer routing before enough data exists.

---

# 403. Documentation Must Track Implementation

At each phase:

```text
update architecture implication

update ADR

update commands

update acceptance evidence

update limitations
```

A roadmap that no longer matches the codebase is dangerous for autonomous agents.

---

# 404. Architecture Deviation Procedure

When implementation reveals a real architectural problem:

```text
1. stop the affected architectural decision

2. gather evidence

3. create ADR

4. evaluate alternatives

5. update core doc

6. update this roadmap if sequencing changes

7. update Doc 10/11 if acceptance/procedure changes

8. resume implementation
```

Do not bury major design changes inside a commit.

---

# 405. Phase Failure vs Engineering Failure

Normal problems such as:

```text
compiler error

test failure

API mismatch

refactor needed
```

do not invalidate the roadmap.

They are normal engineering.

A phase should be considered architecturally blocked only when problems include:

```text
fundamental dependency incompatibility

license conflict

security boundary failure

performance far outside viable range

reference mechanism fundamentally unsuitable

architectural ownership contradiction
```

---

# 406. Development Stop Conditions

Stop and escalate rather than blindly continue when:

```text
data-loss risk

credential exposure risk

unclear destructive behavior

license incompatible

core benchmark fails dramatically

architecture source-of-truth conflict
```

---

# 407. Development Continuation Conditions

Do not stop for ordinary solvable issues.

Once AgentCode can dogfood itself, it should autonomously handle:

```text
compilation

tests

refactors

normal setup

tool version fixes

minor integration conflicts
```

according to normal mission policy.

---

# 408. Non-Negotiable — No Fake Autonomy

AgentCode is **not autonomous** if:

```text
user must type continue

every command needs approval

provider failure ends mission

model context loss restarts task

UI closure ends Worker

LLM final answer directly completes mission
```

---

# 409. Non-Negotiable — No Fake Code Intelligence

AgentCode does not have deep repository understanding if it only:

```text
runs grep

stores embeddings

reads full repository repeatedly

uses old summaries as truth
```

---

# 410. Non-Negotiable — No Fake Memory

Persistent memory is not:

```text
one ever-growing Markdown summary.
```

It requires:

```text
structured facts

evidence

freshness

history

conflict resolution
```

---

# 411. Non-Negotiable — No Fake Multi-Agent System

More agents are not automatically better.

A multi-agent system is meaningful only if:

```text
roles differ

task state persists

Workers can be replaced

parallel work is isolated

Verifier is independent

communication is durable
```

---

# 412. Non-Negotiable — No Fake Verification

Verification is not:

```text
second model says "looks good."
```

It must inspect actual evidence.

---

# 413. Non-Negotiable — No Fake Security

Security cannot be:

```text
run scanners
→ paste output
```

It must include:

```text
triage

confidence

validation

attack-path reasoning

repair

retest
```

---

# 414. Non-Negotiable — No Fake Red Team

Red Team cannot mean uncontrolled offensive automation.

It must include:

```text
authorization

scope

environment classification

safe validation

cleanup

evidence
```

---

# 415. Non-Negotiable — No Fake Design Studio

Design Studio cannot be:

```text
prompt
→ React page
→ done
```

It requires:

```text
product analysis

design brief

design grammar

browser

visual critique

responsive QA

accessibility

functional verification
```

---

# 416. Non-Negotiable — No Fake Token Optimization

One short call does not prove efficiency.

Measure:

```text
tokens per verified task
```

including:

```text
retries

verification

tool output

failed agents
```

---

# 417. Non-Negotiable — No Fake OSS Reuse

A repository being cloned does not mean its mechanism was used.

Every meaningful donor feature needs:

```text
exact source

classification

license

AgentCode owner

integration evidence
```

---

# 418. Non-Negotiable — No Fake Progress UI

A percentage must come from real Kernel state.

Otherwise show a qualitative state.

---

# 419. Non-Negotiable — User Trust

AgentCode must eventually be able to explain:

```text
what changed

why

what evidence exists

what failed

how it recovered

what remains uncertain

what risks were accepted
```

without exposing private hidden reasoning.

---


# HARDENING APPENDIX — OPERATIONAL HANDOFF MATRICES

## HA1. Cross-Phase Parallelization Matrix

The following overlap is encouraged once named interfaces are stable.

| Work | May overlap with | Condition |
|---|---|---|
| P1 provider extraction | P1 code-intel/security/design extraction | donor subsets independent |
| P2 skeleton | late P1 optional extraction | core ADRs/packets approved |
| P4 provider fabric | P5 tools | P3 IDs/events stable |
| P6 Worker benchmark | P7 Git foundations | initial Worker/task contract stable |
| P8 code intelligence | late P6 benchmarks | filesystem/search stable |
| P9 LSP | P10 knowledge schema design | P8 provenance contract stable |
| P11 context | late P10 freshness tests | fact/source interfaces stable |
| P15 browser | P16 skills/hooks/MCP | Tool Broker stable |
| P17 security adapters | P20 Discuss UI/backend | evidence/context interfaces stable |
| P18 security research | P21 Design implementation | browser/resource scheduling coordinated |
| P22 desktop | P23 telemetry instrumentation | mission/UI contracts stable |
| P26 license/security review | late P25 dogfood | production dependency set mostly frozen |
| P27 packaging | late P26 docs/notice generation | no unresolved dependency redesign |

Forbidden parallel duplication:

```text
two Kernels
two Tool Brokers
two Edit journals
two repository graphs
two evidence stores
two security finding authorities
```

---

## HA2. Phase Artifact Families

| Phase family | Durable artifacts |
|---|---|
| P0 | registries, ADRs, scope catalog, governance |
| P1 | extraction reports, adoption decisions, implementation packets |
| P2 | workspace/build/CI/fixtures |
| P3–P7 | DB migrations, protocol contracts, provider/tool/Git runtime |
| P8–P11 | indexes, graph/knowledge/context schemas and benchmarks |
| P12–P14 | Kernel runtime, ChangeSets, evidence/final-audit records |
| P15–P16 | browser/skill/hook/MCP adapters and fixtures |
| P17–P19 | security policy/findings/attack/AI-security evidence |
| P20–P22 | Discuss/Design/UI product records |
| P23–P25 | performance/chaos/dogfood benchmark evidence |
| P26–P29 | SBOM/notices/security/release artifacts |

---

## HA3. Implementation Readiness Rule

A phase or milestone is `READY` when:

```text
hard predecessors satisfied
+
required extraction/adoption packet available
+
input interface version known
+
required fixture/environment exists
+
no known architectural blocker
```

A coding agent must not start by re-deciding the architecture.

If an implementation packet is missing, the correct action is to create/complete the extraction work, not improvise from donor memory.

---

## HA4. Phase Blocker Taxonomy

Use stable categories:

```text
ARCHITECTURE_CONFLICT
LICENSE_BLOCKER
SECURITY_BOUNDARY_FAILURE
PLATFORM_BLOCKER
RESOURCE_VIABILITY_FAILURE
EXTERNAL_PREREQUISITE
BENCHMARK_REJECTION
DATA_LOSS_RISK
SCOPE_DECISION_REQUIRED
```

Normal test/compiler failures stay inside the phase as engineering defects.

---

## HA5. Roadmap Evidence Freshness

A Phase Completion Report can become stale if:

```text
its frozen interface is broken
its required acceptance gate changes
its foundational dependency is replaced
its benchmark environment materially changes
its security/license conclusion is invalidated
```

Stale phase evidence does not automatically set the whole phase to NOT_STARTED.

Instead:

```text
mark affected evidence stale
→ identify impacted milestone/gate
→ rerun targeted validation
→ update completion report revision
```

---

## HA6. Release-Scope Amendment Procedure

If the project proposes to defer a `REQUIRED_V1` capability:

```text
1. identify exact Doc08 requirement/capability
2. state user-visible loss
3. explain why normal engineering repair is insufficient
4. create product/architecture ADR
5. update Doc08 scope matrix
6. update this roadmap
7. update Doc10 gates
8. update Doc11 work packages
9. update UI/docs/marketing claims
10. obtain explicit project approval
```

Difficulty or model failure alone is not a valid automatic deferment.

---

## HA7. Roadmap Quality Invariant

For every future roadmap edit ask:

```text
Does this tell the next engineer what must already be true?
Does it say what is actually built?
Does it say who owns the output?
Does it say how it fails?
Does it say what evidence proves it?
Does it say what later phase may depend on it?
```

If the answer is only:

```text
“Implement X”
```

the roadmap section is still too shallow.

---

## HA8. Revision 3 Summary

Revision 3 preserves the 30-phase foundation of Revision 2 but hardens it with:

1. normative sequencing/dependency classes;
2. explicit phase state semantics;
3. PhaseRecord/MilestoneRecord schemas;
4. workstream view;
5. hard/soft/partial dependency rules;
6. master dependency graph;
7. first-autonomous-mission critical path;
8. MVP-0/Internal Alpha/Feature-Complete Alpha/Beta/RC/V1 milestones;
9. interface freeze points;
10. database/schema migration ownership by phase;
11. constrained implementation-language/package direction;
12. stable fixture/benchmark registry;
13. test-environment matrix;
14. machine-readable Phase Completion Report;
15. roadmap change control;
16. risk register;
17. immediately actionable Phase0/1 start guidance;
18. post-V1 boundary;
19. phase-to-product traceability;
20. phase-to-extraction dependency matrix;
21. detailed implementation-grade contracts for all 30 phases;
22. explicit Design Studio core `REQUIRED_V1` correction;
23. explicit Security baseline `REQUIRED_V1` and applicable advanced-security scoping;
24. canonical Doc07 extraction/catalog names;
25. adaptive concurrency and 8 GB resource rules;
26. mock-vs-live provider testing distinction;
27. thin internal mission UI allowed early while polished product UX remains P22;
28. packaging/signing claim honesty;
29. RC blocker classes;
30. operational parallelization/artifact/blocker/freshness matrices.

The roadmap should now function as the **master dependency and implementation sequencing document**, while Doc 10 defines exact acceptance proof and Doc 11 defines executable Work Packages.

---

# 420. Expected Final Architecture

```text
                              USER
                                │
                                ▼
                     ┌────────────────────┐
                     │    DESKTOP APP     │
                     │ Goal / Discuss /   │
                     │ Design / Security  │
                     └─────────┬──────────┘
                               │
                               ▼
                     BACKGROUND DAEMON
                               │
                               ▼
                     AUTONOMY KERNEL
                               │
          ┌────────────────────┼─────────────────────┐
          ▼                    ▼                     ▼
    Requirements             Tasks              Recovery
          │                    │                     │
          └────────────────────┼─────────────────────┘
                               ▼
                           Scheduler
                               │
             ┌─────────────────┼─────────────────┐
             ▼                 ▼                 ▼
          Planner           Worker          Researcher
             │                 │                 │
             └─────────────────┼─────────────────┘
                               │
                               ▼
                            Verifier
                               │
                               ▼
                         MODEL BROKER
                               │
                               ▼
                     CUSTOM OMNIROUTE
                               │
             ┌─────────────────┼──────────────────┐
             ▼                 ▼                  ▼
        Free Cloud          Local             Paid Reserve
                               │
                               ▼
                       CONTEXT ENGINE
                               │
                               ▼
                    CODE INTELLIGENCE
                               │
       ┌───────────────────────┼────────────────────────┐
       ▼                       ▼                        ▼
 Exact Search              Structural                Semantic
 ripgrep                  Tree-sitter               LSP/SCIP
       │                       │                        │
       └───────────────────────┼────────────────────────┘
                               ▼
                       REPOSITORY GRAPH
                               │
                               ▼
                          TOOL BROKER
                               │
      ┌────────────────────────┼──────────────────────────┐
      ▼                        ▼                          ▼
   Edit/Git                  Shell                     Browser
      │                        │                          │
      └────────────────────────┼──────────────────────────┘
                               ▼
                       IMPLEMENTATION
                               │
                               ▼
                        VERIFICATION
                               │
            ┌──────────────────┼───────────────────┐
            ▼                  ▼                   ▼
          Tests             Security            Visual QA
            │                  │                   │
            └──────────────────┼───────────────────┘
                               ▼
                         FINAL AUDIT
                               │
                               ▼
                      COMPLETION GATE
                        /           \
                      FAIL          PASS
                       │              │
                       ▼              ▼
                     REPAIR       NOTIFY USER
```

---

# 421. Expected Final User Experience

User enters:

```text
Make the authentication subsystem production-ready.

Fix all incomplete behavior, ensure the frontend and
backend are correctly wired, add or repair tests where
needed, audit the security of the flow, and do not stop
until all requirements are verified.
```

AgentCode should then:

```text
persist the original goal

inspect the repository

derive explicit requirements

build task DAG

select Planner/Workers/Verifier

choose appropriate models/providers

build focused context packs

create isolated worktrees

implement changes

run targeted tests

recover from failed providers

replace agents if needed

persist failed approaches

integrate verified changes

run broader verification

perform applicable security checks

run final audit

compare final state against original goal

notify the user
```

The user should mostly experience:

```text
Working...
```

then:

```text
Mission complete.
```

Behind that simple surface may have occurred:

```text
multiple models

multiple providers

context compactions

worktree merges

retries

browser runs

security checks

verification failures

repair cycles
```

The fact that the user did not need to supervise them is precisely the product value.

---

# 422. Master Roadmap Completion Definition

This roadmap is fulfilled only when AgentCode V1 exists as one coherent system demonstrating, on real repositories:

```text
persistent mission state

deep structural repository understanding

semantic code intelligence

incremental indexes

evidence-linked memory

task-specific context

token-aware retrieval

model independence

provider independence

autonomous recovery

real coding capability

high-quality multi-file editing

Git/worktree isolation

process management

browser verification

independent verification

evidence-based completion

native security workflows

authorized adversarial validation

AI security

repository-aware Discuss Mode

product-aware Design Studio

minimal desktop UX

background execution

notifications

resource management

licensing hygiene

packaged macOS operation
```

None of these should exist merely as unconnected demos.

They must operate through the same underlying AgentCode architecture.

---

# 423. Locked Roadmap Principles

1. AgentCode is built foundation-first.

2. Mission state exists outside all LLM sessions.

3. The UI is never mission authority.

4. OSS extraction precedes major subsystem reinvention.

5. The reference library is now sufficient for V1 unless a precise missing capability is discovered.

6. OmniRoute is the strategic provider fork.

7. AgentCode owns the Model Broker.

8. AgentCode owns the Kernel.

9. A strong single Worker precedes sophisticated multi-agent choreography.

10. Tool quality is foundational.

11. Git/worktrees arrive early.

12. Code Intelligence is a core system, not an optional enhancement.

13. Tree-sitter and exact search precede semantic extras.

14. LSP enriches but does not define basic repository understanding.

15. Persistent memory must carry evidence and freshness.

16. `CONTEXT.md` is a handoff view, not truth.

17. Context is task-specific.

18. Large context windows are capacity, not retrieval strategy.

19. Token efficiency is measured across verified work.

20. Multi-agent roles are durable task roles, not virtual employees.

21. Workers are replaceable.

22. Leases and heartbeats make abandoned work recoverable.

23. Advanced editing receives its own architecture.

24. Verification must exist before autonomous completion is trusted.

25. Verifiers should be independent where practical.

26. Browser execution is required for meaningful web/UI validation.

27. Skills, hooks and MCP extend one runtime.

28. Security reuses Kernel, Tool and Context systems.

29. Security scanners are inputs, not truth.

30. Red-team testing is authorization- and scope-aware.

31. Attack chains matter more than isolated alerts.

32. AI applications require AI-specific security.

33. Design Studio uses the same engineering runtime.

34. Design Studio must visually inspect its output.

35. AgentCode explicitly resists generic AI design.

36. Product UI is implemented after runtime truth exists.

37. UI complexity remains lower than backend complexity.

38. Resource usage must remain compatible with the 8 GB target.

39. Optional heavy subsystems are loaded only when useful.

40. Chaos testing is mandatory.

41. AgentCode must dogfood itself.

42. AgentCode must security-test itself.

43. Packaging occurs only after runtime reliability exists.

44. Release candidate means feature freeze.

45. A feature's source code existing does not mean the feature works.

46. Integrated does not mean verified.

47. Verified does not automatically mean mission complete.

48. Every phase requires evidence.

49. Every significant architecture deviation requires an ADR.

50. Every important third-party integration requires a license decision.

51. Every managed external binary requires provenance/version tracking.

52. Every critical recovery path must be tested deliberately.

53. More providers are not automatically better.

54. More models are not automatically better.

55. More agents are not automatically better.

56. More context is not automatically better.

57. More scanners are not automatically better.

58. More UI panels are not automatically better.

59. The primary optimization is reliable verified engineering completion.

60. The user should not need to babysit routine work.

---

# 424. Final Roadmap Statement

AgentCode must be built as a **software-engineering system**, not as a sequence of impressive demos.

The project must never reach a state where:

```text
the interface looks polished
```

but:

```text
closing it kills the mission.
```

It must never reach a state where:

```text
several agents appear to collaborate
```

but:

```text
their shared task state exists only in conversation.
```

It must never claim:

```text
deep repository intelligence
```

while:

```text
continuously rereading the entire repository.
```

It must never claim:

```text
persistent memory
```

when:

```text
one stale Markdown summary silently overrides current code.
```

It must never claim:

```text
verification
```

when:

```text
a second model simply agrees with the first.
```

It must never claim:

```text
security
```

when:

```text
scanner output is merely reformatted.
```

It must never claim:

```text
Red Team
```

when:

```text
the system either refuses to validate anything
or performs uncontrolled destructive actions.
```

It must never claim:

```text
Design Studio
```

when:

```text
one prompt generates one generic React screen
without seeing the rendered result.
```

And most importantly, it must never claim:

```text
Mission complete
```

merely because:

```text
the currently selected LLM stopped generating.
```

The complete engineering progression is therefore:

```text
ARCHITECTURE
      ↓
REFERENCE EXTRACTION
      ↓
LOCAL STATE
      ↓
RELIABLE INFERENCE
      ↓
SAFE TOOLS
      ↓
CAPABLE WORKER
      ↓
WORKTREE ISOLATION
      ↓
CODE INTELLIGENCE
      ↓
SEMANTIC GRAPH
      ↓
PERSISTENT MEMORY
      ↓
EFFICIENT CONTEXT
      ↓
DURABLE AUTONOMY
      ↓
ADVANCED EDITING
      ↓
VERIFICATION
      ↓
BROWSER EXECUTION
      ↓
EXTENSIBILITY
      ↓
SECURITY
      ↓
ADVERSARIAL VALIDATION
      ↓
AI SECURITY
      ↓
DISCUSS
      ↓
DESIGN STUDIO
      ↓
MINIMAL PRODUCT UX
      ↓
OPTIMIZATION
      ↓
CHAOS VALIDATION
      ↓
DOGFOODING
      ↓
HARDENING
      ↓
PACKAGING
      ↓
RELEASE CANDIDATE
      ↓
V1
```

The intended result is:

> **A local-first autonomous software-engineering runtime capable of understanding complex repositories, retaining that understanding across sessions and models, selecting and replacing inference providers automatically, performing serious multi-file engineering work through safe tools and isolated Git worktrees, maintaining persistent evidence and memory, using context efficiently, recovering from failures without routine human supervision, validating its own work independently, reasoning about and repairing security weaknesses, creating and visually verifying professional interfaces, and refusing to declare success until the user's original objective is actually supported by evidence.**

This document is the **V1 Master Project Roadmap, dependency map, milestone specification and primary project-execution sequencing source of truth for AgentCode.**

