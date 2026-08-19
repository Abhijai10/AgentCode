# AgentCode  
# 07 — Implementation & OSS Extraction Blueprint

**Document Status:** V1 — Architecture Locked for Initial Implementation + Hardening Revision 2  
**Date:** 19 August 2026  
**Revision:** Hardening Revision 2 — Extraction Control Specification + Dossier Framework  
**Project:** AgentCode  
**Document Type:** Core Implementation & Open-Source Extraction Specification  

**Depends On:**
- `01 — Model, Provider, Routing & Reliability Architecture`
- `02 — Code Intelligence, Context & Persistent Memory Architecture`
- `03 — Autonomy Kernel & Agent Runtime Architecture`
- `04 — Tool, Edit, Git, Sandbox, Skills & Hooks Architecture`
- `05 — Verification, Security & Red-Team Architecture`
- `06 — Design Studio & Product UX Architecture`

**Primary Reference Repository Root:**  
`/Volumes/T7 Shield/GitHub-Repos-dependency`

**Purpose of This Document:** Define exactly how AgentCode should use the cloned open-source ecosystem without blindly combining frameworks, duplicating mature infrastructure, inheriting incompatible architectures, or wasting development effort rebuilding already-solved primitives.

---

# 1. Purpose

AgentCode is not intended to be implemented by writing every subsystem from zero.

The project has intentionally assembled a large local reference library containing mature implementations of:

- coding-agent runtimes;
- provider routers;
- context engines;
- repository mapping;
- parsing;
- LSP integration;
- shell execution;
- worktrees;
- browser automation;
- memory;
- skills;
- hooks;
- checkpointing;
- security scanners;
- cloud-security systems;
- red-team frameworks;
- application builders;
- UI generation systems;
- evaluation harnesses.

The correct strategy is:

> **Extract the strongest proven implementation patterns from each system, normalize them into one coherent AgentCode architecture, reuse mature external primitives where appropriate, and rewrite only the pieces that define AgentCode's unique behavior.**

The wrong strategy is:

```text
clone 50 repositories
        ↓
copy large chunks together
        ↓
resolve compile errors
        ↓
call it AgentCode
```

That would create a fragile collection of conflicting frameworks rather than a product.

---

# 2. Architectural Authority

The hierarchy is locked:

```text
AGENTCODE CORE DOCS
        ↓
AGENTCODE ARCHITECTURE
        ↓
OSS EXTRACTION DECISIONS
        ↓
REFERENCE REPOSITORIES
```

Reference projects do **not** define AgentCode architecture.

If a reference repository conflicts with Docs 01–06:

```text
Docs 01–06 win.
```

The repository may still contribute an implementation pattern.

---

# 3. Extraction Philosophy

Every repository feature inspected must receive one of the following classifications:

```text
TAKE
ADAPT
WRAP
STUDY
IGNORE
REJECT
```

---

# 4. TAKE

Use when a component is:

- mature;
- appropriately licensed;
- architecture-compatible;
- modular;
- cheaper and safer to reuse directly.

Examples may include:

```text
ripgrep executable
Tree-sitter libraries
Playwright
Git
```

TAKE does not necessarily mean copying source into AgentCode.

It may mean using the upstream library/binary directly.

---

# 5. ADAPT

Use when:

- the implementation pattern is strong;
- AgentCode needs a customized interface or lifecycle;
- upstream architecture cannot be imported unchanged.

Examples:

```text
Aider repo-map ranking
Letta memory hierarchy
Codex approval ideas
Gemini checkpoint concepts
```

---

# 6. WRAP

Use when a mature external tool should remain independent.

Examples:

```text
Semgrep
Trivy
Gitleaks
OSV-Scanner
Prowler
ZAP
Nuclei
```

AgentCode builds:

```text
adapter
policy
result normalization
orchestration
```

around the upstream engine.

---

# 7. STUDY

Use where the repository provides useful ideas but should not become a production dependency.

Examples:

```text
Stack Graphs
Daytona archived architecture
Roo Code historical workflows
```

---

# 8. IGNORE

A feature may be high quality but irrelevant to AgentCode.

Examples:

```text
office visualization
marketing UI
provider-specific product assumptions
hosted SaaS infrastructure not needed by AgentCode
```

---

# 9. REJECT

Use where adopting the design would actively weaken AgentCode.

Examples:

```text
LLM as mission source of truth

unbounded retry loops

PTY-only tool architecture

full-context repository dumping

all-agent global memory

unrestricted shell by default

fake agent personas
```

---

# 10. Extraction Must Be Evidence-Based

No implementation agent may write:

```text
"Codex probably does X."
```

and implement based on assumption.

For every meaningful extraction:

```text
repository
file/module
observed mechanism
AgentCode use
classification
reason
license status
```

must be recorded.

---

# 11. Mandatory Extraction Record

Each extracted mechanism should produce a record similar to:

```text
EXT-042

Subsystem:
Code Intelligence

Source:
Aider

Source Path:
...

Observed Mechanism:
Repository map construction and relevance ranking.

Classification:
ADAPT

AgentCode Destination:
Code Intelligence / RepoMapService

Reason:
Strong compact structural representation but AgentCode requires
additional LSP/SCIP/test/runtime signals.

License Review:
PASS / PENDING / BLOCKED

Implementation Notes:
...
```

---

# 12. Local Reference Repository Library

Current repository root:

```text
/Volumes/T7 Shield/GitHub-Repos-dependency
```

This directory should remain flat.

Do not reorganize repositories physically unless a technical reason appears.

Logical organization belongs in documentation and extraction tooling.

---

# 13. AgentCode Reference Catalog

AgentCode should maintain:

```text
docs/reference/AGENTCODE_REFERENCE_CATALOG.md
```

or equivalent structured database.

It should map every repository to:

```text
category
purpose
priority
license status
upstream URL
local path
extraction status
```

---

# 14. Repository Categories

Current library logically consists of:

```text
MODEL / PROVIDER ROUTING

CODING AGENTS

ORCHESTRATION

EXECUTION / SANDBOX

CODE INTELLIGENCE

MEMORY / CONTEXT

TOKEN EFFICIENCY

SKILLS / WORKFLOWS

BROWSER / QA

DESIGN / APP BUILDERS

APPLICATION SECURITY

CLOUD SECURITY

RED TEAM

AI SECURITY

EVALUATION
```

---

# 15. Source Priority

Repositories should be classified:

```text
P0 — foundational extraction source

P1 — major supporting source

P2 — specialized integration source

P3 — reference-only
```

This prevents agents from treating all repositories equally.

---

# 16. P0 Sources

The highest-priority sources are:

```text
OmniRoute

Codex

Gemini CLI

OpenHands / Software Agent SDK

Aider

mini-SWE-agent

SWE-ReX

Tree-sitter

ast-grep

ripgrep

Letta Code

RTK

Playwright

OpenHack
```

These influence foundational AgentCode runtime behavior.

---

# 17. P1 Sources

```text
OpenCode

Cline

Goose

LangGraph

Microsoft Agent Framework

SCIP

Zoekt

Graphiti

Superpowers

Compound Engineering

Onlook

Dyad

Bolt.diy

Trail of Bits Skills

Prowler
```

---

# 18. P2 Sources

```text
Semgrep

CodeQL

Gitleaks

OSV-Scanner

Trivy

Checkov

ZAP

Nuclei

ScoutSuite

CloudSploit

Stratus Red Team

Pacu

CloudGoat

Promptfoo

Garak

PyRIT

Browser Use

Browser Harness
```

Mostly integrated as specialized tools.

---

# 19. P3 Sources

```text
Continue

Roo Code

Daytona

Stack Graphs

Ctags

Caveman

smolagents

MCP server examples

SWE-bench
```

Some are extremely useful for specific tasks but are not primary architectural foundations.

---

# 20. OmniRoute — Role

**Classification:** `FORK + MODIFY`

AgentCode will not merely call stock OmniRoute.

The known-good OmniRoute version becomes the provider-fabric base.

---

# 21. OmniRoute — Keep

Retain upstream mechanisms for:

```text
provider adapters

provider discovery

model catalog

connection management

quota tracking

cooldowns

health

circuit breakers

latency tracking

cost

fallback

response normalization

Auto-Combo routing infrastructure
```

---

# 22. OmniRoute — Modify

AgentCode requires:

```text
top-K candidates

multi-model fanout

provider diversity

model-family diversity

quota domains

project-specific empirical model scores

role-aware routing

task-risk routing

paid fallback budgets

local Ollama candidates

premature-stop tracking

tool-call reliability

routing explanations

candidate audit trail
```

Doc 01 remains authoritative.

---

# 23. OmniRoute — Must Not Own

OmniRoute must not own:

```text
mission state

task DAG

completion

verification

agent lifecycle
```

Those belong to AgentCode Kernel.

---

# 24. OmniRoute Upstream Strategy

Repository structure:

```text
origin
→ AgentCode customized fork

upstream
→ official OmniRoute
```

Do not blindly merge every new upstream release.

Use:

```text
selective cherry-picks
```

for:

```text
provider fixes

auth changes

new model IDs

security fixes

compatibility updates
```

---

# 25. Munder Difflin — Role

**Classification:** `STUDY + ADAPT`

Useful for:

```text
mailboxes

actor concepts

shared blackboard

supervision

worktree patterns

long-term memory concepts

anti-livelock
```

---

# 26. Munder Difflin — Reject

Do not inherit:

```text
LLM GOD agent as truth

office metaphor

avatar UI

PTY-first architecture

filesystem-only mission truth
```

---

# 27. Aider — Role

**Classification:** `ADAPT HEAVILY`

Primary value:

```text
repo maps

Tree-sitter integration

symbol ranking

compact code context

Git-aware editing

model-specific edit formats
```

---

# 28. Aider — Adaptation

AgentCode extends Aider-style maps with:

```text
LSP

SCIP

test intelligence

runtime evidence

dependency impact

Git recency

schema/API relationships
```

Aider should not become the AgentCode runtime.

---

# 29. LangGraph — Role

**Classification:** `STUDY + SELECTIVE ADAPT`

Study:

```text
checkpoint semantics

graph execution

resume

durability

interruptions

state transitions
```

Do not automatically build AgentCode on full LangGraph.

The Kernel architecture remains intentionally simpler and locally deterministic.

---

# 30. OpenHands / Software Agent SDK — Role

**Classification:** `ADAPT HEAVILY`

Study:

```text
typed actions

observations

workspace

shell

sandbox

tool representation

skills

event model

agent loop
```

Potentially reuse modular libraries where compatibility is excellent.

---

# 31. Codex — Role

**Classification:** `STUDY + ADAPT HEAVILY`

One of the most important references.

Inspect:

```text
filesystem operations

patching

shell execution

sandbox architecture

approval modes

task lifecycle

Git interaction

minimal UX

background execution

agent/tool protocol
```

---

# 32. Codex — AgentCode Goal

AgentCode should achieve comparable fundamental competence at:

```text
read

search

multi-file edit

shell

test

Git

recovery
```

without depending on the Codex product itself.

---

# 33. Gemini CLI — Role

**Classification:** `ADAPT HEAVILY`

Inspect:

```text
checkpointing

hooks

skills

extensions

MCP

context management

policy engine

session resume

project instructions
```

---

# 34. OpenCode — Role

**Classification:** `ADAPT`

Inspect:

```text
LSP

permissions

session handling

tool APIs

plan/build separation

subagent architecture
```

Do not inherit every UX convention.

---

# 35. Cline — Role

**Classification:** `ADAPT`

Inspect:

```text
file editing

checkpoints

browser integration

autonomous workflow

tool UX

worktrees
```

---

# 36. Goose — Role

**Classification:** `ADAPT`

Inspect:

```text
desktop/CLI split

extensions

MCP

local-first execution

provider integration
```

---

# 37. mini-SWE-agent — Role

**Classification:** `ADAPT PRINCIPLES`

The primary lesson is:

> A highly capable coding loop does not need to be architecturally enormous.

Study:

```text
compact loop

tool interaction

environment usage

failure iteration
```

AgentCode's Kernel adds durability outside this loop.

---

# 38. SWE-ReX — Role

**Classification:** `ADAPT / POSSIBLE DIRECT DEPENDENCY`

Inspect:

```text
execution environment abstraction

shell environments

sandbox boundaries

local/remote execution interface
```

Evaluate whether portions can directly serve AgentCode Tool Engine.

---

# 39. Microsoft Agent Framework — Role

**Classification:** `STUDY`

Inspect:

```text
workflow graph

checkpointing

durable execution

multi-agent communication
```

Do not introduce a second mission-runtime truth system.

---

# 40. smolagents — Role

**Classification:** `STUDY`

Useful for:

```text
minimal agent abstractions

tool definition

code agents
```

Mostly conceptual.

---

# 41. Tree-sitter — Role

**Classification:** `TAKE`

Use mature upstream library.

AgentCode should not implement its own parser framework.

AgentCode builds:

```text
language adapters

symbol normalization

incremental index

relationship graph
```

around it.

---

# 42. ast-grep — Role

**Classification:** `TAKE / WRAP`

Use for:

```text
structural search

AST matching

safe repetitive transformation
```

AgentCode wraps with stable internal APIs.

---

# 43. ripgrep — Role

**Classification:** `TAKE`

Use directly for:

```text
fast exact search
```

Do not recreate filesystem text search.

---

# 44. SCIP — Role

**Classification:** `ADAPT / OPTIONAL`

Use where language indexers provide high-quality semantic indexes.

Do not make AgentCode depend on SCIP for basic operation.

---

# 45. Zoekt — Role

**Classification:** `OPTIONAL WRAP`

Enable only for repositories where:

```text
large repository search
```

justifies its overhead.

---

# 46. Universal Ctags — Role

**Classification:** `FALLBACK WRAP`

Use for:

```text
broad language symbol extraction
```

when richer systems unavailable.

---

# 47. Stack Graphs — Role

**Classification:** `STUDY`

Use concepts for:

```text
cross-file symbol resolution

name-resolution graph
```

Do not make an archived project a critical foundational dependency.

---

# 48. Letta Code — Role

**Classification:** `ADAPT HEAVILY`

High-value extraction source for:

```text
persistent memory

Git-backed memory

context hierarchy

memory maintenance

cross-session continuity

structured compaction
```

---

# 49. Letta Core — Role

**Classification:** `STUDY`

Inspect broader:

```text
agent memory APIs

memory persistence

state concepts
```

AgentCode should not pull unnecessary server architecture.

---

# 50. Graphiti — Role

**Classification:** `STUDY / FUTURE`

Potential V2+ value:

```text
temporal knowledge

fact evolution

relationship history
```

V1 should remain SQLite-based unless benchmarks prove otherwise.

---

# 51. RTK — Role

**Classification:** `ADAPT / WRAP HEAVILY`

High priority.

Use for:

```text
command output compression

Git output reduction

test output reduction

build-log compression
```

AgentCode owns raw-output persistence and retrieval.

---

# 52. Caveman — Role

**Classification:** `BENCHMARK AGAINST RTK`

Do not stack compression systems blindly.

Run empirical comparison.

Select strongest mechanisms.

---

# 53. Superpowers — Role

**Classification:** `ADAPT SKILLS`

Study:

```text
TDD

debugging

planning

worktrees

verification-before-completion
```

Convert valuable workflows into AgentCode-native skills.

---

# 54. Compound Engineering Plugin — Role

**Classification:** `ADAPT SKILLS`

Study:

```text
research

planning

review

production workflows

task decomposition
```

---

# 55. Trail of Bits Skills — Role

**Classification:** `ADAPT CAREFULLY`

High-value security workflows.

Use for:

```text
deep audit

false-positive reduction

differential review

security context building

unsafe-default detection
```

License obligations must be reviewed before direct adaptation.

Where required:

```text
reimplement concepts independently
```

rather than copying text.

---

# 56. Letta Skills — Role

**Classification:** `ADAPT SKILL FORMAT / CONTENT SELECTIVELY`

Useful for:

```text
memory

compaction

agent workflow

portable SKILL.md conventions
```

---

# 57. MCP Servers Repository — Role

**Classification:** `STUDY + COMPATIBILITY TEST`

Use official examples for:

```text
MCP protocol

server integration

resources

tools

transports
```

AgentCode should implement its own stable MCP client layer.

---

# 58. Playwright — Role

**Classification:** `TAKE`

Primary deterministic browser engine.

Use upstream package.

AgentCode owns:

```text
browser session manager

task integration

evidence storage

visual QA orchestration
```

---

# 59. Browser Use — Role

**Classification:** `ADAPT`

Study:

```text
agent browser navigation

state representation

browser reasoning

recovery
```

Do not replace deterministic Playwright flows with fully agentic browser behavior when deterministic automation is available.

---

# 60. Browser Harness — Role

**Classification:** `STUDY / ADAPT`

Evaluate:

```text
browser environment abstraction

test harness

agent-browser reliability
```

---

# 61. Onlook — Role

**Classification:** `ADAPT HEAVILY FOR DESIGN STUDIO`

Study:

```text
DOM-to-source mapping

visual element selection

React component mapping

live preview

direct UI editing
```

---

# 62. Dyad — Role

**Classification:** `ADAPT PRODUCT FLOW`

Study:

```text
local prompt-to-app

preview

iteration

BYOK

project generation
```

Any separately licensed commercial/proprietary directory must remain excluded unless licensing is explicitly compatible.

---

# 63. Bolt.diy — Role

**Classification:** `ADAPT WORKFLOW`

Study:

```text
prompt-to-application

generation loop

runtime preview

multi-provider experience
```

Do not assume bundled runtime technologies have the same license as repository source.

---

# 64. Design Builders — Rule

AgentCode should extract:

```text
workflow ideas
```

not become:

```text
a fork of one existing app builder.
```

Design Studio must remain integrated with AgentCode's Kernel, code intelligence and verification systems.

---

# 65. OpenHack — Role

**Classification:** `ADAPT HEAVILY`

Primary security-agent workflow source.

Study:

```text
recon

hunter tasks

validation

verification

sandboxed reproduction

finding lifecycle
```

---

# 66. Semgrep — Role

**Classification:** `WRAP`

AgentCode should call upstream scanner.

Own:

```text
rule selection

scope

result normalization

triage

finding state
```

---

# 67. CodeQL — Role

**Classification:** `WRAP / LICENSE REVIEW REQUIRED`

Use:

```text
query libraries

deep dataflow/taint analysis
```

only under terms compatible with AgentCode's intended usage.

The CodeQL query repository and the distributed CLI/engine must be treated as separate licensing/deployment questions.

Do not bundle CodeQL blindly.

---

# 68. Gitleaks — Role

**Classification:** `WRAP / POSSIBLE BUNDLE`

Use for:

```text
current-tree secrets

Git-history secrets
```

AgentCode must redact secret values.

---

# 69. OSV-Scanner — Role

**Classification:** `WRAP / POSSIBLE BUNDLE`

Use for:

```text
dependency vulnerability matching
```

---

# 70. Trivy — Role

**Classification:** `WRAP`

Use for:

```text
dependency vulnerabilities

container vulnerabilities

misconfigurations

secrets

SBOM-related scanning
```

---

# 71. Checkov — Role

**Classification:** `WRAP`

Use for:

```text
Terraform

CloudFormation

Kubernetes

Helm

Docker

IaC graph analysis
```

---

# 72. ZAP — Role

**Classification:** `EXTERNAL TOOL ADAPTER`

Use for:

```text
DAST

web scanning

active testing
```

Likely too large to embed deeply into AgentCode source.

AgentCode owns lifecycle and result normalization.

---

# 73. Nuclei — Role

**Classification:** `WRAP`

Use engine plus version-controlled templates.

AgentCode owns:

```text
scope

rate limiting

template policy

result interpretation
```

---

# 74. Nuclei Templates — Role

**Classification:** `DATA / RULESET DEPENDENCY`

Treat template updates separately from engine updates.

Record:

```text
template commit/version
```

for reproducibility.

---

# 75. OpenSSF Scorecard — Role

**Classification:** `WRAP`

Useful for:

```text
supply-chain posture

repository-security practices
```

Not every AgentCode mission needs it.

---

# 76. Prowler — Role

**Classification:** `WRAP HEAVILY`

Primary cloud posture platform.

Use:

```text
cloud configuration

IAM

compliance

security checks
```

AgentCode adds:

```text
attack-path reasoning

finding normalization

repair planning
```

---

# 77. ScoutSuite — Role

**Classification:** `WRAP / SECONDARY CLOUD SOURCE`

Use to diversify cloud posture evidence where useful.

Avoid duplicating identical checks unnecessarily.

---

# 78. CloudSploit — Role

**Classification:** `WRAP / SECONDARY`

Similar policy:

```text
use when complementary coverage justifies execution.
```

---

# 79. Stratus Red Team — Role

**Classification:** `WRAP FOR AUTHORIZED LABS`

Use for:

```text
controlled cloud attack-technique simulation
```

Never enable automatically against production.

---

# 80. Pacu — Role

**Classification:** `LAB-ONLY / EXPLICIT AUTHORIZATION`

Use as:

```text
AWS offensive-security reference/integration
```

for controlled environments.

---

# 81. CloudGoat — Role

**Classification:** `TEST LAB`

Use for:

```text
AgentCode cloud-security benchmark

attack-chain regression

red-team acceptance tests
```

Excellent for proving AgentCode Security without risking production infrastructure.

---

# 82. Promptfoo — Role

**Classification:** `WRAP / ADAPT`

Use for:

```text
LLM evaluation

prompt injection testing

AI red team
```

---

# 83. Garak — Role

**Classification:** `WRAP`

Use as:

```text
LLM vulnerability probe suite
```

Normalize output into AgentCode Security findings.

---

# 84. PyRIT — Role

**Classification:** `ADAPT / WRAP`

Useful for:

```text
multi-turn adversarial AI testing

orchestrated red-team campaigns
```

Use only where justified by AI application scope.

---

# 85. SWE-bench — Role

**Classification:** `EVALUATION REFERENCE`

Do not integrate into runtime.

Use for:

```text
benchmarking AgentCode coding quality

comparing agent-loop strategies

regression evaluation
```

---

# 86. Continue — Role

**Classification:** `REFERENCE ONLY`

Study historical patterns for:

```text
context providers

IDE interaction

coding workflows
```

Do not build new critical dependencies on an archived/read-only project.

---

# 87. Roo Code — Role

**Classification:** `REFERENCE ONLY`

Study:

```text
modes

tool workflows

autonomous task UX
```

Archived project status means no foundational dependence.

---

# 88. Daytona — Role

**Classification:** `REFERENCE ONLY`

Study:

```text
sandbox architecture

remote development environments
```

Do not make current AgentCode V1 depend on an unmaintained public implementation.

---

# 89. License Strategy

Before AgentCode incorporates source from any repository:

```text
LICENSE CHECK
```

is mandatory.

No implementation agent may assume:

```text
open source
=
copy anything freely.
```

---

# 90. License Classification

Each source receives:

```text
PERMISSIVE

COPYLEFT

SOURCE-AVAILABLE

MIXED

UNKNOWN

INCOMPATIBLE
```

---

# 91. License Gate

Before copying/adapting code:

```text
identify repository license
        ↓
identify file-specific exception
        ↓
identify bundled assets
        ↓
identify dependency licenses
        ↓
determine AgentCode distribution compatibility
        ↓
PASS / REIMPLEMENT / REJECT
```

---

# 92. Concept vs Code

A license may restrict direct reuse while still allowing AgentCode to independently implement an architectural idea.

Therefore:

```text
conceptual study
```

and:

```text
source copying
```

must be treated differently.

---

# 93. Attribution

For reused code requiring attribution:

AgentCode should maintain:

```text
THIRD_PARTY_NOTICES.md
```

containing:

```text
project

copyright

license

source

AgentCode component using it
```

---

# 94. Third-Party Manifest

Maintain machine-readable:

```text
third_party_manifest.json
```

or equivalent.

Fields:

```text
component

upstream

commit

license

usage

bundled

modified

notice_required
```

---

# 95. Pin Source Versions

Extraction work must record upstream commit.

For every foundational reference:

```text
repository

commit SHA

extraction date
```

This makes AgentCode architecture reproducible even if upstream changes later.

---

# 96. Reference Update Policy

After AgentCode V1 implementation begins:

```text
do not continuously chase upstream changes.
```

Update reference-derived components only for:

```text
security fixes

important reliability fixes

provider compatibility

major useful improvement
```

---

# 97. No Git Submodules for Reference Library

The huge OSS reference collection should not become:

```text
AgentCode repository submodules.
```

It is a development research library.

Production dependencies should be deliberately declared individually.

---

# 98. Do Not Vendor Entire Repositories

AgentCode must not contain:

```text
vendor/codex

vendor/openhands

vendor/semgrep

vendor/playwright
```

unless an extremely specific technical reason exists.

---

# 99. Wrapper Principle

For external tooling:

```text
AgentCode Interface
       ↓
Adapter
       ↓
Pinned Upstream Tool
```

Example:

```text
SecurityScannerAdapter
       ↓
Trivy
```

---

# 100. Stable Internal APIs

Agents/models should interact with AgentCode capability APIs.

They should not rely heavily on upstream command syntax.

Example:

```text
run_dependency_security_scan()
```

rather than exposing every raw Trivy option to every Worker.

---

# 101. Why Stable Internal APIs Matter

This allows AgentCode to later replace:

```text
Trivy
```

with:

```text
another scanner
```

without changing agent behavior.

Same for:

```text
search
browser
LSP
sandbox
```

---

# 102. Implementation Language Boundaries

AgentCode should not force every reused system into one programming language.

Likely architecture contains:

```text
Rust
TypeScript
Python
external binaries
```

where appropriate.

However language boundaries must be deliberate.

---

# 103. Recommended Core Boundary

Preferred high-level split:

```text
DESKTOP UI
Tauri + React/TypeScript

AUTONOMY KERNEL
Rust or another reliable native service

PROVIDER FABRIC
custom OmniRoute fork / TypeScript-compatible layer

SPECIALIZED SECURITY TOOLS
external binaries/processes

OPTIONAL PYTHON SECURITY/AI TOOLS
managed adapters
```

Final exact language choice belongs to implementation roadmap after repository extraction.

---

# 104. Local RPC Boundary

Desktop UI should communicate with daemon through a stable local API.

Possible:

```text
Unix socket

localhost HTTP

WebSocket

Tauri IPC where appropriate
```

Do not tightly couple renderer state to Kernel internals.

---

# 105. Core vs Plugin Boundary

AgentCode Core should contain only foundational capability.

Plugins/skills may extend:

```text
cloud providers

framework skills

security tools

external integrations
```

---

# 106. What Must Be Core

Core V1:

```text
Kernel

Model Broker

OmniRoute interface

Code Intelligence

Context Engine

Tool Broker

Edit Engine

Git/worktrees

Verification

Skill loader

Hook engine

Desktop UI
```

---

# 107. What Should Remain Optional

```text
Zoekt

SCIP for unsupported languages

heavy security scanners

cloud security

Pacu

PyRIT

large browser-agent models

specialized design integrations
```

AgentCode should not require all optional systems just to fix a small local bug.

---

# 108. Feature Detection

At startup/project bootstrap AgentCode determines:

```text
available tools

available language servers

installed optional scanners

local models

container runtime

browser availability
```

Capabilities register dynamically.

---

# 109. Tool Installation Strategy

For external tools choose one of:

```text
BUNDLED

MANAGED DOWNLOAD

SYSTEM DEPENDENCY

CONTAINERIZED

REMOTE API
```

per tool.

Doc 07 requires the choice to be explicit.

---

# 110. Bundled

Good when:

```text
small

stable

permissively licensed

cross-platform binary manageable
```

---

# 111. Managed Download

Useful when:

```text
binary updates often

bundling adds major app size

tool already publishes signed releases
```

AgentCode downloads pinned version with:

```text
checksum verification.
```

---

# 112. System Dependency

Suitable when:

```text
developers commonly already install tool

distribution is complex

licensing discourages bundling
```

AgentCode can detect and guide installation.

---

# 113. Containerized

Useful for:

```text
heavy security scanners

isolation

complex runtimes
```

but Docker cannot be mandatory for normal AgentCode coding.

---

# 114. Version Registry

AgentCode should maintain:

```text
tool_registry
```

containing:

```text
tool

version

source

checksum

installation path

capabilities

license metadata
```

---

# 115. OSS Extraction Campaign

Do not ask one agent:

```text
read every repository and design AgentCode.
```

Use subsystem-specific extraction campaigns.

---

# 116. Campaign A — Provider Fabric

Inspect:

```text
OmniRoute

Codex provider abstraction where useful

OpenCode provider abstraction

Goose
```

Output:

```text
EXTRACTION_01_PROVIDER_FABRIC.md
```

---

# 117. Provider Campaign Questions

Answer:

```text
How are providers represented?

How are credentials isolated?

How are model capabilities represented?

How are provider failures normalized?

Which code can be reused?

What conflicts with Doc 01?
```

---

# 118. Campaign B — Code Intelligence

Inspect:

```text
Aider

Codex

OpenCode

Tree-sitter

ast-grep

SCIP

Zoekt

Ctags

Stack Graphs
```

Output:

```text
EXTRACTION_02_CODE_INTELLIGENCE.md
```

---

# 119. Code Intelligence Campaign Must Identify

```text
repo-map implementation

symbol extraction

ranking

LSP integration

structural search

incremental indexing

cache strategy

large-repo search

data structures

performance bottlenecks
```

---

# 120. Campaign C — Context & Memory

Inspect:

```text
Letta Code

Letta

Graphiti

Gemini CLI

Cline

RTK

Caveman

Aider
```

Output:

```text
EXTRACTION_03_CONTEXT_MEMORY.md
```

---

# 121. Context Campaign Questions

```text
How is memory stored?

How is context compacted?

How is old information retrieved?

What remains always in context?

How is raw history preserved?

How are summaries refreshed?

What mechanisms reduce tokens?
```

---

# 122. Campaign D — Agent Runtime

Inspect:

```text
Codex

mini-SWE-agent

OpenHands

Software Agent SDK

Gemini CLI

Munder Difflin

LangGraph

Microsoft Agent Framework
```

Output:

```text
EXTRACTION_04_AGENT_RUNTIME.md
```

---

# 123. Runtime Campaign Questions

```text
agent loop

task state

session state

worker lifecycle

checkpointing

failure recovery

agent messaging

tool interaction

durability
```

---

# 124. Campaign E — Tools & Editing

Inspect:

```text
Codex

Aider

Gemini CLI

Cline

OpenHands

OpenCode
```

Output:

```text
EXTRACTION_05_TOOLS_EDITING.md
```

---

# 125. Editing Campaign Must Compare

```text
patch format

search/replace

whole-file editing

structured edit

checkpoint behavior

concurrency protection

rollback

multi-file edits
```

---

# 126. Campaign F — Git & Worktrees

Inspect:

```text
Codex

Cline

Superpowers

Munder Difflin

Aider
```

Output:

```text
EXTRACTION_06_GIT_WORKTREES.md
```

---

# 127. Campaign G — Sandbox & Permissions

Inspect:

```text
Codex

OpenHands

SWE-ReX

Gemini CLI

OpenCode

Cline
```

Output:

```text
EXTRACTION_07_SANDBOX_PERMISSIONS.md
```

---

# 128. Campaign H — Skills & Hooks

Inspect:

```text
Gemini CLI

Superpowers

Compound Engineering

Trail of Bits Skills

Letta Skills

OpenHands

Claude-compatible patterns where documented
```

Output:

```text
EXTRACTION_08_SKILLS_HOOKS.md
```

---

# 129. Campaign I — Browser & QA

Inspect:

```text
Playwright

Browser Use

Browser Harness

Cline
```

Output:

```text
EXTRACTION_09_BROWSER_QA.md
```

---

# 130. Campaign J — Design Studio

Inspect:

```text
Onlook

Dyad

Bolt.diy
```

Output:

```text
EXTRACTION_10_DESIGN_STUDIO.md
```

---

# 131. Design Campaign Questions

```text
How is preview generated?

How is code mapped to visual elements?

How are edits reflected live?

How are project files managed?

How does prompt-to-app flow work?

What can be reused without inheriting product UI?
```

---

# 132. Campaign K — Security Agent

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

Output:

```text
EXTRACTION_11_APPSEC.md
```

---

# 133. Campaign L — Cloud Security

Inspect:

```text
Prowler

ScoutSuite

CloudSploit

Stratus Red Team

Pacu

CloudGoat
```

Output:

```text
EXTRACTION_12_CLOUD_SECURITY.md
```

---

# 134. Campaign M — AI Security

Inspect:

```text
Promptfoo

Garak

PyRIT
```

Output:

```text
EXTRACTION_13_AI_SECURITY.md
```

---

# 135. Campaign N — Product UX

Inspect:

```text
Codex

OpenCode

Cline

Goose

Dyad

Onlook
```

Output:

```text
EXTRACTION_14_PRODUCT_UX.md
```

Focus on:

```text
minimal task workflow

background execution

diff review

progress presentation

approval UX

preview UX
```

not visual copying.

---

# 136. Extraction Agent Rules

Every campaign agent must:

```text
READ actual code

READ license

READ architecture docs

IDENTIFY exact files

TRACE execution paths

TEST key behavior when practical
```

It must not rely only on READMEs.

---

# 137. No Superficial Extraction

A result such as:

```text
"Codex uses sandboxing, so we should use sandboxing."
```

is insufficient.

Required:

```text
where sandbox policy lives

how command is intercepted

how permissions are represented

how process gets launched

what assumptions exist

what can be reused
```

---

# 138. Extraction Depth Levels

Every candidate mechanism can be investigated at:

```text
L1 — Documentation

L2 — Source path

L3 — Control-flow tracing

L4 — Local execution/test

L5 — Prototype extraction
```

Foundational mechanisms should reach:

```text
L3–L5.
```

---

# 139. Prototype Before Adoption

For risky architectural components, build focused spikes.

Examples:

```text
Tree-sitter index prototype

LSP manager prototype

worktree Worker prototype

transactional edit prototype

daemon restart prototype

OmniRoute top-K prototype

RTK compression benchmark
```

Do not integrate large systems without proving the critical mechanism.

---

# 140. Extraction Artifact Structure

Recommended:

```text
docs/extraction/
│
├── 01_provider_fabric.md
├── 02_code_intelligence.md
├── 03_context_memory.md
├── 04_agent_runtime.md
├── 05_tools_editing.md
├── ...
└── decisions/
```

---

# 141. Extraction Matrix

Each campaign should finish with:

| Capability | Source | Classification | AgentCode Destination | Reuse Form | License Gate | Priority |
|---|---|---|---|---|---|---|

This becomes implementation input.

---

# 142. Code Copy Threshold

Direct code reuse should occur only when it produces clear benefit.

Ask:

```text
Would understanding and maintaining this copied code
take less effort than implementing the normalized AgentCode version?
```

If no:

```text
ADAPT CONCEPT
```

rather than copy.

---

# 143. Abstraction Extraction

Often the most valuable thing to reuse is:

```text
interface design
```

rather than implementation.

Example:

OpenHands workspace abstractions may inspire:

```text
AgentCode Workspace API
```

without copying its runtime.

---

# 144. Test Extraction

Tests can be extremely valuable references.

Study upstream tests for:

```text
edge cases

failure modes

expected behavior

platform differences
```

AgentCode can independently write equivalent behavior tests.

---

# 145. OSS Regression Tests

When AgentCode adopts an upstream mechanism, write AgentCode-native tests.

Do not depend solely on upstream test suites.

Example:

```text
AgentCode worktree isolation test

AgentCode patch rollback test

AgentCode model failover test
```

---

# 146. Benchmark Extraction

Some competing mechanisms require empirical comparison.

Examples:

```text
RTK vs Caveman

ripgrep vs Zoekt threshold

LSP vs SCIP coverage

editing formats by model

local vs cloud routing

browser-use vs deterministic Playwright
```

---

# 147. Benchmark Repository Set

Create representative AgentCode benchmark repositories:

```text
small TypeScript

Next.js full-stack

Python

Rust

Go

polyglot

large monorepo

security-vulnerable lab
```

---

# 148. Performance Benchmark Metrics

Measure:

```text
latency

RAM

CPU

disk

tokens

success

correctness

retries
```

Do not select components solely from popularity.

---

# 149. 8 GB Mac Constraint

Every foundational subsystem must be tested on the real hardware target.

AgentCode must avoid requiring simultaneously:

```text
Zoekt

several LSP servers

large local model

browser

multiple heavy workers
```

unless Resource Governor can manage them.

---

# 150. OSS Process Isolation

External tools should generally execute as managed subprocesses.

Benefits:

```text
crash isolation

memory cleanup

version isolation

easy upgrade

clear security boundary
```

---

# 151. Process Adapter Pattern

Conceptually:

```text
ToolAdapter
    start()
    invoke()
    parse()
    normalize()
    health()
    stop()
```

---

# 152. JSON Preferred

Where upstream tools support:

```text
JSON

SARIF

structured output
```

prefer it over scraping terminal text.

---

# 153. CLI Scraping

Use text parsing only when structured output unavailable.

Parser must be version-aware.

---

# 154. Upstream Tool Health

External tool adapters should expose:

```text
AVAILABLE

MISSING

UNHEALTHY

INCOMPATIBLE_VERSION

DISABLED
```

AgentCode degrades gracefully.

---

# 155. Auto-Installation

AgentCode may offer:

```text
Install required tool
```

for missing optional tooling.

Do not silently download large external security systems without informing user.

---

# 156. Security Tool Updates

Security intelligence becomes stale quickly.

For:

```text
Nuclei templates

vulnerability databases

scanner databases
```

AgentCode should support controlled updates independently from application releases.

---

# 157. Database Update Separation

Example:

```text
Trivy engine version
```

and:

```text
Trivy vulnerability database version
```

must be tracked separately.

---

# 158. Provider Updates

OmniRoute provider metadata may require much faster updates than AgentCode Core.

Provider adapters should therefore remain modular.

---

# 159. Language Adapter Updates

Tree-sitter grammar and LSP adapters should be modular.

Adding:

```text
Java

C#

Kotlin
```

must not require redesigning Code Intelligence Core.

---

# 160. Design Studio Framework Adapters

Initial strong support may focus on:

```text
React

Next.js

Tailwind
```

because Onlook/Dyad/Bolt concepts are strongest there.

Later:

```text
Vue

Svelte

native UI
```

can use adapters.

---

# 161. Security Adapter Registry

AgentCode Security should expose:

```text
SecurityAdapterRegistry
```

where tools register:

```text
target types

scan types

risk

requirements

result parser
```

---

# 162. Scanner Selection Engine

Do not run all scanners for every mission.

Example:

```text
Rust CLI
```

does not need:

```text
ZAP
```

unless web interface exists.

---

# 163. Scanner Relevance

Select based on:

```text
language

framework

deployment

changed files

mission

risk
```

---

# 164. Avoid Duplicate Scanner Work

If:

```text
Trivy
```

and:

```text
OSV
```

produce nearly identical dependency data, AgentCode may use one as primary and the other for:

```text
high-risk cross-check
```

rather than run both constantly.

---

# 165. UI Source Reuse

AgentCode should not copy the visual identity of:

```text
Codex

Onlook

Dyad
```

Study interaction mechanics.

Create original AgentCode design consistent with Doc 06.

---

# 166. No Generic Framework Frankenstein

AgentCode should not contain:

```text
LangGraph mission graph

plus Microsoft workflow graph

plus OpenHands event loop

plus Munder agent graph
```

all controlling one mission.

Exactly one authority:

```text
AgentCode Kernel.
```

---

# 167. No Duplicate Memory Systems

Likewise do not run:

```text
Letta memory

Graphiti graph memory

custom JSON memory

SQLite memory
```

simultaneously as competing truth.

V1 authority:

```text
SQLite structured state
+
AgentCode readable context files.
```

Other projects provide patterns.

---

# 168. No Duplicate Code Indexes Without Reason

Avoid maintaining:

```text
Tree-sitter graph

Ctags graph

SCIP graph

LSP graph

Stack Graph graph
```

as separate truths.

Normalize them into:

```text
AgentCode Repository Graph.
```

Each source contributes evidence.

---

# 169. Source Confidence

Relationship edges may record source:

```text
LSP

SCIP

Tree-sitter

Ctags

inference
```

with confidence.

---

# 170. No Duplicate Browser Runtimes

Preferred:

```text
Playwright
```

as deterministic core.

Browser Use provides agentic layer.

Do not embed three browsers unless technically justified.

---

# 171. No Duplicate Security Databases

AgentCode should normalize findings.

Do not create independent finding stores for every scanner.

One:

```text
AgentCode Security Finding Database.
```

---

# 172. No Duplicate Skill Engines

AgentCode should normalize:

```text
Superpowers

Letta Skills

Trail of Bits Skills

Gemini skills
```

into one AgentCode Skill Engine.

---

# 173. Skill Importer Architecture

Conceptually:

```text
Source Skill
     ↓
Importer
     ↓
Normalized Metadata
     ↓
AgentCode Skill Package
```

---

# 174. Skill Conversion

If licensing permits copying/adaptation:

```text
convert.
```

If not:

```text
extract workflow concept
→ independently author AgentCode skill.
```

---

# 175. Tool Security Review

Any external executable integrated into AgentCode should undergo:

```text
source/reputation review

license review

binary provenance review

checksum/signature review

sandbox review
```

---

# 176. External Binary Supply Chain

Managed downloads must use:

```text
HTTPS

pinned release

checksum

known publisher
```

where available.

---

# 177. No `curl | sh`

AgentCode should not install foundational dependencies using unverified:

```text
curl ... | sh
```

flows.

---

# 178. Plugin Isolation

Future third-party plugins should not execute with full Kernel authority.

They receive:

```text
capabilities
```

through Tool Broker.

---

# 179. Core Plugin API

Potential extension points:

```text
ProviderAdapter

LanguageAdapter

ToolAdapter

SecurityAdapter

SkillProvider

ContextSource

HookProvider
```

---

# 180. API Stability

Internal extension APIs should be versioned.

Example:

```text
agentcode.tool.v1
```

to prevent uncontrolled plugin coupling.

---

# 181. Extraction Dependency Graph

Extraction order matters.

Recommended:

```text
Provider Fabric
      ↓
Kernel Runtime
      ↓
Tool/Sandbox
      ↓
Code Intelligence
      ↓
Context/Memory
      ↓
Editing/Git
      ↓
Verification
      ↓
Security
      ↓
Design Studio
      ↓
Desktop UX
```

Some campaigns may run in parallel.

---

# 182. Why Kernel Before Full UI

AgentCode's defining capability is:

```text
autonomous durable execution.
```

A beautiful UI over a fragile agent loop provides little value.

Therefore implementation should prove daemon/Kernel first.

---

# 183. Why Code Intelligence Early

Likewise a durable agent with superficial repository understanding remains weak.

Code Intelligence must mature early enough to guide real Worker implementation.

---

# 184. Why Security Later but Architecturally Prepared

Security depends on:

```text
Tool Broker

Code Intelligence

Context

Kernel tasks

Evidence
```

so the deepest Security mode should come after these foundations.

However adapters/interfaces should be designed early.

---

# 185. Why Design Studio After Browser Engine

Design Studio requires:

```text
editing

browser

screenshots

visual context

verification
```

so building it before those foundations would duplicate work.

---

# 186. Implementation Stage 0 — Extraction Infrastructure

Before AgentCode implementation begins, create:

```text
reference catalog

license catalog

extraction templates

benchmark harness

architecture decision log
```

---

# 187. Stage 0 Deliverables

```text
AGENTCODE_REFERENCE_CATALOG.md

OSS_LICENSE_MATRIX.md

EXTRACTION_TEMPLATE.md

THIRD_PARTY_NOTICES.md

BENCHMARK_PLAN.md
```

---


# HARDENING REVISION 2 — OSS EXTRACTION CONTROL SPECIFICATION + DOSSIER FRAMEWORK

This hardening section is normative.

The base document already established the correct philosophy:

> **Borrow primitives. Own orchestration.**

It also correctly rejected the idea of combining entire agent frameworks into a single “Frankenstein” runtime. The purpose of this hardening revision is to solve the remaining weakness: the previous document described **how extraction should happen**, but it did not define enough canonical records, source-pinning rules, campaign exit criteria, adoption gates, implementation packets, and repository-level status semantics to guarantee that the extraction program itself is reproducible.

This revision therefore distinguishes two separate but linked outputs:

```text
DOC 07
│
├── EXTRACTION CONTROL SPECIFICATION
│   ├── records
│   ├── lifecycle
│   ├── source pinning
│   ├── license gates
│   ├── campaign procedure
│   ├── benchmark/prototype rules
│   ├── adoption review
│   └── implementation handoff
│
└── EXTRACTION DOSSIER / CATALOG
    ├── repository catalog
    ├── donor decision matrix
    ├── campaign reports
    ├── source pointers
    ├── verified observations
    ├── license reviews
    ├── adoption decisions
    └── benchmark evidence
```

The control specification is complete when this document is complete.

The source-level dossier is complete only when the companion extraction reports contain verified repository SHAs, exact source paths, traced mechanisms, tests, license results, and reviewed adoption decisions.

No sentence in this document may be interpreted as proof that a source-level extraction was performed unless it is marked `EXTRACTION_VERIFIED` and contains the required source evidence.

---

## H1. Normative Decision and Evidence Classes

AgentCode must distinguish architecture decisions from extraction evidence.

| Class | Meaning |
|---|---|
| `LOCKED_ARCHITECTURE` | A decision inherited from Docs 01–06. OSS research may not silently change it. |
| `CONSTRAINED_IMPLEMENTATION_DECISION` | The architecture constrains the outcome, but Phase 0/1 may select one implementation after evidence/ADR. |
| `EXTRACTION_VERIFIED` | Source-level mechanism was located at a pinned SHA and reviewed at the required depth. |
| `EXTRACTION_PENDING` | Mechanism is a known research target but source-level proof has not yet been completed. |
| `SOURCE_STALE` | Previously verified extraction refers to an older upstream SHA and must be revalidated before relying on changed files. |
| `LICENSE_VERIFIED` | License/redistribution decision has evidence sufficient for the intended reuse form. |
| `LICENSE_PENDING` | License has not been reviewed sufficiently for direct reuse/distribution. |
| `LICENSE_BLOCKED` | Intended reuse form is not acceptable under current distribution plan. |
| `BENCHMARK_PENDING` | Competing mechanisms need empirical comparison before adoption. |
| `DYNAMIC_RUNTIME_DATA` | Tool availability/version/capability that is discovered at runtime, not locked in architecture. |
| `OPTIONAL_V1` | Useful capability that may be absent without blocking normal V1 coding. |
| `POST_V1` | Deliberately outside initial V1 release scope. |
| `PRIOR_OBSERVATION_REVERIFY_REQUIRED` | Earlier project research reported a fact, but this hardening environment could not independently re-open the donor clone. |

### Mandatory rule

A donor recommendation such as:

```text
Aider → repo-map ideas
```

is not equivalent to:

```text
Aider repo-map implementation verified at SHA X,
files A/B/C,
control flow traced,
license reviewed,
adoption approved.
```

The first is a research direction.

The second is an implementation-ready extraction.

---

## H2. Architectural Authority

The complete authority chain is:

```text
USER / APPROVED PROJECT DECISIONS
        ↓
DOCS 01–06 ARCHITECTURE
        ↓
APPROVED ADRs
        ↓
DOC 07 EXTRACTION / ADOPTION DECISIONS
        ↓
PINNED DONOR SOURCE
        ↓
UPSTREAM DOCUMENTATION / README / ISSUES
```

A donor's architecture is evidence, not authority.

Examples:

```text
LangGraph has its own graph runtime
≠
AgentCode should use LangGraph as mission truth.

Letta has memory abstractions
≠
Letta becomes AgentCode's project-truth database.

OpenHands has an event loop
≠
OpenHands owns AgentCode task state.

Onlook has visual editing
≠
AgentCode becomes an Onlook fork.
```

When an extraction uncovers a genuinely superior mechanism that conflicts with a locked AgentCode decision, the extraction record must say:

```text
ARCHITECTURE_CONFLICT
```

and create an ADR proposal.

The extraction agent may not resolve that conflict by implementation.

---

## H3. Canonical Repository Catalog Name

The single canonical human-readable repository catalog is:

```text
docs/reference/AGENTCODE_REFERENCE_CATALOG.md
```

Machine-readable representation may also exist:

```text
docs/reference/agentcode_reference_catalog.json
```

or a development database.

Older references to:

```text
OSS_REFERENCE_CATALOG.md
```

are legacy aliases only.

There must not be two independently maintained catalogs.

Canonical source of repository metadata:

```text
machine-readable catalog
        ↓
generated AGENTCODE_REFERENCE_CATALOG.md
```

where practical.

---

## H4. Local Reference Library Policy

Canonical development root:

```text
/Volumes/T7 Shield/GitHub-Repos-dependency
```

The root remains **physically flat**.

Reasons:

1. many repositories belong to several categories;
2. stable paths are useful in extraction reports;
3. physical reorganization creates needless path churn;
4. logical grouping belongs in metadata.

### Known naming collision

The prior project discussion identified a collision around:

```text
skills
```

because a `skills` directory already existed when attempting to clone Letta Skills.

Required verification procedure:

```bash
ROOT='/Volumes/T7 Shield/GitHub-Repos-dependency'

git -C "$ROOT/skills" remote get-url origin
```

If the existing repository is confirmed to be Trail of Bits skills:

```bash
mv "$ROOT/skills" "$ROOT/trailofbits-skills"
git clone --depth 1 https://github.com/letta-ai/skills.git "$ROOT/letta-skills"
```

This is a **recommended migration**, not a claim that it has already been executed.

The catalog should preserve aliases:

```text
legacy path: skills
canonical logical name: trailofbits-skills
```

until migration is confirmed.

---

## H5. Reference Repository Record

Every donor repository must have a canonical `ReferenceRepository` record.

```ts
type ReferenceRepository = {
  repository_id: string

  canonical_name: string
  display_name: string

  local_directory: string
  path_aliases: string[]

  upstream: {
    url: string
    owner: string
    repository: string
  }

  git: {
    origin_url?: string
    upstream_url?: string
    head_sha?: string
    branch?: string
    dirty?: boolean
    shallow?: boolean
    verified_at?: string
  }

  lifecycle: {
    archived:
      | "YES"
      | "NO"
      | "UNKNOWN"
    maintenance:
      | "ACTIVE"
      | "MAINTENANCE"
      | "UNMAINTAINED"
      | "ARCHIVED"
      | "UNKNOWN"
    lifecycle_evidence_ref?: string
  }

  classification: {
    priority: "P0" | "P1" | "P2" | "P3"
    categories: string[]
    donor_role: string[]
    production_dependency:
      | "EXPECTED"
      | "POSSIBLE"
      | "NO"
      | "UNDECIDED"
  }

  licensing: {
    status:
      | "LICENSE_VERIFIED"
      | "LICENSE_PENDING"
      | "LICENSE_BLOCKED"
    classification?:
      | "PERMISSIVE"
      | "COPYLEFT"
      | "SOURCE_AVAILABLE"
      | "MIXED"
      | "UNKNOWN"
      | "INCOMPATIBLE"
    review_ref?: string
  }

  extraction: {
    campaigns: string[]
    status:
      | "NOT_STARTED"
      | "IN_PROGRESS"
      | "PARTIAL"
      | "IMPLEMENTATION_READY"
      | "REFERENCE_ONLY"
      | "REJECTED"
    extraction_refs: string[]
  }

  pin_policy:
    | "PIN_FOR_EXTRACTION"
    | "PIN_RUNTIME_DEPENDENCY"
    | "FOLLOW_TOOL_RELEASE_CHANNEL"
    | "REFERENCE_ONLY"

  notes: string[]
}
```

### Identity rule

`repository_id` is stable even if the local directory is renamed.

Example:

```text
repository_id:
repo:trailofbits-skills

local_directory:
trailofbits-skills
```

### Freshness rule

A repository record without a recently verified `head_sha` may still be used for:

```text
catalog discovery
historical comparison
planning
```

but cannot support a new `EXTRACTION_VERIFIED` source-level claim.

---

## H6. Repository Verification Procedure

At the beginning of a campaign, gather donor metadata without mutating the repository.

Safe baseline commands:

```bash
REPO='/Volumes/T7 Shield/GitHub-Repos-dependency/aider'

git -C "$REPO" remote -v
git -C "$REPO" rev-parse HEAD
git -C "$REPO" branch --show-current
git -C "$REPO" status --short --branch
git -C "$REPO" rev-parse --is-shallow-repository
find "$REPO" -maxdepth 2 \
  \( -iname 'LICENSE*' -o -iname 'COPYING*' -o -iname 'NOTICE*' \) \
  -print
```

Then inspect top-level structure:

```bash
find "$REPO" -maxdepth 2 -type f | head -300
```

Targeted search:

```bash
rg -n "relevant_symbol|class_name|command_name" "$REPO"
```

### Do not automatically run

```text
npm install
pnpm install
pip install
cargo build
make
curl | sh
setup scripts
postinstall hooks
```

simply because a README says to do so.

Donor code is untrusted until inspected.

---

## H7. Canonical Extraction Record

The prior extraction example with `...` placeholders is replaced by the following normative model.

```ts
type ExtractionRecord = {
  extraction_id: string
  campaign_id: string

  subsystem: string
  capability: string

  source: {
    repository_id: string
    commit_sha: string
    branch?: string

    files: Array<{
      path: string
      symbol?: string
      line_or_range_hint?: string
      role: string
    }>

    tests: Array<{
      path: string
      symbol?: string
      behavior_proven: string
    }>
  }

  observation: {
    mechanism_summary: string
    entrypoint: string
    control_flow: string[]
    important_data_structures: string[]
    invariants: string[]
    failure_behavior: string[]
    persistence_behavior: string[]
    concurrency_behavior: string[]
    platform_assumptions: string[]
    external_dependencies: string[]
    performance_characteristics: string[]
  }

  extraction_depth:
    | "L0_CATALOG"
    | "L1_DOCS"
    | "L2_SOURCE_LOCATED"
    | "L3_CONTROL_FLOW"
    | "L4_EXECUTED_TESTED"
    | "L5_AGENTCODE_PROTOTYPE"

  classification:
    | "TAKE"
    | "ADAPT"
    | "WRAP"
    | "STUDY"
    | "IGNORE"
    | "REJECT"

  reuse_form:
    | "DEPENDENCY"
    | "EXTERNAL_TOOL"
    | "SOURCE_REUSE"
    | "INDEPENDENT_REIMPLEMENTATION"
    | "REFERENCE_ONLY"

  destination: {
    agentcode_subsystem: string
    module_or_interface: string
  }

  adaptation: {
    retained_behavior: string[]
    changed_behavior: string[]
    rejected_upstream_behavior: string[]
    normalization_required: string[]
  }

  licensing: {
    review_ref: string
    status:
      | "LICENSE_VERIFIED"
      | "LICENSE_PENDING"
      | "LICENSE_BLOCKED"
  }

  validation: {
    prototype_refs: string[]
    benchmark_refs: string[]
    agentcode_test_requirements: string[]
  }

  risks: string[]

  architecture_refs: string[]
  roadmap_refs: string[]
  acceptance_gate_refs: string[]

  reviewer?: string
  confidence: "HIGH" | "MEDIUM" | "LOW"

  status:
    | "DISCOVERED"
    | "SOURCE_LOCATED"
    | "CONTROL_FLOW_TRACED"
    | "TESTED"
    | "PROTOTYPED"
    | "LICENSE_CLEARED"
    | "DECISION_REVIEWED"
    | "IMPLEMENTATION_READY"
    | "BLOCKED"
    | "REJECTED"
    | "SOURCE_STALE"

  created_at: string
  updated_at: string
}
```

### Required behavior

A record cannot become:

```text
IMPLEMENTATION_READY
```

merely because:

```text
source files were located.
```

The adoption decision must also be reviewed and the required license/prototype/benchmark gates must pass.

---

## H8. Extraction Status Promotion Rules

Canonical promotion:

```text
DISCOVERED
   ↓
SOURCE_LOCATED
   ↓
CONTROL_FLOW_TRACED
   ↓
TESTED            (where practical/required)
   ↓
PROTOTYPED        (where architecture/risk requires)
   ↓
LICENSE_CLEARED
   ↓
DECISION_REVIEWED
   ↓
IMPLEMENTATION_READY
```

Not every record requires every intermediate stage.

Examples:

### ripgrep invocation

Likely:

```text
SOURCE_LOCATED
→ CONTROL_FLOW_TRACED at adapter boundary
→ TESTED
→ LICENSE_CLEARED
→ DECISION_REVIEWED
→ IMPLEMENTATION_READY
```

No elaborate prototype of ripgrep itself is necessary.

### transactional edit semantics inspired by several agents

Likely:

```text
SOURCE_LOCATED
→ CONTROL_FLOW_TRACED
→ TESTED
→ PROTOTYPED
→ LICENSE_CLEARED / independent reimplementation decision
→ DECISION_REVIEWED
→ IMPLEMENTATION_READY
```

### archived project studied for concepts

May stop:

```text
CONTROL_FLOW_TRACED
→ REFERENCE_ONLY
```

without production dependency.

---

## H9. Source Staleness

Every source-level claim is pinned to a SHA.

Example:

```text
Source:
repo:aider

Commit:
abc123

Path:
aider/repomap.py
```

If the local donor later changes to:

```text
def456
```

the extraction record remains historically valid for `abc123`.

It does **not** automatically describe `def456`.

If AgentCode decides to update the implementation based on newer upstream behavior:

```text
new SHA
→ source impact check
→ re-trace changed relevant files
→ rerun relevant donor/AgentCode tests
→ update extraction record
→ update adoption decision if behavior changed
```

State:

```text
SOURCE_STALE
```

is used when an implementation decision requires current upstream behavior but the source evidence is pinned to an older changed mechanism.

---

## H10. Adoption Decision Record

Observation and adoption must remain separate.

An upstream mechanism may be excellent but still wrong for AgentCode.

```ts
type AdoptionDecision = {
  decision_id: string
  capability: string

  candidates: Array<{
    extraction_ref: string
    summary: string
  }>

  selected?: {
    extraction_ref: string
    classification:
      | "TAKE"
      | "ADAPT"
      | "WRAP"
      | "STUDY"
      | "IGNORE"
      | "REJECT"
    reuse_form: string
  }

  rationale: {
    architecture_fit: string
    correctness: string
    maintainability: string
    performance: string
    resource_cost: string
    security: string
    licensing: string
    platform_support: string
    replacement_cost: string
  }

  rejected_alternatives: Array<{
    extraction_ref: string
    reason: string
  }>

  agentcode_destination: string

  implementation_contract: string[]
  prohibited_inheritance: string[]

  fallback?: string

  benchmark_refs: string[]
  prototype_refs: string[]
  license_review_refs: string[]

  adr_ref?: string

  status:
    | "PROPOSED"
    | "REVIEWED"
    | "APPROVED"
    | "BLOCKED"
    | "SUPERSEDED"

  reviewed_at?: string
}
```

### Example distinction

```text
Observation:
LangGraph supports resumable graph execution.

Adoption decision:
STUDY its durability patterns.
REJECT it as AgentCode mission authority.
```

That separation prevents donor architecture from creeping into the product unnoticed.

---

## H11. Reuse Forms

`TAKE`, `ADAPT`, and `WRAP` describe **architectural treatment**.

Reuse form describes **how code/tooling actually enters AgentCode**.

### `DEPENDENCY`

AgentCode imports a library/package.

Examples may include:

```text
Tree-sitter library
Playwright package
```

subject to license/platform review.

### `EXTERNAL_TOOL`

AgentCode invokes an independently installed/bundled/managed executable.

Examples:

```text
ripgrep
Gitleaks
Trivy
Nuclei
Prowler
```

### `SOURCE_REUSE`

AgentCode copies/adapts upstream source.

Requires the strongest provenance/license discipline.

### `INDEPENDENT_REIMPLEMENTATION`

AgentCode studies behavior/interface ideas but writes original AgentCode-native implementation.

This is often preferable for:

```text
agent loops
memory orchestration
approval ideas
checkpoint coordination
repository-map ranking extensions
```

### `REFERENCE_ONLY`

No production dependency and no substantial source reuse.

---

## H12. Classification-to-Reuse Matrix

| Classification | Common reuse forms | Meaning |
|---|---|---|
| `TAKE` | DEPENDENCY, EXTERNAL_TOOL | Use mature primitive largely as-is behind AgentCode interface. |
| `ADAPT` | DEPENDENCY, SOURCE_REUSE, INDEPENDENT_REIMPLEMENTATION | Keep useful behavior while replacing incompatible lifecycle/authority. |
| `WRAP` | EXTERNAL_TOOL, DEPENDENCY | Upstream engine remains independent; AgentCode owns adapter/policy/state. |
| `STUDY` | REFERENCE_ONLY, INDEPENDENT_REIMPLEMENTATION | Use ideas, not runtime dependency. |
| `IGNORE` | REFERENCE_ONLY | No current value. |
| `REJECT` | REFERENCE_ONLY | Explicitly prohibit architecture/behavior. |

`TAKE` never means:

```text
copy an arbitrary source directory into AgentCode.
```

---

## H13. License Review Record

License review must occur before source copying or bundling/distribution decisions.

```ts
type LicenseReviewRecord = {
  license_review_id: string

  repository_id: string
  commit_sha: string

  detected_files: Array<{
    path: string
    type: "LICENSE" | "NOTICE" | "COPYING" | "FILE_HEADER" | "ASSET_TERMS" | "OTHER"
  }>

  repository_classification:
    | "PERMISSIVE"
    | "COPYLEFT"
    | "SOURCE_AVAILABLE"
    | "MIXED"
    | "UNKNOWN"
    | "INCOMPATIBLE"

  spdx_identifiers: string[]

  file_specific_exceptions: string[]
  separately_licensed_directories: string[]
  asset_restrictions: string[]
  trademark_notes: string[]

  intended_reuse:
    | "DEPENDENCY"
    | "EXTERNAL_TOOL"
    | "SOURCE_REUSE"
    | "REFERENCE_ONLY"

  distribution: {
    bundled: boolean
    modified: boolean
    redistributed: boolean
    source_offer_required?: boolean
    notice_required?: boolean
    license_text_required?: boolean
  }

  decision:
    | "PASS"
    | "PASS_WITH_OBLIGATIONS"
    | "REIMPLEMENT"
    | "DO_NOT_DISTRIBUTE"
    | "BLOCKED_PENDING_REVIEW"

  obligations: string[]
  evidence_refs: string[]

  reviewer: string
  reviewed_at: string
}
```

### Important boundary

This record is engineering license hygiene.

It is **not legal advice**.

If public distribution implications remain uncertain:

```text
BLOCKED_PENDING_REVIEW
```

until a qualified human/legal review resolves them.

---

## H14. File-Level and Asset-Level Licensing

Repository-level license is insufficient when a project contains:

```text
commercial/pro directories
separately licensed UI assets
fonts
icons
examples copied from other projects
generated code
embedded binaries
WebContainer/API terms
model weights
datasets
```

Therefore direct reuse must identify the actual files or distributed artifact.

Known project cautions from prior research include:

```text
Dyad
→ inspect separately licensed/pro directories.

Bolt.diy
→ repository source terms and WebContainer/API terms are separate questions.

CodeQL
→ query/library repository and distributed CLI/engine are separate licensing questions.

Munder Difflin
→ source code and artwork/assets may have separate restrictions.

Ctags
→ license implications require explicit review before embedding/linking decisions.
```

These are **review requirements**, not final license conclusions unless the campaign records proof.

---

## H15. Third-Party Provenance

Any material third-party source reuse must remain traceable.

Machine-readable manifest:

```ts
type ThirdPartyComponent = {
  component_id: string
  upstream_url: string
  upstream_commit_or_version: string
  source_paths?: string[]
  agentcode_paths?: string[]

  usage:
    | "DEPENDENCY"
    | "BUNDLED_BINARY"
    | "MANAGED_TOOL"
    | "SOURCE_REUSE"
    | "OPTIONAL_TOOL"

  modified: boolean
  bundled: boolean

  license_review_ref: string
  notice_required: boolean
  notice_entry_ref?: string

  extraction_refs: string[]
  adoption_decision_ref: string
}
```

Human-readable:

```text
THIRD_PARTY_NOTICES.md
```

Generated release SBOM should include package/binary dependencies separately.

---

## H16. Fork Policy

A fork is justified only when all are true:

1. upstream implementation already provides substantial value;
2. AgentCode requires sustained changes that cannot cleanly live in an adapter;
3. architecture remains compatible;
4. license allows the intended fork/distribution;
5. patch burden is acceptable;
6. fork has explicit owner;
7. compatibility tests exist;
8. upstream security updates can still be consumed.

Known planned major fork:

```text
OmniRoute
```

Most other dependencies should remain upstream-managed.

### Fork record

```ts
type ForkRecord = {
  fork_id: string
  repository_id: string
  base_sha: string

  origin_url: string
  upstream_url: string

  patches: Array<{
    patch_id: string
    purpose: string
    affected_paths: string[]
    architecture_ref: string
  }>

  divergence_budget: {
    max_long_lived_patch_count?: number
    review_trigger?: string
  }

  sync_policy: string[]
  compatibility_tests: string[]
  security_update_policy: string[]

  owner: string
}
```

---

## H17. OmniRoute Fork Boundary

AgentCode intentionally modifies OmniRoute for provider-fabric mechanics required by Doc 01.

### Keep or adapt within OmniRoute fork

Examples:

```text
provider adapters
connection records
provider catalog/discovery
quota-source integration
provider health
cooldowns/circuit breaker plumbing
response normalization
latency/cost observation
provider fallback primitives
```

### AgentCode-specific additions may include

```text
connection-aware quota domains
richer health/capability metadata
candidate enumeration support
route audit data
local-provider registration hooks
```

### Keep outside OmniRoute in Model Broker

```text
task complexity/risk semantics
Planner/Worker/Verifier role semantics
critical-task model-family diversity policy
project empirical performance ranking
risk-based fanout
mission-level budget policy
completion/verification authority
```

Doc 01 is authoritative if any boundary is ambiguous.

---

## H18. Dependency Admission Record

No foundational package/tool enters AgentCode because an implementation agent “likes it.”

```ts
type DependencyAdmission = {
  admission_id: string
  component: string

  purpose: string
  architecture_refs: string[]

  alternatives_considered: string[]
  why_current_stack_is_insufficient: string

  source: {
    upstream_url: string
    version_or_sha: string
    release_channel?: string
  }

  maintenance: {
    lifecycle_status: string
    recent_activity_evidence?: string
    replacement_risk: string
  }

  licensing: {
    license_review_ref: string
    distribution_ok: boolean
  }

  supply_chain: {
    publisher: string
    checksum_or_signature_strategy: string
    install_scripts_reviewed: boolean
  }

  platform: {
    darwin_arm64: string
    darwin_x64: string
    windows_x64: string
    linux_x64?: string
  }

  resources: {
    install_size?: string
    expected_ram?: string
    cpu_pattern?: string
    startup_cost?: string
  }

  trust: {
    network_access: string
    filesystem_access: string
    secret_access: string
    sandbox_profile: string
  }

  update_policy: string
  rollback_policy: string
  removal_plan: string

  decision:
    | "APPROVED"
    | "APPROVED_OPTIONAL"
    | "REJECTED"
    | "PENDING"

  reviewer: string
}
```

---

## H19. External Binary Provenance

Managed binaries require:

```text
tool identity
publisher
release URL
version
architecture
SHA-256
signature verification when available
download time
install path
license
capabilities
engine version
data/ruleset version
last health check
```

Never install foundational tooling using:

```bash
curl https://example | sh
```

without independent download verification and explicit review.

For tools with separate intelligence feeds:

```text
Trivy engine version
≠
Trivy vulnerability DB version

Nuclei engine version
≠
Nuclei template version
```

Track separately.

---

## H20. Installation Mode Matrix

Every external capability receives one explicit installation mode:

```text
BUNDLED
MANAGED_DOWNLOAD
SYSTEM_DEPENDENCY
CONTAINERIZED
REMOTE_API
NOT_SUPPORTED
```

### `BUNDLED`

Use when:

- small enough;
- stable;
- redistribution permitted;
- platform matrix manageable;
- startup path critical.

### `MANAGED_DOWNLOAD`

Use when:

- upstream publishes trustworthy versioned releases;
- binary is too large or updates too often to bundle;
- checksum/signature verification exists;
- optional capability can be installed on demand.

### `SYSTEM_DEPENDENCY`

Use when:

- tool is commonly available;
- redistribution/license makes bundling undesirable;
- installation should remain user/admin controlled.

### `CONTAINERIZED`

Use when:

- dependency is heavy;
- isolation benefit is high;
- container engine is already present.

Containers must never become mandatory for normal coding.

### `REMOTE_API`

Only for capabilities that are truly remote by nature.

Normal local coding must remain functional without hosted proprietary runtime infrastructure.

---

## H21. Capability Registry Contract

Runtime detection of optional tooling should normalize into:

```ts
type CapabilityRecord = {
  capability_id: string
  provider_type:
    | "LIBRARY"
    | "BINARY"
    | "LANGUAGE_SERVER"
    | "BROWSER"
    | "CONTAINER_RUNTIME"
    | "LOCAL_MODEL"
    | "PLUGIN"
    | "REMOTE_SERVICE"

  name: string
  version?: string
  source?: string

  status:
    | "AVAILABLE"
    | "MISSING"
    | "UNHEALTHY"
    | "INCOMPATIBLE_VERSION"
    | "DISABLED"
    | "STALE_DATA"

  capabilities: string[]
  platform: string

  last_probe_at: string
  provenance_ref?: string
}
```

The presence of a cloned donor repository does **not** mean its tool is available in AgentCode runtime.

---

## H22. Stable Extension Boundaries

Doc 07 defines OSS-facing adapter boundaries, while Docs 01–06 define subsystem semantics.

Canonical interface families:

```text
agentcode.provider.v1
agentcode.language.v1
agentcode.tool.v1
agentcode.security.v1
agentcode.skill.v1
agentcode.context_source.v1
agentcode.hook_provider.v1
```

Every extension contract should expose:

```text
identity/version
capability descriptor
health
lifecycle
normalized request
normalized result
normalized error
provenance
```

Upstream-specific command syntax must remain behind the adapter.

---

## H23. One-Authority Invariants

The reference library contains many systems that solve overlapping problems.

AgentCode must not stack them as competing truths.

### Mission truth

One:

```text
AgentCode Kernel / SQLite state
```

Not:

```text
Kernel
+ LangGraph mission state
+ OpenHands event truth
+ Munder GOD agent state
```

### Repository relationships

One:

```text
AgentCode Repository Graph
```

Evidence may come from:

```text
Tree-sitter
LSP
SCIP
Ctags
Zoekt
Git
tests
```

### Persistent project knowledge

One AgentCode-owned knowledge authority.

Letta/Graphiti supply ideas; they do not become competing truth stores.

### Tool permission

One:

```text
AgentCode Tool Broker
```

### Editing

One:

```text
AgentCode ChangeSet/Edit Engine
```

### Security findings

One normalized:

```text
AgentCode Security Finding Store
```

### Skills

One:

```text
AgentCode Skill Engine
```

### Browser

Deterministic core:

```text
Playwright
```

Agentic browser systems can layer on top when necessary.

---

## H24. Reference Library Capability Map

```text
AGENTCODE
│
├── CORE
│   ├── Autonomy Kernel
│   ├── Model Broker
│   ├── OmniRoute
│   ├── Agent Runtime
│   ├── Tool Engine
│   ├── Edit Engine
│   └── Code Intelligence
│
├── KNOWLEDGE
│   ├── Repository Graph
│   ├── Context Engine
│   ├── CONTEXT.md
│   ├── Persistent Decisions
│   └── Evidence Store
│
├── MODES
│   ├── Goal
│   ├── Discuss
│   ├── Design
│   └── Security
│
├── SKILLS
│   ├── language/framework
│   ├── debugging
│   ├── testing
│   ├── browser QA
│   ├── UI/UX
│   ├── AppSec
│   ├── cloud
│   └── AI security
│
└── VERIFICATION
    ├── tests
    ├── type/build
    ├── runtime/browser
    ├── security
    └── independent model review
```

Donors are mapped to capabilities logically.

Do not reorganize clones physically into these folders.

---

## H25. Canonical Known Repository Catalog

The following catalog records **known project research targets**, not proof that every local clone is currently present or unchanged.

Unless an exact SHA/license is separately verified, use:

```text
Source status: REVERIFY_REQUIRED
License status: LICENSE_PENDING
```

| Repository | Priority | Primary role | Default treatment |
|---|---:|---|---|
| OmniRoute | P0 | provider fabric | FORK + MODIFY |
| codex | P0 | tools/edit/sandbox/runtime/UX | STUDY + ADAPT HEAVILY |
| gemini-cli | P0 | checkpoint/hooks/context/skills/policy | ADAPT HEAVILY |
| OpenHands | P0 | workspace/tools/sandbox/event loop | ADAPT HEAVILY |
| software-agent-sdk | P0/P1 | modular software-agent primitives | STUDY / ADAPT |
| aider | P0 | repo map/context/edit/Git | ADAPT HEAVILY |
| mini-swe-agent | P0 | compact coding loop | ADAPT PRINCIPLES |
| SWE-ReX | P0 | execution/sandbox abstraction | ADAPT / POSSIBLE DEPENDENCY |
| tree-sitter | P0 | parsing foundation | TAKE |
| ast-grep | P0 | structural search/transforms | TAKE / WRAP |
| ripgrep | P0 | exact search | TAKE / EXTERNAL TOOL |
| letta-code | P0 | memory-first coding / Git-backed memory concepts | ADAPT HEAVILY |
| letta | P1 | broader stateful memory concepts | STUDY |
| rtk | P0 | deterministic tool-output compression | ADAPT / WRAP HEAVILY |
| caveman | P3 | compression/style comparison | BENCHMARK / STUDY |
| playwright | P0 | deterministic browser automation | TAKE |
| openhack | P0 | security-agent workflow | ADAPT HEAVILY |
| opencode | P1 | LSP/permissions/session/tools | ADAPT |
| cline | P1 | editing/checkpoints/browser/worktrees | ADAPT |
| goose | P1 | desktop/CLI/extensions/MCP | ADAPT |
| langgraph | P1 | graph/checkpoint durability ideas | STUDY / SELECTIVE ADAPT |
| agent-framework | P1 | durable workflows/multi-agent concepts | STUDY |
| smolagents | P3 | minimal agent/tool abstractions | STUDY |
| scip | P1 | semantic indexes | OPTIONAL ADAPT |
| zoekt | P1 | large-repo search acceleration | OPTIONAL WRAP |
| ctags | P3 | fallback symbol extraction | FALLBACK WRAP |
| stack-graphs | P3 | name-resolution concepts | STUDY / ARCHIVED REFERENCE |
| graphiti | P1/P3 | temporal knowledge concepts | FUTURE / STUDY |
| superpowers | P1 | reusable engineering skills | ADAPT SKILLS |
| compound-engineering-plugin | P1 | research/planning/review workflows | ADAPT SKILLS |
| servers | P3 | MCP reference servers | STUDY + COMPATIBILITY |
| onlook | P1 | DOM/source visual editing | ADAPT HEAVILY FOR DESIGN |
| dyad | P1 | local prompt-to-app product flow | ADAPT PRODUCT FLOW |
| bolt.diy | P1 | app-generation/preview workflow | ADAPT WORKFLOW |
| browser-use | P2 | agentic browser reasoning | ADAPT |
| browser-harness | P2 | browser harness/reliability patterns | STUDY / ADAPT |
| semgrep | P2 | SAST engine | WRAP |
| codeql | P2 | deep query/dataflow option | WRAP / LICENSE REVIEW |
| gitleaks | P2 | secret scanning | WRAP |
| osv-scanner | P2 | dependency vulnerability matching | WRAP |
| trivy | P2 | dependency/container/IaC/security scan | WRAP |
| checkov | P2 | IaC analysis | WRAP |
| zaproxy | P2 | DAST | EXTERNAL TOOL ADAPTER |
| nuclei | P2 | template-driven security probing | WRAP |
| nuclei-templates | P2 | Nuclei ruleset | DATA DEPENDENCY |
| scorecard | P2 | supply-chain posture | WRAP / OPTIONAL |
| prowler | P1/P2 | cloud posture | WRAP HEAVILY |
| ScoutSuite | P2 | cloud posture diversity | WRAP / SECONDARY |
| cloudsploit | P2 | cloud posture diversity | WRAP / SECONDARY |
| stratus-red-team | P2 | cloud attack simulation | LAB WRAP |
| pacu | P2 | AWS offensive testing | LAB ONLY |
| cloudgoat | P2 | disposable cloud security lab | TEST LAB |
| promptfoo | P2 | AI evaluation/red-team | WRAP / ADAPT |
| garak | P2 | LLM probe suite | WRAP |
| PyRIT | P2 | advanced AI red-team orchestration | OPTIONAL / POST-V1 DEPTH |
| SWE-bench | P3 | coding evaluation | EVALUATION REFERENCE |
| continue | P3 | historical context/IDE patterns | REFERENCE ONLY |
| Roo-Code | P3 | historical autonomous coding UX | REFERENCE ONLY |
| daytona | P3 | sandbox/remote dev architecture | REFERENCE ONLY |
| munder-difflin | P1/P3 | coordination/mailbox/blackboard concepts | STUDY + ADAPT SELECTIVELY |
| skills / trailofbits-skills | P1 | security skills | ADAPT CAREFULLY |
| letta-skills | P1 | memory/workflow skills | ADAPT SELECTIVELY |

This table is the **planning catalog**.

The generated `AGENTCODE_REFERENCE_CATALOG.md` should later include verified Git and license metadata.

---

## H26. Prior Observations Requiring Reverification

Earlier project work reported:

```text
opencode
- approximately 1.0 GB local clone
- branch: dev
- clean
- reportedly up-to-date with origin/dev
- observed commit prefix: 9b0dd36c...
- observation date: 18 Aug 2026
```

This hardening environment cannot access:

```text
/Volumes/T7 Shield/GitHub-Repos-dependency
```

and therefore does **not** independently certify those details.

Catalog status:

```text
PRIOR_OBSERVATION_REVERIFY_REQUIRED
```

Before source-level extraction:

```bash
git -C "$ROOT/opencode" status --short --branch
git -C "$ROOT/opencode" rev-parse HEAD
git -C "$ROOT/opencode" remote -v
```

The same rule applies to all donor SHAs mentioned in earlier conversations.

---

## H27. Special Donor Decisions Already Locked

### Munder Difflin

**Use/Study:**

```text
durable blackboard
mailboxes
supervision/coordination ideas
single-writer state
per-agent directories
SQLite/FTS concepts
worktree coordination
anti-livelock ideas
```

**Reject:**

```text
LLM GOD / supreme model as source of truth
office/avatar/Pixi product metaphor
filesystem-only mission truth
PTY-first tool execution
provider-specific runtime hooks
```

Munder Difflin remains a donor/reference, not AgentCode foundation.

### Cursor

Cursor is not an OSS donor clone for AgentCode core.

Treat it as an **external product/architecture reference**.

Borrow concepts already identified through public research:

```text
Merkle/content-hash incremental indexing
incremental verification
nested/scoped project rules
background-agent product behavior
rich context source ideas
```

Do not claim access to closed Cursor implementation.

### Continue

Historical/reference only.

### Roo Code

Historical/archived/shutdown reference only.

### Daytona

Reference only; do not create new foundational V1 dependence on the public implementation.

### Stack Graphs

Archived name-resolution research reference.

### Letta Code

High-priority memory/context donor.

Particularly valuable for:

```text
Git-backed memory concepts
persistent coding state
context hierarchy
cross-session continuity
structured compaction
```

### RTK

High priority for deterministic tool-output compression.

Locked AgentCode rule:

```text
raw output retained locally
compressed model view is derived
```

Do not assume RTK automatically equals lower provider billing without measurement.

### Caveman

Study terse output/compression style and benchmark against RTK where useful.

Do not stack both systems blindly.

---

## H28. Extraction Depth Levels

Canonical depth:

```text
L0 — CATALOG ONLY
L1 — DOCUMENTATION UNDERSTANDING
L2 — SOURCE LOCATED
L3 — CONTROL FLOW TRACED
L4 — EXECUTED / TESTED
L5 — AGENTCODE PROTOTYPE
```

### L0

Known repository and broad role.

Not sufficient for implementation.

### L1

README/docs/API understanding.

Useful for navigation, not foundational adoption.

### L2

Exact implementation files/symbols identified at pinned SHA.

### L3

Mechanism traced through:

```text
entry point
→ configuration/parser
→ orchestration
→ core implementation
→ persistence/external process
→ failure path
→ relevant tests
```

### L4

Behavior executed in a controlled environment or verified via robust upstream tests where execution is impractical.

### L5

Critical mechanism reproduced through an AgentCode-focused spike.

### Required depth policy

| Reuse type | Typical minimum |
|---|---|
| foundational architecture adaptation | L3–L5 |
| source reuse | L3 + license cleared; L4 preferred |
| small library dependency | L2–L4 |
| external scanner wrapper | L2–L4 |
| reference-only idea | L1–L3 |
| rejected architecture | sufficient evidence to justify rejection |

---

## H29. Source Tracing Method

For every implementation mechanism, answer these questions in order.

### 1. What is the public entry point?

Examples:

```text
CLI command
library function
HTTP handler
provider registration
tool declaration
event handler
```

### 2. Where is configuration parsed?

Identify:

```text
defaults
environment
project config
flags
policy
```

### 3. Where is the real core behavior?

Do not confuse interface/type declarations with implementation.

### 4. What state does it read/write?

Examples:

```text
filesystem
SQLite
Git
memory
cache
process registry
provider state
```

### 5. What external effects occur?

Examples:

```text
spawn command
network request
Git mutation
browser launch
file write
cloud mutation
```

### 6. How does it fail?

Trace:

```text
timeout
nonzero exit
exception/result
retry
fallback
cleanup
```

### 7. Which upstream tests prove intended behavior?

Record exact paths/symbols.

### 8. Which assumptions conflict with AgentCode?

Examples:

```text
single-agent assumption
hosted backend
global memory
unrestricted shell
provider-specific coupling
```

---

## H30. Campaign Research Packet

Do not tell an agent:

```text
Read all reference repos.
```

Create:

```ts
type ExtractionCampaignPacket = {
  campaign_id: string
  objective: string

  architecture_refs: string[]

  donor_subset: Array<{
    repository_id: string
    priority: "PRIMARY" | "SECONDARY" | "CONTROL"
    source_sha?: string
  }>

  questions: string[]

  known_candidate_paths?: string[]
  search_terms: string[]

  excluded_scope: string[]

  required_depth: string

  required_prototypes: string[]
  required_benchmarks: string[]

  required_outputs: string[]
  review_level: string

  token_or_time_budget?: string
}
```

### Context discipline

Campaign agent receives:

```text
relevant Doc section
+
campaign packet
+
catalog entries
+
target donor repos
+
existing extraction records
```

not all Docs 01–07 and all repositories unless truly necessary.

---

## H31. Extraction Agent Safety

Donor source, comments, README instructions, issues, fixtures, and scanner output are **untrusted data**.

They cannot instruct the extraction agent to:

```text
ignore AgentCode architecture
run arbitrary install scripts
upload credentials
change project files
disable safety policy
clone unrelated repositories
```

### Default donor clone permissions

```text
READ_ONLY
NO CREDENTIALS
NO WRITE TO DONOR
NO BROAD NETWORK EXECUTION
```

If execution is necessary:

```text
disposable environment
synthetic credentials/data
recorded commands
bounded network
no project secrets
```

---

## H32. Repository Contamination Prevention

Do not edit donor clones during ordinary research.

If an experiment requires changes:

```text
create disposable worktree/branch
record base SHA
record patch
run experiment
delete/reset disposable environment
```

Never leave a donor clone silently modified and then use its behavior as though it were upstream.

Catalog field:

```text
dirty = true
```

blocks new source-verification claims until the modification is understood.

---

## H33. Prototype Record

```ts
type PrototypeRecord = {
  prototype_id: string
  capability: string
  hypothesis: string

  source_extraction_refs: string[]

  implementation: {
    location: string
    scope: string
    deliberately_not_implemented: string[]
  }

  fixture_refs: string[]

  metrics: Record<string, string | number>
  observed_failures: string[]

  result:
    | "SUPPORTED"
    | "PARTIALLY_SUPPORTED"
    | "REJECTED"
    | "INCONCLUSIVE"

  adoption_effect: string
  cleanup_status: string
}
```

Candidate V1 spikes:

```text
OmniRoute top-K/failover
daemon/local IPC
Tree-sitter normalized symbols
LSP lifecycle
worktree isolation
transactional ChangeSet recovery
RTK compression
DOM-to-source mapping
scanner normalization
MCP trust boundary
```

---

## H34. Benchmark Record

```ts
type ExtractionBenchmark = {
  benchmark_id: string
  question: string

  candidates: string[]
  fixtures: string[]

  environment: {
    hardware_profile: string
    os: string
    architecture: string
    tool_versions: Record<string, string>
  }

  repetitions: number

  metrics: Array<{
    name: string
    unit: string
    direction: "LOWER_BETTER" | "HIGHER_BETTER"
  }>

  correctness_ground_truth: string

  results: Record<string, unknown>

  noise_or_limitations: string[]

  decision?: string
}
```

Popularity, GitHub stars, or benchmark claims from upstream marketing never substitute for AgentCode's own fit benchmark.

---

## H35. Benchmark Corpus

Create controlled fixture families:

```text
BENCH-TS-SMALL
BENCH-NEXT-FULLSTACK
BENCH-PY
BENCH-RUST
BENCH-GO
BENCH-POLYGLOT
BENCH-MONOREPO-LARGE
BENCH-EDIT-CONFLICT
BENCH-BROWSER-DYNAMIC
BENCH-SEC-VULNERABLE
BENCH-AI-SECURITY
BENCH-CLOUD-LAB
```

Every fixture records:

```text
fixture version
ground truth
size
languages
frameworks
expected tests
known defects
expected relevant files
expected source relationships
```

---

## H36. Known Benchmark Decisions Still Pending

### RTK vs Caveman

Measure:

```text
raw bytes/tokens
compressed bytes/tokens
loss of error information
model task success
need to fetch raw output
latency
CPU/RAM
```

### ripgrep / Zoekt activation

Measure:

```text
repository size
query latency
index build cost
incremental update cost
RAM
disk
```

The decision is not “one replaces the other.”

Likely:

```text
ripgrep always available
Zoekt activates when repository/query profile justifies it
```

subject to benchmark.

### LSP vs SCIP

Measure incremental semantic value.

SCIP remains optional if LSP + Tree-sitter gives sufficient practical coverage.

### Browser Use vs Playwright

Deterministic known flow:

```text
prefer Playwright
```

Unknown/dynamic semantic exploration:

```text
agentic browser layer may help
```

### Edit formats

Benchmark by model class and task type, not globally.

### Security scanner overlap

Measure:

```text
unique confirmed findings
duplicate rate
runtime
memory
maintenance cost
```

before running overlapping tools by default.

---

## H37. Review Levels

### R0 — Catalog note

No independent review required beyond basic metadata check.

### R1 — Study-only extraction

Technical review recommended.

### R2 — Runtime dependency / wrapper

Requires:

```text
technical review
license review
integration test plan
```

### R3 — Source reuse / foundational adaptation

Requires:

```text
independent technical reviewer
license review
prototype where applicable
AgentCode-native tests
provenance manifest
```

### R4 — Fork

Requires:

```text
architecture review
technical review
license review
maintenance owner
upstream sync policy
compatibility test suite
```

OmniRoute is an R4-class decision.

---

## H38. Canonical Campaign Numbering

The old document used a sequence that later project planning refined.

The canonical campaign files are now:

```text
docs/extraction/
├── 00_REFERENCE_CATALOG_AND_LICENSES.md
├── 01_PROVIDER_FABRIC_EXTRACTION.md
├── 02_AGENT_RUNTIME_EXTRACTION.md
├── 03_TOOLS_EDITING_EXTRACTION.md
├── 04_CODE_INTELLIGENCE_EXTRACTION.md
├── 05_CONTEXT_MEMORY_EXTRACTION.md
├── 06_GIT_WORKTREES_EXTRACTION.md
├── 07_SANDBOX_PERMISSIONS_EXTRACTION.md
├── 08_SKILLS_HOOKS_MCP_EXTRACTION.md
├── 09_BROWSER_QA_EXTRACTION.md
├── 10_DESIGN_STUDIO_EXTRACTION.md
├── 11_APPSEC_EXTRACTION.md
├── 12_CLOUD_SECURITY_EXTRACTION.md
├── 13_AI_SECURITY_EXTRACTION.md
└── 14_PRODUCT_UX_EXTRACTION.md
```

Legacy references such as:

```text
02_CODE_INTELLIGENCE
03_CONTEXT_MEMORY
04_AGENT_RUNTIME
```

must be migrated or aliased.

Doc 09/11 determine project scheduling; this numbering only standardizes extraction artifacts.

---

# CAMPAIGN 00 — REFERENCE GOVERNANCE, CATALOG & LICENSES

## H39. Objective

Create the durable inventory that prevents every later research agent from rediscovering repository metadata.

### Primary inputs

All known top-level donor directories.

### Required outputs

```text
AGENTCODE_REFERENCE_CATALOG.md
agentcode_reference_catalog.json
OSS_LICENSE_MATRIX.md
third_party_manifest.json
THIRD_PARTY_NOTICES.md skeleton
```

### Procedure

For every repository:

1. verify local path;
2. record remote;
3. record HEAD;
4. record branch/dirty/shallow state;
5. locate license/notice files;
6. classify priority/categories;
7. attach relevant campaigns;
8. record maintenance/archive evidence;
9. mark license status;
10. record whether it is expected to ship.

### Exit gate

No P0/P1 donor may remain:

```text
unknown local identity
```

even if its detailed extraction is still pending.

---

# CAMPAIGN 01 — PROVIDER FABRIC

## H40. Objective

Determine precisely which OmniRoute mechanisms AgentCode will fork/retain, which capabilities belong in Model Broker instead, and what supporting provider abstractions are worth borrowing from Codex/OpenCode/Goose.

### Primary donors

```text
OmniRoute
```

### Secondary

```text
Codex
OpenCode
Goose
```

### Read first

Doc 01 hardened sections on:

```text
Model Broker ownership
OmniRoute ownership
ProviderConnection
QuotaDomain
health/circuit breaker
candidate generation
routing decision
```

### Source questions

Trace:

```text
provider registration
model catalog
connection/credential references
quota observation
health state
cooldown/circuit breaker
request normalization
streaming
error normalization
fallback
```

### Explicit non-goals

Do not extract mission/task authority from any provider layer.

Do not move:

```text
risk-based model-family diversity
Planner/Coder/Verifier role policy
completion authority
```

into OmniRoute.

### Required prototypes

```text
top-K candidate enumeration
connection failover
quota-domain sharing
local Ollama candidate
```

### Exit

Implementation packet defines:

```text
fork base
retained upstream modules
AgentCode patches
Broker boundary
compatibility tests
upstream sync policy
```

---

# CAMPAIGN 02 — AGENT RUNTIME

## H41. Objective

Extract excellent execution-loop, session, checkpoint, event, and recovery patterns without importing a second mission state machine.

### Primary donors

```text
Codex
mini-SWE-agent
OpenHands
software-agent-sdk
Gemini CLI
```

### Secondary

```text
Munder Difflin
LangGraph
Microsoft Agent Framework
smolagents
```

### Core questions

Trace:

```text
agent/session construction
model turn
tool-call dispatch
tool result incorporation
continuation
stop condition
checkpoint
resume
cancellation
failure/retry
subagent/session spawn
```

### Compare

```text
simple loop vs framework graph
in-memory vs durable state
event records
session replacement
tool execution ownership
```

### Reject

Any pattern where:

```text
LLM response
→ directly marks mission complete
```

or where agent transcript becomes authoritative mission truth.

### Prototype

A minimal Worker that:

```text
starts
uses tools
edits fixture
tests
checkpoints
dies
restarts through new session
continues
```

under Kernel-owned state.

---

# CAMPAIGN 03 — TOOLS & EDITING

## H42. Objective

Extract Codex/Claude-class repository manipulation competence while preserving Doc 04 Tool Broker and ChangeSet authority.

### Primary donors

```text
Codex
Aider
Cline
Gemini CLI
OpenHands
```

### Secondary

```text
OpenCode
SWE-ReX
```

### Trace separately

#### Filesystem

```text
read
range read
write
move
delete
path normalization
symlink handling
```

#### Search

```text
exact search
file discovery
structured search
```

#### Shell

```text
spawn
cwd/env
timeout
streaming
process tree
background process
```

#### Editing

```text
search/replace
patch
whole file
structured edit
preconditions
validation
rollback
```

### Required comparison

Create matrix:

| Mechanism | Codex | Aider | Cline | Gemini | AgentCode choice |
|---|---|---|---|---|---|

for:

```text
patch representation
stale edit handling
multi-file edits
error reporting
model repair loop
```

### Prototype

Crash during 3-file ChangeSet and prove AgentCode rollback/reconcile.

---

# CAMPAIGN 04 — CODE INTELLIGENCE

## H43. Objective

Select and adapt the strongest primitives for exact, structural, semantic, graph, test, and large-repo intelligence.

### Primary donors/primitives

```text
Aider
Tree-sitter
ast-grep
ripgrep
Codex
OpenCode
```

### Secondary

```text
SCIP
Zoekt
Ctags
Stack Graphs
```

### Trace

```text
Aider repo map
symbol extraction/ranking
Tree-sitter parser lifecycle
ast-grep invocation/result
LSP lifecycle in donor agents
SCIP ingestion
Zoekt index/query lifecycle
ignore handling
incremental update
cache identity
```

### Required evidence

For Aider repo-map extraction:

```text
entry point
symbol/tag extraction
graph/ranking
token-budget behavior
file selection
test coverage
```

not merely:

```text
“Aider uses repo maps.”
```

### Cursor external-reference note

No Cursor source extraction exists.

Use only documented/publicly researched concepts:

```text
content-hash/Merkle incremental indexing
scoped rules
incremental verification
background-agent UX
```

### Prototype

```text
10k-file fixture
modify 3 files
prove only affected structural artifacts rebuild
```

---

# CAMPAIGN 05 — CONTEXT & MEMORY

## H44. Objective

Extract persistence/compaction/token-efficiency mechanisms that improve Doc 02 without introducing competing memory truth.

### Primary

```text
Letta Code
Gemini CLI
RTK
Aider
```

### Secondary

```text
Letta
Cline
Graphiti
Caveman
```

### Trace

```text
memory persistence
memory update
Git-backed memory behavior
context selection
context compaction
raw-history retention
handoff
prompt cache patterns
tool-output compression
```

### Key design test

For every donor memory mechanism ask:

```text
Is this authoritative state,
derived summary,
conversation memory,
or retrieval accelerator?
```

Map it into AgentCode's canonical:

```text
SQLite truth
Git evidence
structured project knowledge
generated CONTEXT.md
context packs
raw evidence
```

### Prototype

Model/session replacement should resume known fixture task without repeating broad repository discovery.

---

# CAMPAIGN 06 — GIT & WORKTREES

## H45. Objective

Extract robust isolation/checkpoint/integration mechanics.

### Primary

```text
Codex
Cline
Aider
Superpowers
Munder Difflin
```

### Trace

```text
worktree creation
branch naming
dirty repo handling
checkpoint commits
rollback
worker isolation
merge/integration
conflict handling
cleanup
```

### Critical questions

```text
Who owns worktree lifecycle?
What happens if user modifies base branch?
What happens if worktree disappears?
What metadata survives agent restart?
How is destructive Git prevented?
```

### Prototype

Two Workers modify independent files in separate worktrees, then integrate.

Second test intentionally causes shared-file conflict.

---

# CAMPAIGN 07 — SANDBOX & PERMISSIONS

## H46. Objective

Extract strong execution isolation and approval ideas while conforming to Doc 04's risk/capability policy.

### Primary

```text
Codex
OpenHands
SWE-ReX
Gemini CLI
OpenCode
Cline
```

### Trace

```text
policy representation
command interception
cwd/workspace boundary
filesystem scope
network scope
environment filtering
approval
sandbox launch
failure
process cleanup
```

### macOS requirement

Research must explicitly assess what is genuinely enforceable on macOS.

Do not claim a theoretical sandbox capability that cannot be implemented reliably on the target.

### Prototype

Attempt:

```text
workspace symlink -> ~/.ssh
write through symlink
```

Expected:

```text
denied
```

---

# CAMPAIGN 08 — SKILLS, HOOKS & MCP

## H47. Objective

Normalize useful skill/workflow ecosystems into one AgentCode extensibility model.

### Donors

```text
Gemini CLI
Superpowers
Compound Engineering
Trail of Bits Skills
Letta Skills
OpenHands
official MCP server examples
```

### Trace

```text
skill discovery
metadata
scope
progressive loading
hooks
event triggers
timeouts
MCP discovery
capability schemas
transport
error handling
```

### Special requirement

Determine:

```text
skill content license
skill executable assets
third-party commands invoked by skill
```

separately.

### Trust test

A malicious MCP description or skill text must not:

```text
elevate Tool Broker permissions
change Kernel state
override project rules
```

---

# CAMPAIGN 09 — BROWSER & QA

## H48. Objective

Use Playwright as deterministic browser core and identify where agentic browser layers add real value.

### Primary

```text
Playwright
```

### Supporting

```text
Browser Use
Browser Harness
Cline
```

### Trace

```text
browser launch
context/profile
page lifecycle
action API
selectors/accessibility tree
screenshots
network/console
download/upload handling
crash recovery
```

### Comparison

Known flow:

```text
Playwright deterministic actions
```

Unknown semantic exploration:

```text
Browser Use-style reasoning
```

### Prototype

Dev server + browser crash + restart + session recreation without losing mission state.

---

# CAMPAIGN 10 — DESIGN STUDIO

## H49. Objective

Extract preview/source-mapping/app-builder mechanics without inheriting donor visual identity.

### Donors

```text
Onlook
Dyad
Bolt.diy
```

### Trace

```text
preview lifecycle
source project management
DOM instrumentation
component/source mapping
visual selection
edit application
hot reload
generation loop
provider/BYOK boundaries
```

### License focus

Explicitly inspect:

```text
Dyad pro/commercial boundaries
Bolt source vs WebContainer/runtime/API terms
assets/fonts/icons
```

### Prototype

Narrow React/Next.js:

```text
render element
→ select DOM node
→ resolve likely component/file
→ send target through Doc02 context
→ edit via Doc04
→ rerender
```

If source mapping is unreliable, keep it optional rather than advertising universal support.

---

# CAMPAIGN 11 — APPSEC

## H50. Objective

Produce adapter and workflow decisions for baseline V1 security.

### Core tool donors

```text
OpenHack
Trail of Bits Skills
Semgrep
Gitleaks
OSV-Scanner
Trivy
Checkov
```

### Conditional

```text
CodeQL
ZAP
Nuclei
Scorecard
```

### Trace for each scanner

```text
installation
version discovery
target scope
structured output
exit-code semantics
partial failure
rules/database version
timeout/cancel
raw artifact
```

### Security workflow donors

Study OpenHack / security skills for:

```text
recon
hypothesis
validation
false-positive reduction
differential review
verification
```

but Doc 05 owns finding state and attack-path architecture.

---

# CAMPAIGN 12 — CLOUD SECURITY

## H51. Objective

Integrate cloud posture evidence and lab-safe attack simulation without making cloud exploitation part of normal coding.

### Posture donors

```text
Prowler
ScoutSuite
CloudSploit
```

### Lab/adversarial donors

```text
Stratus Red Team
Pacu
CloudGoat
```

### Required decisions

```text
Prowler primary posture role
secondary-tool overlap policy
credential scope
read-only default
resource normalization
lab-only active techniques
cleanup
```

### Critical rejection

Never adopt:

```text
“credentials can reach it, therefore it is in scope.”
```

AgentCode uses explicit cloud scope from Doc 05.

---

# CAMPAIGN 13 — AI SECURITY

## H52. Objective

Determine which external harnesses best cover deterministic and advanced AI-security testing.

### Donors

```text
Promptfoo
Garak
PyRIT
```

### Compare

```text
test definition format
multi-turn orchestration
target adapters
scoring
reporting
prompt injection
tool/agent testing
dataset/plugin model
```

### V1 expectation

Do not require all three.

Likely policy:

```text
Promptfoo: optional/valuable deterministic harness
Garak: optional probe suite
PyRIT: advanced optional/post-V1 depth
```

subject to Doc 05 release scope.

---

# CAMPAIGN 14 — PRODUCT UX

## H53. Objective

Extract interaction mechanics that support Doc 06's quiet, minimal engineering product.

### Donors

```text
Codex
OpenCode
Cline
Goose
Dyad
Onlook
```

### Study

```text
task/mission entry
background execution
diff review
activity grouping
approval UX
reconnect/session history
preview UX
project navigation
```

### Explicit rejection

Do not copy donor visual identity.

Do not inherit:

```text
chat-only product structure
terminal-first default
provider dashboard as home
agent avatars
```

### Output

Concrete UI interaction patterns and implementation constraints mapped into Doc 06.

---

## H54. Campaign Report Structure

Every campaign report must contain substantive sections:

```text
1. Scope
2. Architecture constraints
3. Donor SHAs
4. License status
5. Questions
6. Source map
7. Mechanism traces
8. Data structures
9. Failure behavior
10. Upstream tests
11. Comparative matrix
12. TAKE/ADAPT/WRAP/STUDY/REJECT decisions
13. Prohibited inheritance
14. Prototypes
15. Benchmarks
16. AgentCode destination contracts
17. Implementation packets
18. Open blockers
19. Independent review
20. Completion evidence
```

A report consisting primarily of:

```text
repo descriptions
README summaries
feature lists
```

fails.

---

## H55. Source-Level Evidence Format

For a mechanism:

```markdown
### EXT-CI-004 — Aider Repository Map

Status:
EXTRACTION_PENDING / VERIFIED

Source:
repo:aider

Commit:
<required exact SHA>

Entrypoint:
<exact path:symbol>

Core implementation:
<path:symbol>
<path:symbol>

Tests:
<path:test>

Trace:
entry → collection → graph/ranking → token fit → output

Behavior:
...

Failure behavior:
...

AgentCode decision:
ADAPT

Destination:
Code Intelligence / RepoMap

Why:
...

License:
LICENSE_PENDING / review ID
```

No `...` is allowed in a final verified record.

`TO_BE_VERIFIED_DURING_CAMPAIGN` is allowed only while status is pending.

---

## H56. Implementation Packet

Doc 11 Work Packages should consume an `ImplementationPacket`, not whole donor repositories.

```ts
type ImplementationPacket = {
  packet_id: string

  capability: string
  agentcode_destination: string

  architecture_refs: string[]

  selected_mechanism: {
    adoption_decision_ref: string
    extraction_refs: string[]
  }

  upstream_refs: Array<{
    repository_id: string
    commit_sha: string
    paths: string[]
  }>

  normalized_contract: {
    inputs: string[]
    outputs: string[]
    errors: string[]
    invariants: string[]
  }

  retain: string[]
  modify: string[]
  prohibit: string[]

  license_obligations: string[]

  prototype_refs: string[]
  benchmark_refs: string[]

  tests_to_build: string[]
  failure_tests: string[]

  doc10_gate_refs: string[]

  unresolved_questions: string[]
}
```

This packet is the primary bridge:

```text
OSS research
→ AgentCode implementation
```

---

## H57. Implementation Context Discipline

For a task:

```text
Implement Context Pack Builder
```

the Worker should receive:

```text
Doc 02 relevant sections
ImplementationPacket
AgentCode current context code
specific Letta/Aider/RTK source pointers
relevant tests
acceptance criteria
```

not:

```text
all Letta source
all Aider
all Gemini
all RTK
all Docs 01–11
```

This directly applies AgentCode's own context-efficiency principles to its development process.

---

## H58. Concrete Extraction Example — ripgrep

This example demonstrates record depth without pretending an exact local source path was verified in this environment.

```text
Extraction ID:
EXT-CI-RG-001

Capability:
Exact repository text search

Source:
repo:ripgrep

Source SHA:
TO_BE_VERIFIED_DURING_CAMPAIGN

Source files:
TO_BE_VERIFIED_DURING_CAMPAIGN

Classification:
TAKE

Reuse form:
EXTERNAL_TOOL or packaged dependency decision

AgentCode destination:
Code Intelligence / ExactSearchAdapter
```

### Observed/expected upstream role to verify

ripgrep should remain responsible for:

```text
fast recursive text search
ignore behavior
regex/literal matching
line/column result production
```

AgentCode owns:

```text
workspace scope
query limits
structured result parsing
sensitive path filtering
provenance
context relevance
tool evidence
```

### Required extraction work

Verify:

1. CLI flags needed for stable structured output;
2. behavior with binary files;
3. Gitignore/hidden files;
4. invalid regex errors;
5. cancellation;
6. exit codes for no match vs actual failure;
7. platform availability;
8. license/distribution.

### AgentCode tests

```text
literal match
regex match
no result
invalid regex
ignored path
symlink safety
large output truncation
process cancellation
```

### Adoption outcome

Expected:

```text
TAKE primitive
WRAP behind ExactSearchAdapter
```

unless campaign evidence uncovers a blocker.

---

## H59. Concrete Extraction Example — Tree-sitter

```text
Extraction ID:
EXT-CI-TS-001

Capability:
Incremental structural parsing

Source:
repo:tree-sitter

SHA:
TO_BE_VERIFIED_DURING_CAMPAIGN

Classification:
TAKE

Reuse form:
DEPENDENCY
```

Tree-sitter should own:

```text
parser
syntax tree
incremental parse primitive
grammar execution
```

AgentCode must own:

```text
language adapter registry
symbol normalization
file identity
parse cache key
content hash
generation publication
repository graph
freshness
failure/degraded mode
```

### Extraction questions

Verify:

```text
parser lifecycle
incremental edit API
tree reuse rules
thread/process safety
grammar loading
parse error representation
memory ownership
platform build requirements
```

### Prototype

Fixture:

```text
10,000 files
initial parse
modify 3 files
```

Evidence:

```text
files reparsed
elapsed time
peak RAM
cache hits
correct symbol changes
```

### Reject

Do not let Tree-sitter's syntax tree become:

```text
the entire AgentCode semantic graph.
```

It is one evidence source.

---

## H60. Concrete Extraction Example — Playwright

```text
Extraction ID:
EXT-BR-PW-001

Capability:
Deterministic browser execution

Classification:
TAKE

Reuse form:
DEPENDENCY + managed browser binaries
```

Playwright owns:

```text
browser protocol
page/context control
selectors
network/console APIs
screenshots/traces
```

AgentCode owns:

```text
session registry
profile isolation
Tool Broker policy
dev-server lifecycle
evidence IDs
screenshot sensitivity
retry/cancellation
Kernel task integration
visual QA orchestration
```

### Source-level campaign must verify

```text
browser binary install/update mechanics
context isolation
timeout behavior
process cleanup
trace storage
download handling
platform support
```

### Acceptance

A browser crash must be recoverable without losing mission state.

---

## H61. Concrete Extraction Example — RTK

```text
Extraction ID:
EXT-CTX-RTK-001

Capability:
Deterministic terminal/tool-output compression

Classification:
ADAPT / WRAP HEAVILY

Reuse form:
EXTERNAL_TOOL or internalized compatible filter after review
```

Locked AgentCode rule:

```text
RAW OUTPUT
    ↓ persisted
FILTERED VIEW
    ↓ model
```

Never:

```text
compress
→ discard raw evidence
```

### Benchmark

Corpus:

```text
Git status/log/diff
Cargo test
npm test
pytest
TypeScript build
Rust compiler errors
directory listings
```

Measure:

```text
raw tokens
filtered tokens
retained actionable failures
model success
raw-refetch frequency
latency
```

Compare Caveman only where its mechanism is genuinely comparable.

---

## H62. Concrete Extraction Example — OmniRoute

```text
Extraction ID:
EXT-PROV-OMNI-001

Capability:
Provider connection/catalog/failover fabric

Classification:
FORK + MODIFY

Reuse form:
FORKED SOURCE
```

### Required source extraction

Pin exact base SHA.

Trace:

```text
provider registry
catalog discovery
credential references
connection selection
health
quota
cooldown
request
stream
error
fallback
normalization
```

### Fork-specific implementation packet

Must list:

```text
unchanged upstream modules
modified modules
new AgentCode modules
compatibility tests
patch IDs
upstream sync strategy
```

### Boundary test

A task risk classification must not appear inside OmniRoute fork merely because routing needs it.

Model Broker receives top candidates and applies AgentCode task semantics.

---

## H63. Concrete Extraction Example — Semgrep

```text
Extraction ID:
EXT-SEC-SEMGREP-001

Capability:
Static security candidate generation

Classification:
WRAP

Reuse form:
EXTERNAL_TOOL
```

Semgrep owns:

```text
rule execution
AST/dataflow analysis supported by its engine
native findings
```

AgentCode owns:

```text
applicability
rule selection
tool install/version
scope
raw artifact
normalized finding instance
dedupe
confidence
validation
root finding
remediation
regression
```

### Critical rule

Semgrep result:

```text
candidate evidence
```

not:

```text
confirmed vulnerability.
```

### Required adapter tests

```text
tool missing
unsupported project
normal finding
no finding
timeout
partial output
invalid JSON
rule version change
path scope
prompt-injection text in finding
```

---

## H64. Build-vs-Borrow Decision Matrix

For every capability, score qualitatively:

| Dimension | Build AgentCode-native | Borrow/Wrap |
|---|---|---|
| strategic differentiation | high favors build | low favors borrow |
| maturity upstream | low favors build | high favors borrow |
| architecture fit | poor favors build/adapt | strong favors reuse |
| license | blocked favors reimplement | clear favors reuse |
| maintenance burden | lower own code may win | upstream maintenance may win |
| platform coverage | weak upstream may block | strong upstream favors reuse |
| security boundary | risky embedded code may favor process wrapper | isolated tool favors wrap |
| API churn | high churn favors adapter boundary | stable API favors dependency |
| amount of code | small specialized code may build | large mature engine should reuse |
| tests | weak upstream reduces benefit | mature tests increase reuse confidence |

The decision should explain tradeoffs, not just output a number.

---

## H65. Upstream Tests and Independent Test Writing

Upstream tests are useful to learn:

```text
edge cases
expected failures
platform behavior
concurrency
invariants
```

But AgentCode should normally write its own behavior tests around AgentCode interfaces.

Directly copying tests requires license clearance just like other source.

If an upstream test reveals a useful case:

```text
record source test pointer
→ independently encode AgentCode fixture/assertion
```

when license/provenance policy calls for reimplementation.

---

## H66. No Hidden Third-Party Source

Substantial copied/adapted code may not lose provenance.

Acceptable tracking:

```text
third_party_manifest entry
extraction_id
adoption_decision_id
upstream source SHA/path
license notice
```

Code comments should be added where required or genuinely useful, but there is no requirement to spam every file with:

```text
Copied from ...
```

The machine-readable manifest is the durable trace.

---

## H67. Donor Lifecycle and Retirement

If upstream becomes:

```text
archived
compromised
license-changed
unmaintained
incompatible
unavailable
```

response depends on reuse form.

### Reference only

Historical extraction remains valid as a description of pinned source.

### Dependency

Evaluate:

```text
pin current version
replace
fork
remove capability
```

### Managed external binary

Disable update channel if provenance compromised.

Use last trusted version only if security policy permits.

### Fork

AgentCode owns maintenance and security response.

---

## H68. Reference Hunting Is Closed

This is a locked project decision.

The current donor library is sufficient to begin V1 extraction and implementation.

A new repository may be added only after:

```text
specific missing capability
        ↓
search existing catalog/extractions
        ↓
prove current donors insufficient
        ↓
identify 1–2 targeted new sources
        ↓
license/dependency intake
        ↓
catalog entry
```

Do not restart:

```text
find every autonomous agent on GitHub
```

research.

---

## H69. Implementation Language Boundary

The likely V1 direction remains:

```text
Rust
→ durable local Kernel/tool/process/index-critical native components

TypeScript
→ Tauri/React UI and OmniRoute/provider layer where compatible

Python
→ optional external/security/AI tooling when ecosystem requires it

External binaries
→ mature scanners/searchers/tooling
```

This remains a:

```text
CONSTRAINED_IMPLEMENTATION_DECISION
```

until implementation ADRs lock exact package boundaries.

Doc 07 does not independently override Docs 01–06.

---

## H70. Local IPC Boundary

The old list:

```text
Unix socket
localhost HTTP
WebSocket
Tauri IPC
```

represents implementation candidates, not four simultaneous APIs.

The final IPC mechanism must be resolved through ADR/spike and satisfy:

```text
local-only default
versioned protocol
request IDs/idempotency
snapshot + streaming events
reconnect
backpressure
low overhead
desktop/daemon lifecycle separation
filesystem/OS permission safety
diagnostics
```

Docs 03 and 06 define the logical contract.

---

## H71. Platform Compatibility Record

Every shipped/bundled/managed external tool should record:

```ts
type PlatformSupport = {
  component: string

  darwin_arm64:
    | "VERIFIED"
    | "SUPPORTED_UPSTREAM"
    | "UNSUPPORTED"
    | "UNKNOWN"

  darwin_x64:
    | "VERIFIED"
    | "SUPPORTED_UPSTREAM"
    | "UNSUPPORTED"
    | "UNKNOWN"

  windows_x64:
    | "VERIFIED"
    | "SUPPORTED_UPSTREAM"
    | "UNSUPPORTED"
    | "UNKNOWN"

  linux_x64?:
    | "VERIFIED"
    | "SUPPORTED_UPSTREAM"
    | "UNSUPPORTED"
    | "UNKNOWN"

  notes: string[]
}
```

Current primary hardware target:

```text
macOS Apple Silicon / 8 GB Mac
```

Do not claim Windows support merely because an upstream binary exists.

---

## H72. 8 GB Mac Extraction Gate

Every candidate foundational mechanism must answer:

```text
What is baseline RAM?
What is peak RAM?
Does it remain resident?
Can it be unloaded?
Does it duplicate another index/runtime?
Does it spawn children?
Does it use a browser?
Does it require Docker?
How does it behave under memory pressure?
```

Examples:

### Zoekt

Optional until benefit exceeds index/RAM cost.

### local visual model

Load only when needed.

### several LSPs

Resource Governor should close idle ones.

### heavy security tools

Schedule rather than run all concurrently.

---

## H73. Supply-Chain Review During Extraction

Before adopting a package/tool:

1. inspect package manifests;
2. inspect install/postinstall scripts;
3. identify downloaded binaries;
4. identify network fetches;
5. identify dynamic plugin loading;
6. identify bundled native code;
7. check release provenance;
8. record checksum/signature strategy;
9. identify update channel;
10. identify whether a compromised upstream update can execute automatically.

No auto-update path should bypass AgentCode's provenance controls.

---

## H74. Scanner and Ruleset Updates

Security engines and security intelligence may update independently.

Track:

```text
engine version
ruleset version
database version
template version
download provenance
last update
last successful health check
```

Examples:

```text
Nuclei engine
Nuclei templates

Trivy engine
Trivy vulnerability DB
```

A scanner may be:

```text
AVAILABLE
```

while its intelligence feed is:

```text
STALE_DATA
```

Doc 05 determines how that affects verification.

---

## H75. Model/Provider Metadata Updates

Provider catalogs change faster than AgentCode core.

Do not hardcode provider/model examples from Doc 01 into static architecture.

OmniRoute/provider adapters should permit:

```text
catalog refresh
provider adapter update
model alias/deprecation update
```

without redesigning Kernel or UI.

---

## H76. Language Adapter Updates

Adding a new language should normally require:

```text
grammar/parser adapter
symbol normalization
LSP adapter if available
test intelligence
build-system detection
```

not rewriting repository graph architecture.

Tree-sitter grammar version must be part of index/cache provenance.

---

## H77. Extraction Versioning

Every campaign output has:

```text
campaign version
donor SHAs
core-doc version refs
review status
created date
supersedes
```

Re-extraction creates a new version rather than silently editing history.

Adoption decision may point to:

```text
EXTRACTION-v2
```

while preserving why v1 was superseded.

---

## H78. Cross-Document Traceability

Every implementation-ready extraction should link:

```text
Architecture requirement/section
→ ExtractionRecord
→ AdoptionDecision
→ ImplementationPacket
→ Doc 09 phase
→ Doc 11 work package
→ Doc 10 gate
→ AgentCode test/evidence
```

Conceptual chain:

```text
DOC 04:
Transactional ChangeSet required
        ↓
EXT-EDIT-...
        ↓
ADOPT-EDIT-...
        ↓
PKT-EDIT-...
        ↓
P13-WP...
        ↓
P13-G...
        ↓
test/evidence
```

This makes donor influence auditable without letting donor source become architecture truth.

---

## H79. Extraction Review Checklist

Independent reviewer asks:

### Evidence

```text
Is donor SHA pinned?
Are exact files/symbols identified?
Was core behavior traced?
Were upstream tests inspected?
Are failure paths included?
```

### Architecture

```text
Does recommendation obey Docs 01–06?
Does it create duplicate authority?
Does it silently expand V1 scope?
```

### Reuse

```text
Is TAKE/ADAPT/WRAP correct?
Could a stable wrapper reduce coupling?
Is source copying truly beneficial?
```

### License

```text
Was actual reused file/tool reviewed?
Are assets/pro directories separate?
Is redistribution decision supported?
```

### Performance

```text
Does it fit 8 GB target?
Was benchmark needed but skipped?
```

### Security

```text
Does dependency introduce network/secrets/sandbox risk?
Is install/update provenance safe?
```

---

## H80. Extraction Completion Matrix

A campaign is complete only when every required capability has one of:

```text
IMPLEMENTATION_READY
REJECTED_WITH_ALTERNATIVE
BLOCKED_WITH_EXPLICIT_OWNER
INDEPENDENT_IMPLEMENTATION_APPROVED
```

Not:

```text
“we looked at the repo.”
```

Campaign summary table:

| Capability | Primary extraction | Depth | License | Adoption | Prototype | Benchmark | Packet | Status |
|---|---|---:|---|---|---|---|---|---|

---

## H81. Phase 1 Overall Exit Gate

The OSS extraction phase may be declared complete only when:

1. `AGENTCODE_REFERENCE_CATALOG.md` exists.
2. Machine-readable catalog exists or generation strategy is implemented.
3. P0/P1 donors have pinned SHA metadata for the extraction performed.
4. The `skills` naming collision is resolved or explicitly cataloged.
5. Selected production dependencies have license/provenance status.
6. The 14 canonical campaign reports exist.
7. P0 foundational mechanisms reach required depth.
8. Every planned production dependency has a dependency-admission decision.
9. Every source reuse/fork has a license review.
10. OmniRoute fork base and patch strategy are pinned.
11. Required prototypes are complete.
12. Required benchmark decisions are complete.
13. Implementation packets exist for roadmap Phase 2+ work.
14. No architecture conflict remains silently unresolved.
15. No shipped V1 dependency remains `UNKNOWN`/`INCOMPATIBLE` licensing.
16. Third-party manifest/notices structure exists.
17. No implementation phase requires broad re-reading of the donor library.
18. New repository hunting remains closed unless a concrete capability gap appears.

---

## H82. What Doc 07 Does Not Own

Doc 07 does **not** define:

```text
provider scoring
quota algorithms
Kernel state machine
scheduler
context relevance scoring
memory freshness semantics
sandbox permission semantics
ChangeSet transactions
verification finding semantics
red-team safety policy
Design Studio UX
mission UI
```

Those are Docs 01–06.

Doc 07 answers:

```text
Which proven source should inform/build this?
How deeply did we inspect it?
What exactly did upstream do?
What will AgentCode reuse?
How?
Under what license/provenance?
With what tests/benchmarks?
What must not be inherited?
```

It also does not claim that the source-level dossier is complete merely because this control specification is complete.

---

## H83. Hardening Revision Summary

This revision corrects the major weakness identified in the documentation audit.

The old Doc 07 was a strong OSS strategy but still required future agents to invent:

```text
record schemas
source freshness
license review structure
campaign exit states
adoption-decision format
implementation packet format
benchmark manifests
review levels
canonical campaign numbering
catalog authority
```

Hardening Revision 2 adds:

1. explicit architecture/extraction/license status classes;
2. control-specification vs dossier distinction;
3. canonical `AGENTCODE_REFERENCE_CATALOG.md`;
4. stable repository identity and metadata;
5. exact source-pinning/freshness rules;
6. complete ExtractionRecord schema;
7. extraction lifecycle;
8. AdoptionDecision schema;
9. reuse-form taxonomy;
10. LicenseReviewRecord schema;
11. third-party provenance model;
12. fork record and divergence policy;
13. dependency admission;
14. external binary provenance;
15. installation mode policy;
16. runtime CapabilityRegistry;
17. versioned extension boundaries;
18. formal one-authority invariants;
19. comprehensive known donor catalog;
20. prior-observation/reverification handling;
21. locked donor-specific take/reject lessons;
22. L0–L5 extraction depth;
23. source tracing method;
24. campaign packet;
25. extraction security and clone contamination policy;
26. PrototypeRecord;
27. BenchmarkRecord;
28. benchmark corpus;
29. stable campaign numbering;
30. detailed campaign playbooks 00–14;
31. campaign report template;
32. implementation packet;
33. six representative full extraction examples;
34. build-vs-borrow matrix;
35. upstream-test/provenance rules;
36. donor retirement policy;
37. stop-collecting rule;
38. constrained implementation language/IPC boundaries;
39. platform compatibility;
40. 8 GB Mac extraction gate;
41. supply-chain intake;
42. security ruleset/update separation;
43. extraction versioning;
44. cross-document traceability;
45. independent review;
46. campaign and Phase 1 completion gates.

The companion extraction reports remain mandatory for source-level proof.

---

# 188. Implementation Stage 1 — Core Repository Skeleton

Create AgentCode repository structure.

Example:

```text
AgentCode/
│
├── apps/
│   └── desktop/
│
├── crates/ or core/
│   ├── kernel/
│   ├── tools/
│   ├── git/
│   └── intelligence/
│
├── services/
│   ├── provider/
│   └── adapters/
│
├── skills/
│
├── docs/
│
└── tests/
```

Exact language structure is implementation-specific.

---

# 189. Stage 1 Goal

Produce:

```text
daemon starts

UI connects

SQLite initialized

mission can be created

event logged

clean shutdown/restart
```

No advanced agent yet.

---

# 190. Stage 2 — Provider Fabric

Integrate customized OmniRoute fork.

Prove:

```text
candidate discovery

health

fallback

free-first route

paid fallback policy

local Ollama
```

---

# 191. Stage 3 — Basic Agent Runtime

Using insights from:

```text
mini-SWE-agent

Codex

OpenHands
```

implement:

```text
single Worker

model call

tool loop

task state

checkpoint
```

---

# 192. Stage 4 — Tool Engine

Implement native:

```text
read

search

edit

shell

Git

test
```

plus role/policy enforcement.

---

# 193. Stage 5 — Worktree Isolation

Every implementation task operates through:

```text
isolated worktree
```

with checkpoint evidence.

---

# 194. Stage 6 — Code Intelligence V1

Integrate:

```text
ripgrep

Tree-sitter

repo map

basic graph

initial LSP adapters
```

---

# 195. Stage 7 — Context Engine

Implement:

```text
targeted context packs

token budgets

CONTEXT.md

compaction

handoff
```

RTK integration begins here.

---

# 196. Stage 8 — Durable Kernel

Expand:

```text
Planner

task DAG

leases

heartbeats

recovery

parallel workers

Verifier
```

---

# 197. Stage 9 — Advanced Editing

Implement:

```text
multi-file change sets

transactional rollback

structured patches

AST transforms

LSP rename
```

---

# 198. Stage 10 — Browser / UI Verification

Integrate Playwright.

Prove:

```text
start app

open browser

interact

screenshot

console/network capture
```

---

# 199. Stage 11 — Verification Engine

Implement:

```text
requirement tracing

evidence

independent verification

incremental verification

final audit
```

---

# 200. Stage 12 — Security Engine

Integrate first:

```text
Gitleaks

OSV

Trivy

Semgrep

Checkov
```

then:

```text
ZAP

Nuclei

Prowler
```

---

# 201. Stage 13 — Adversarial Security

Build:

```text
attack graph

validation workflow

isolated lab

Stratus/CloudGoat integration

AI red-team adapters
```

---

# 202. Stage 14 — Design Studio

Integrate concepts from:

```text
Onlook

Dyad

Bolt
```

with existing:

```text
Browser Engine

Edit Engine

Visual QA

Context Engine
```

---

# 203. Stage 15 — Minimal Desktop Product

Build polished:

```text
Goal

Discuss

Design

Security

Changes

Activity

Notifications
```

---

# 204. Stage 16 — Optimization

Measure:

```text
tokens

RAM

latency

retries

tool overhead

index cost

provider cost
```

Optimize based on evidence.

---

# 205. Stage 17 — Chaos Validation

Deliberately:

```text
kill Worker

kill Planner

kill UI

kill daemon

trigger provider 429

break LSP

force context exhaustion

produce false done claim
```

AgentCode must recover.

---

# 206. Stage 18 — Production Readiness

Review:

```text
security

licensing

crash handling

updater

packaging

logs

privacy

resource management

documentation
```

---

# 207. Extraction Before Stage Rule

A subsystem implementation phase should not begin until its corresponding extraction report reaches sufficient depth.

Example:

```text
Code Intelligence implementation
```

requires:

```text
EXTRACTION_02_CODE_INTELLIGENCE
```

approved.

---

# 208. Avoid Analysis Paralysis

Extraction should answer implementation questions.

It must not become endless repo research.

Each campaign should end when:

```text
strong mechanism identified

integration path known

licensing understood

prototype sufficient
```

---

# 209. Timebox Extraction

Campaigns should be timeboxed by subsystem complexity.

Foundational source tracing can be deep.

Secondary references should receive limited attention.

---

# 210. Extraction Priority Rule

When multiple repos implement same capability:

```text
inspect strongest 2–3 deeply
```

rather than:

```text
inspect 12 superficially.
```

---

# 211. Example — Editing

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
```

No need to understand every editor implementation equally.

---

# 212. Example — Memory

Deep:

```text
Letta Code

Gemini CLI
```

Supporting:

```text
Graphiti

Cline
```

---

# 213. Example — Agent Loop

Deep:

```text
mini-SWE-agent

Codex

OpenHands
```

Supporting:

```text
LangGraph

Microsoft Agent Framework
```

---

# 214. No Model Transcript Dependency

Extraction agents must produce durable artifacts.

Do not rely on:

```text
"the research agent remembers what it saw."
```

Every conclusion enters extraction docs.

---

# 215. Source Pointers

Extraction output must reference:

```text
repo

path

symbol/class/function

commit
```

for every major mechanism.

---

# 216. Extraction Confidence

Each recommendation may receive:

```text
HIGH

MEDIUM

LOW
```

confidence.

Low-confidence mechanisms require prototype or deeper tracing before adoption.

---

# 217. Extraction Conflicts

If two reference projects demonstrate conflicting patterns:

```text
record both

compare tradeoffs

decide according to AgentCode architecture
```

Do not silently choose based on popularity.

---

# 218. ADR Integration

Major implementation choices produce:

```text
Architecture Decision Records
```

Examples:

```text
ADR-001 Kernel language

ADR-002 local IPC mechanism

ADR-003 worktree layout

ADR-004 LSP manager

ADR-005 scanner installation policy
```

---

# 219. Licensing ADRs

Special cases should produce decisions.

Examples:

```text
CodeQL distribution

Trail of Bits skill reuse

Dyad pro directory exclusion

Bolt/WebContainer boundaries

Munder artwork exclusion
```

---

# 220. Asset Policy

Do not reuse:

```text
logos

icons

artwork

fonts

marketing graphics
```

from reference projects unless independently licensed and explicitly selected.

AgentCode must have original brand assets.

---

# 221. Munder Artwork

Even where source code is permissive, artwork/assets with different licensing must remain excluded unless separately licensed.

This reinforces:

```text
license at file/asset level,
not only repository level.
```

---

# 222. User Data Isolation

Reference analytics/telemetry systems should not automatically be copied.

AgentCode should default toward:

```text
local-first

minimal telemetry

explicit privacy policy
```

Final telemetry decision belongs to PRD/master roadmap.

---

# 223. Hosted Dependencies

Avoid architecture that requires:

```text
vendor cloud

hosted proprietary backend

paid remote sandbox
```

for basic AgentCode operation.

The product goal remains local-first.

---

# 224. External Services

AgentCode may optionally integrate external services.

They must degrade gracefully.

Example:

```text
GitHub unavailable
```

should not prevent:

```text
local repository coding.
```

---

# 225. Build Reproducibility

AgentCode builds should use pinned dependencies and lockfiles.

External binaries should be pinned by version/checksum.

---

# 226. Cross-Platform Packaging

V1 should prioritize:

```text
macOS Apple Silicon
```

while avoiding architecture that makes Windows support impossible.

External tool adapters must record platform availability.

---

# 227. Unsupported Tool Fallback

If tool unavailable on platform:

```text
capability registry marks unavailable
```

and Kernel selects fallback.

Example:

```text
Zoekt unavailable
→ ripgrep.
```

---

# 228. Security Tool Isolation

Security tools may process malicious content.

Run them with appropriately restrictive sandbox.

Their output should be considered untrusted data.

---

# 229. Scanner Output Prompt Injection

Security scanner output may include attacker-controlled strings.

AgentCode must never treat scanner result text as system instructions.

Same trust-boundary principle from Doc 02 applies.

---

# 230. OSS Documentation as Untrusted Content

When an extraction agent reads source repository text:

```text
README

comments

issues
```

these remain reference data.

They cannot override AgentCode architecture or tool policy.

---

# 231. AgentCode Core Testing Strategy

Each adapted mechanism needs tests at three levels:

```text
unit

integration

chaos/regression
```

---

# 232. Example — Provider Routing Tests

```text
quota exhausted

provider down

model missing

paid fallback denied

diversity constraint
```

---

# 233. Example — Code Intelligence Tests

```text
symbol update

branch switch

worktree overlay

LSP failure

stale knowledge

incremental index
```

---

# 234. Example — Editing Tests

```text
human concurrent edit

half-applied patch

multi-file rollback

rename

formatter interference
```

---

# 235. Example — Security Tests

```text
known vulnerable fixtures

false positive

attack chain

secret redaction

scope enforcement
```

---

# 236. Upstream Test Reuse

When license permits, upstream test cases may inspire AgentCode regression cases.

Prefer independently authored tests around AgentCode interfaces.

---

# 237. AgentCode Self-Benchmark

AgentCode should eventually use itself to improve AgentCode.

However:

```text
self-editing
```

must operate through normal worktrees, verification and review.

No privileged bypass.

---

# 238. Dogfooding

After basic V1:

```text
AgentCode
```

should be used to:

```text
fix AgentCode bugs

write AgentCode tests

audit AgentCode security

redesign AgentCode UI
```

This provides powerful validation.

---

# 239. AgentCode Security Dogfood

Use:

```text
Promptfoo

Garak

PyRIT

Semgrep

Trivy

OpenHack patterns
```

to attack AgentCode itself.

High-value targets:

```text
tool policy

prompt injection

MCP

skills

secrets

sandbox

provider trust
```

---

# 240. Extraction Completion Gates

A repository campaign is complete when:

```text
exact mechanisms identified

source paths recorded

license reviewed

classification assigned

integration boundary defined

prototype completed where needed

known risks documented
```

---

# 241. Implementation Readiness Gate

Subsystem may begin implementation when:

```text
architecture locked

extraction complete

interfaces defined

primary dependencies selected

license gate passed

acceptance tests drafted
```

---

# 242. Dependency Admission Gate

A new third-party dependency enters AgentCode only if:

```text
needed

maintained enough

license acceptable

security acceptable

resource cost acceptable

clear owner/interface
```

---

# 243. Dependency Removal

If a dependency becomes:

```text
unmaintained

vulnerable

license-incompatible

too heavy

superseded
```

stable internal interfaces should allow replacement.

---

# 244. Upstream Fork Policy

Fork upstream only when AgentCode needs sustained code modification.

Good example:

```text
OmniRoute.
```

Usually avoid forks for:

```text
ripgrep

Tree-sitter

Trivy

Playwright
```

unless absolutely required.

---

# 245. Fork Maintenance Burden

Every fork creates:

```text
security update burden

merge burden

release burden
```

Therefore forks need explicit justification.

---

# 246. Patch-Stack Strategy

For necessary small modifications to external projects:

prefer:

```text
small maintained patch set
```

over large divergence.

---

# 247. Build-Time vs Runtime Dependencies

Record whether each dependency is:

```text
BUILD

RUNTIME

OPTIONAL_RUNTIME

DEVELOPMENT_REFERENCE
```

The reference library itself is:

```text
DEVELOPMENT_REFERENCE.
```

---

# 248. Reference Repository Updates

Because the local clones are shallow:

```text
git fetch
```

may later update them.

Do not update automatically during an extraction campaign without recording the new commit.

---

# 249. Extraction Reproducibility

Each campaign begins with:

```text
git rev-parse HEAD
```

for every source repository.

Write SHAs into report.

---

# 250. OSS Local Search

Create tooling that can search across reference repositories without feeding everything to an LLM.

Possible:

```text
ripgrep

Zoekt

local symbol indexes
```

This mirrors AgentCode's own context-efficiency philosophy.

---

# 251. Reference Intelligence Index

AgentCode development may create a separate index:

```text
.reference-intelligence/
```

covering all OSS sources.

Do not confuse it with runtime project intelligence.

---

# 252. Extraction Context Efficiency

Research agent receives:

```text
specific question

relevant repos

relevant search results

target files
```

not 50 complete repositories.

---

# 253. Example Extraction Question

Bad:

```text
Study Codex.
```

Good:

```text
Trace how Codex authorizes and executes shell commands,
including policy checks, sandbox selection, process spawning,
error representation and approval flow.
```

---

# 254. OSS Feature Database

Future helpful structure:

```text
oss_features
```

fields:

```text
feature

source_repo

source_path

classification

AgentCode_subsystem

status

license

notes
```

---

# 255. Prevent Duplicate Extraction

Before researching a mechanism, search extraction DB.

Do not repeatedly spend tokens rediscovering:

```text
how Aider repo map works.
```

---

# 256. Extraction Agent Handoff

Research agents use Doc 02 handoff mechanisms.

Long repo investigations should survive model/provider changes.

---

# 257. Strong Models for Extraction

High-complexity extraction should route to strong models when needed.

Routine code-navigation can use cheaper models.

Doc 01 controls this.

---

# 258. No Paid Model for Mechanical Inspection

Tasks like:

```text
find class

list files

locate function
```

should use deterministic tools/local helpers rather than paid reasoning.

---

# 259. Model Escalation During Extraction

Use stronger model when:

```text
architecture ambiguous

multiple systems conflict

license implication unclear

complex concurrency behavior
```

---

# 260. Implementation Agent Restrictions

Implementation agents should not be allowed to silently add new large frameworks.

Any foundational dependency not listed in approved architecture requires:

```text
ADR
+
dependency admission gate.
```

---

# 261. No Surprise Architecture

An agent may not decide:

```text
"Let's replace SQLite with Redis."
```

without explicit architecture decision.

---

# 262. No Surprise Framework Replacement

Likewise:

```text
Tauri → Electron

Rust Kernel → LangGraph

OmniRoute → LiteLLM
```

requires explicit reevaluation.

---

# 263. Architecture Flexibility

Locked does not mean immutable forever.

A component may change if empirical evidence shows:

```text
major flaw

platform incompatibility

unacceptable performance

license issue

maintenance risk
```

Document the change.

---

# 264. Selection by Evidence

Framework reputation is not enough.

AgentCode chooses based on:

```text
measured fit
```

to our architecture.

---

# 265. Build vs Borrow Decision

For every subsystem ask:

```text
Is there already a mature primitive?

Can it be isolated behind our interface?

Would building it produce strategic differentiation?
```

If mature primitive exists and differentiation is low:

```text
borrow/wrap.
```

---

# 266. AgentCode Differentiation

AgentCode should spend engineering effort on:

```text
durable autonomy

multi-provider intelligence

deep repository context

cross-model continuity

verification

all-round engineering workflows

Design Studio

integrated security

minimal product UX
```

not reimplementing grep or static scanners.

---

# 267. Core Unique Systems

These are primarily AgentCode-owned:

```text
Autonomy Kernel

Model Broker overlay

Requirement Matrix

Context Pack Builder

Evidence-linked Knowledge Store

Cross-agent Handoff

Role-specific Context

Transactional Change Sets

Completion Gate

Attack-Chain Reasoner

Design Director / Anti-Slop pipeline

Minimal Goal-mode UX
```

---

# 268. Wrapper-Owned Systems

AgentCode owns wrappers for:

```text
Tree-sitter

LSP

SCIP

Zoekt

Playwright

Semgrep

Trivy

Prowler

Nuclei

etc.
```

---

# 269. Reference-Only Systems

Some ideas may never ship as dependencies.

That is acceptable.

The purpose of the local library is:

```text
avoid reinventing known solutions
```

not:

```text
maximize number of dependencies.
```

---

# 270. Extraction Review

Every campaign report should be reviewed by another model/agent.

Verifier asks:

```text
Did researcher actually inspect code?

Did it miss better existing mechanisms?

Does recommendation violate architecture?

Did it misread licensing?

Is proposed reuse actually necessary?
```

---

# 271. Implementation Review

When AgentCode implements an extracted mechanism, compare:

```text
upstream behavior

AgentCode behavior

architecture requirement
```

Tests should prove intended behavior.

---

# 272. Extraction Traceability

Every major AgentCode component should eventually answer:

```text
Which architecture document required this?

Which OSS sources informed it?

Which implementation owns it?

Which tests prove it?
```

---

# 273. Documentation Cross-Linking

Implementation docs should reference:

```text
Doc 02 section

Doc 03 section

Doc 04 section
```

rather than duplicate architecture.

---

# 274. Code Comments

Do not clutter source with:

```text
"Copied from X"
```

unless legally required or useful.

Third-party notices/license comments follow actual legal obligations.

---

# 275. Attribution Accuracy

Do not claim code is independently written if substantial source was copied.

Likewise do not attribute generic architectural ideas unnecessarily.

---

# 276. AgentCode Repository Metadata

Recommended top-level:

```text
LICENSE

THIRD_PARTY_NOTICES.md

SECURITY.md

CONTRIBUTING.md

ARCHITECTURE.md
```

when project reaches appropriate maturity.

---

# 277. Security Policy

`SECURITY.md` should eventually explain:

```text
responsible disclosure

supported versions

reporting process
```

before public release.

---

# 278. Dependency SBOM

AgentCode release process should generate an SBOM.

Useful formats:

```text
CycloneDX

SPDX
```

This helps monitor third-party risk.

---

# 279. License Automation

CI should eventually run:

```text
dependency license check

forbidden license check

third-party notice validation
```

---

# 280. OSS Vulnerability Monitoring

AgentCode's own dependencies should be checked using:

```text
OSV

Trivy
```

or equivalent.

The system should use its own security architecture on itself.

---

# 281. Upstream Security Advisories

Forked projects such as OmniRoute require monitoring.

Selective upstream security patches should receive high priority.

---

# 282. Optional Component Updates

Tools may have independent channels.

Example:

```text
AgentCode 1.0

Trivy adapter
Trivy engine X

Nuclei templates Y

provider catalog Z
```

AgentCode Core need not release each time a data/rule set updates.

---

# 283. Extraction Exit Principle

Once implementation decisions are sufficiently grounded:

```text
stop researching.
```

The project must move forward.

---

# 284. Reference Hunting Is Now Closed

The current repository library is considered sufficient for V1.

Adding another framework requires a specific unresolved capability gap.

No further broad:

```text
"find more agent repos"
```

campaign is required.

---

# 285. Missing Capability Procedure

If implementation discovers missing capability:

```text
define precise gap
        ↓
search current library
        ↓
if unsolved:
research one or two new sources
        ↓
dependency admission gate
```

Do not resume indiscriminate repo collection.

---

# 286. Implementation Handoff Package

Before primary coding begins, the implementation agent should receive:

```text
Docs 01–07

PRD

Master Roadmap

Success Definition

Phase Playbook

AgentCode Reference Catalog

Extraction Reports

License Matrix

current AgentCode repo
```

---

# 287. Implementation Agent Must Not Read All References Initially

For a task:

```text
build Context Pack Builder
```

it receives:

```text
Doc 02

relevant extraction report

relevant source pointers

AgentCode code
```

not all reference repositories.

---

# 288. Source Retrieval on Demand

The agent may query:

```text
Aider exact repo-map file

Letta exact compaction module
```

when needed.

This dramatically reduces context waste.

---

# 289. Token-Efficient Implementation Philosophy

The same rule applies to building AgentCode as running AgentCode:

```text
relevant source
>
maximum source.
```

---

# 290. Example Implementation Context

Task:

```text
Implement transactional multi-file change set.
```

Context:

```text
Doc 04 relevant sections

AgentCode Edit Engine current source

extraction report

Codex/Aider/Cline exact relevant modules

tests

task acceptance criteria
```

Not:

```text
all six repositories.
```

---

# 291. Phase Acceptance

Each implementation stage must satisfy Doc 10 Success Definition.

The roadmap cannot mark:

```text
Integrated Tree-sitter
```

complete because:

```text
dependency added.
```

It needs functioning AgentCode behavior.

---

# 292. Evidence Required for OSS Adoption

For major integrations retain:

```text
prototype

benchmark

tests

license record

source references
```

---

# 293. Example — RTK Adoption Gate

Before locking RTK integration:

```text
run real AgentCode command corpus

compare raw tokens

compare compressed tokens

measure lost debugging information

compare Caveman

verify raw retrieval
```

---

# 294. Example — Zoekt Gate

Measure:

```text
repo size threshold

index cost

memory

search latency

ripgrep comparison
```

Only enable where benefit justifies cost.

---

# 295. Example — SCIP Gate

Measure:

```text
language coverage

indexing complexity

quality vs LSP

runtime cost
```

SCIP remains optional if value is small.

---

# 296. Example — Graphiti Gate

Do not add Graphiti simply because temporal graphs are attractive.

Add only if:

```text
SQLite knowledge model
```

fails measurable requirements.

---

# 297. Example — Browser Use Gate

If deterministic Playwright covers task:

```text
do not invoke LLM browser agent.
```

Use Browser Use only where:

```text
interface unknown/dynamic

semantic exploration required.
```

---

# 298. Example — Security Scanner Gate

Do not run:

```text
Semgrep
CodeQL
Trivy
Checkov
ZAP
Nuclei
```

all for a README change.

Tool selection is risk- and scope-aware.

---

# 299. Extraction Quality Metrics

Track:

```text
repositories inspected

exact source paths identified

mechanisms adopted

mechanisms rejected

copied LOC

wrapped components

license blockers

implementation reuse savings
```

---

# 300. Goal of OSS Reuse

The metric is not:

```text
how much code did we copy?
```

The metric is:

```text
how much proven engineering did we avoid needlessly reinventing?
```

---

# 301. Minimal Fork Footprint

AgentCode should aim for:

```text
few maintained forks
```

with OmniRoute being the major known exception.

Most dependencies should remain upstream-managed.

---

# 302. Unified AgentCode Experience

Regardless of underlying source:

```text
Aider

Playwright

Trivy

Prowler

RTK
```

the user should experience:

```text
AgentCode.
```

No fragmented CLI choreography.

---

# 303. Adapter Error Normalization

Every adapter maps tool-specific errors into AgentCode categories.

Example:

```text
BINARY_MISSING

UNSUPPORTED_PROJECT

AUTH_REQUIRED

TIMEOUT

SCAN_FAILED

VERSION_MISMATCH
```

---

# 304. Adapter Logging

Record:

```text
tool

version

arguments summary

duration

status

artifact refs
```

without leaking secrets.

---

# 305. Adapter Tests

Each external tool adapter requires:

```text
availability test

successful run test

failure test

output parsing test

version mismatch test
```

---

# 306. Compatibility Matrix

Maintain:

```text
macOS ARM64

macOS x64 later

Windows later
```

for every bundled/managed tool.

---

# 307. Native vs WASM

Where mature tools provide WASM and native versions, benchmark.

Do not choose WASM only for packaging convenience if performance becomes unacceptable.

---

# 308. AgentCode Update Architecture

Core updates and tool updates should remain separable where possible.

This reduces release coupling.

---

# 309. Extraction Security

Reference repositories themselves are untrusted code.

Do not:

```text
npm install
run test
execute script
```

in every cloned repo automatically.

Inspect first.

Run in sandbox when required.

---

# 310. Reference Repo Execution

If extraction requires running a reference project:

```text
create isolated environment

avoid real credentials

avoid broad network

record commands
```

---

# 311. License Review Timing

License review occurs:

```text
before copying code
```

not after implementation.

---

# 312. Unknown License

If license status is unclear:

```text
DO NOT COPY.
```

Concept may be independently implemented after appropriate review.

---

# 313. Public Release Gate

Before AgentCode becomes publicly distributed:

```text
full dependency/license audit

third-party notices

asset audit

SBOM

security audit
```

are mandatory.

---

# 314. Private Development

Even during private development, maintain licensing hygiene from the start.

This avoids expensive cleanup later.

---

# 315. OSS Contribution Opportunity

If AgentCode finds a generic upstream bug while integrating a dependency:

```text
prefer contributing fix upstream
```

when practical rather than maintaining permanent private patch.

---

# 316. Strategic Fork Criteria

Fork only if:

```text
AgentCode fundamentally changes behavior

upstream interface unsuitable

feature cannot live in adapter

upstream unlikely to accept change
```

---

# 317. OmniRoute Meets Fork Criteria

AgentCode's routing additions significantly alter selection policy.

Therefore the fork is justified.

---

# 318. Scanner Forks Usually Do Not

AgentCode needs scanners to:

```text
scan.
```

Our differentiation occurs after scanning.

Therefore adapters are preferable.

---

# 319. Architecture Completion Criterion

Doc 07 succeeds if an implementation team can answer for every major subsystem:

```text
What should we build?

What should we reuse?

What should we wrap?

What should we study?

Which repo should we inspect?

What licensing gate applies?

What should never be inherited?
```

---


# HARDENING NOTE — COMPLETION INTERPRETATION

The original checklist below remains valid as a high-level checklist, but a checkbox may be marked complete only under the H81 Phase 1 exit rules.

In particular:

```text
“provider extraction complete”
```

means the canonical campaign report contains implementation-ready extraction/adoption records—not merely that the donor repositories were listed.

Likewise:

```text
“license matrix exists”
```

is insufficient if shipped dependencies remain unresolved.

The hardening definitions are authoritative where the original checklist was less specific.

---

# 320. OSS Extraction Completion Definition

The OSS extraction phase is complete when:

```text
✓ reference catalog exists

✓ repository commits are pinned

✓ license matrix exists

✓ foundational repos classified

✓ provider extraction complete

✓ code intelligence extraction complete

✓ context/memory extraction complete

✓ runtime extraction complete

✓ editing extraction complete

✓ Git/worktree extraction complete

✓ sandbox/permission extraction complete

✓ skills/hooks extraction complete

✓ browser extraction complete

✓ design extraction complete

✓ AppSec extraction complete

✓ cloud-security extraction complete

✓ AI-security extraction complete

✓ UX extraction complete

✓ foundational prototypes completed

✓ selected dependencies have integration boundaries

✓ no unresolved license blocker exists for V1 foundations

✓ THIRD_PARTY_NOTICES structure exists

✓ direct-copy decisions are documented

✓ wrappers selected for mature external tooling

✓ benchmark decisions recorded

✓ architecture conflicts resolved through ADRs

✓ implementation can begin without broad repository re-research
```

---

# 321. V1 AgentCode Implementation Completion Relationship

This document does not independently declare AgentCode V1 complete.

It defines:

```text
how implementation should be constructed.
```

Actual project completion is governed by:

```text
Doc 10 — Success Definition & Acceptance Gates
```

and:

```text
Doc 11 — Phase-Wise Implementation Playbook.
```

---

# 322. Locked Architectural Principles

The following are locked for V1:

1. AgentCode architecture is authoritative over reference frameworks.

2. OSS repositories are donors, not masters.

3. Every borrowed mechanism is classified.

4. AgentCode will not blindly combine complete frameworks.

5. Exact source paths must support major extraction decisions.

6. README-only analysis is insufficient for foundational mechanisms.

7. Foundational mechanisms require source-level inspection.

8. Strong candidate mechanisms should be prototyped before large integration.

9. AgentCode should reuse mature primitives rather than rebuild them.

10. AgentCode should own the orchestration layer around those primitives.

11. Stable internal AgentCode APIs isolate upstream dependencies.

12. External security tools are normally wrapped, not forked.

13. OmniRoute is intentionally forked and customized.

14. Aider provides repo-map and context ideas, not AgentCode runtime.

15. Letta provides memory ideas, not mission truth.

16. LangGraph provides durability ideas, not AgentCode Kernel authority.

17. Munder provides coordination patterns, not office UI or LLM authority.

18. Codex is a primary execution/sandbox/editing reference.

19. Gemini CLI is a primary hooks/checkpoint/context reference.

20. OpenHands is a primary tools/workspace/sandbox reference.

21. mini-SWE-agent is a primary simplicity/agent-loop reference.

22. Tree-sitter, ripgrep and Playwright should normally be used as proven primitives.

23. RTK-style deterministic output compression is a high-priority optimization.

24. Context extraction follows information density rather than repository size.

25. Security scanners remain specialized engines under one AgentCode security model.

26. Prowler is the primary broad cloud-posture reference/integration.

27. Stratus/Pacu/CloudGoat belong to authorized red-team/lab workflows.

28. Promptfoo/Garak/PyRIT support AI security.

29. Onlook/Dyad/Bolt inform Design Studio but do not define AgentCode UX.

30. AgentCode must have an original visual identity.

31. Asset licensing is separate from source-code licensing.

32. Unknown licensing blocks direct source reuse.

33. Third-party attribution must be maintained continuously.

34. Reference repositories should not become production submodules.

35. Most dependencies should remain upstream-managed.

36. Fork count should remain small.

37. Every fork creates explicit maintenance responsibility.

38. Tool versions must be pinned/traceable.

39. External binaries must have provenance/checksum controls when managed by AgentCode.

40. Optional heavy tools must not become requirements for normal coding.

41. AgentCode must run well on target hardware without all optional tools resident.

42. One mission runtime, one memory authority, one repository graph, one security finding model.

43. Duplicate frameworks must be normalized, not stacked.

44. Extraction is subsystem-specific.

45. Implementation agents receive targeted reference material.

46. AgentCode development must practice the same token-efficiency philosophy as AgentCode runtime.

47. New dependencies require an admission gate.

48. Broad repository hunting is considered complete for V1.

49. Future new repositories require a precise capability gap.

50. The purpose of OSS reuse is to accelerate quality, not maximize copied code.

---

# 323. Final Extraction Map

```text
                           AGENTCODE
                               │
                               ▼
                       CORE ARCHITECTURE
                          Docs 01–06
                               │
                               ▼
                       EXTRACTION ENGINE
                               │
     ┌─────────────────────────┼─────────────────────────┐
     ▼                         ▼                         ▼
 PROVIDER                 CODE/CONTEXT               RUNTIME
 OmniRoute                Aider                      Codex
 OpenCode                 Tree-sitter                mini-SWE
 Goose                    LSP/SCIP                   OpenHands
                          Letta                      Gemini
                          RTK                        SWE-ReX
     │                         │                         │
     └─────────────────────────┼─────────────────────────┘
                               ▼
                      AGENTCODE NORMALIZATION
                               │
      ┌────────────────────────┼────────────────────────┐
      ▼                        ▼                        ▼
   TOOLS/GIT                DESIGN                  SECURITY
 Codex/Cline               Onlook                 OpenHack
 OpenHands                 Dyad                   Semgrep
 Superpowers               Bolt                   Trivy
 Playwright                                       Prowler
      │                        │                        │
      └────────────────────────┼────────────────────────┘
                               ▼
                      STABLE AGENTCODE APIs
                               │
                               ▼
                         IMPLEMENTATION
                               │
                               ▼
                            TESTING
                               │
                               ▼
                       SUCCESS GATES
```

---

# 324. Core Reuse Decision Map

```text
PROBLEM:
Need exact search

SOLUTION:
ripgrep

AGENTCODE WORK:
adapter + result normalization
```

```text
PROBLEM:
Need structural parsing

SOLUTION:
Tree-sitter

AGENTCODE WORK:
symbol model + graph + incremental cache
```

```text
PROBLEM:
Need browser automation

SOLUTION:
Playwright

AGENTCODE WORK:
session manager + evidence + Kernel integration
```

```text
PROBLEM:
Need vulnerability scanning

SOLUTION:
mature scanners

AGENTCODE WORK:
orchestration + attack reasoning + verification
```

```text
PROBLEM:
Need provider routing

SOLUTION:
OmniRoute fork

AGENTCODE WORK:
significant custom routing extensions
```

```text
PROBLEM:
Need durable autonomous execution

SOLUTION:
AgentCode-owned Kernel

REFERENCE:
Codex
mini-SWE
OpenHands
LangGraph
Gemini
```

---

# 325. Final Statement

AgentCode should not win by writing more code than every existing coding agent.

It should win by **combining proven engineering more intelligently than existing systems**.

When mature projects already provide:

```text
fast search
parsing
browser automation
security scanning
cloud auditing
```

AgentCode should use them.

When mature agents demonstrate:

```text
excellent editing
checkpointing
worktrees
memory
hooks
sandboxing
```

AgentCode should study and adapt those mechanisms.

But when those systems disagree about:

```text
mission authority
completion
provider routing
memory truth
multi-agent coordination
```

AgentCode must follow its own architecture.

The central implementation principle is:

> **Borrow primitives. Own orchestration.**

The local reference library exists so AgentCode developers do not repeatedly solve already-solved problems.

The correct workflow is therefore:

```text
DEFINE SUBSYSTEM
      ↓
READ AGENTCODE SPEC
      ↓
SELECT RELEVANT REFERENCE REPOSITORIES
      ↓
LOCATE EXACT IMPLEMENTATIONS
      ↓
CLASSIFY TAKE / ADAPT / WRAP / STUDY / REJECT
      ↓
VERIFY LICENSE
      ↓
PROTOTYPE
      ↓
BENCHMARK
      ↓
NORMALIZE BEHIND AGENTCODE API
      ↓
TEST
      ↓
INTEGRATE
```

Not:

```text
read everything
      ↓
copy everything
      ↓
hope it works.
```

The intended result is:

> **A coherent AgentCode implementation built on a deliberately selected combination of proven open-source primitives, carefully adapted architecture patterns and AgentCode-owned control systems—minimizing duplicated engineering while preserving maintainability, performance, legal clarity, security and a single unified product architecture.**

This document is the **V1 source of truth for AgentCode's implementation and open-source extraction strategy.**

