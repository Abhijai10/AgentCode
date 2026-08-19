# AgentCode  
# 05 — Verification, Security & Red-Team Architecture

**Document Status:** V1 — Architecture Locked for Initial Implementation  
**Date:** 19 August 2026  
**Project:** AgentCode  
**Document Type:** Core Architecture Specification  

**Depends On:**
- `01 — Model, Provider, Routing & Reliability Architecture`
- `02 — Code Intelligence, Context & Persistent Memory Architecture`
- `03 — Autonomy Kernel & Agent Runtime Architecture`
- `04 — Tool, Edit, Git, Sandbox, Skills & Hooks Architecture`

**Primary Reference Repository Root:**  
`/Volumes/T7 Shield/GitHub-Repos-dependency`

**Scope:** Requirement verification, mechanical gates, independent model verification, adversarial review, test evidence, integration verification, incremental verification, evidence freshness, final mission audit, application security, dependency security, secret scanning, infrastructure-as-code security, supply-chain checks, web security, cloud posture assessment, cloud-security reasoning, authorized attacker simulation, attack-chain validation, isolated red-team environments, AI/LLM security testing, vulnerability confirmation, false-positive reduction, security remediation, regression testing, security reporting and mission-level security completion gates.

---

# 1. Purpose

AgentCode must not consider software correct merely because:

```text
the code compiles
```

or:

```text
tests passed
```

or:

```text
the Worker says it is done
```

or:

```text
another LLM says the implementation looks good.
```

A professional autonomous engineering system requires several independent forms of evidence.

AgentCode must answer:

> **Does the implementation actually satisfy the original goal, work correctly as an integrated system, and avoid leaving significant known security weaknesses behind?**

The Verification, Security and Red-Team subsystem exists to answer that question.

---

# 2. Fundamental Principle

The system should optimize for:

```text
PROVEN COMPLETION
```

rather than:

```text
PLAUSIBLE COMPLETION
```

Every important claim should eventually connect to evidence.

Example:

```text
Requirement:
Expired reset token must be rejected.

Implementation evidence:
PasswordService.verifyResetToken()

Test evidence:
expired_reset_token.test.ts

Runtime evidence:
HTTP 401

Verifier:
PASS

Status:
VERIFIED
```

This is significantly stronger than:

```text
Worker:
"I implemented reset token validation."
```

---

# 3. Relationship to Docs 01–04

Doc 01 determines:

```text
WHO performs verification?
```

Doc 02 determines:

```text
WHAT repository evidence they receive?
```

Doc 03 determines:

```text
WHEN verification is required
and whether completion is allowed.
```

Doc 04 determines:

```text
WHICH tools can actually execute
tests, browsers, scanners and security checks.
```

Doc 05 determines:

```text
WHAT constitutes proof
and
HOW aggressively AgentCode challenges its own work.
```

Combined:

```text
IMPLEMENTATION
      │
      ▼
MECHANICAL CHECKS
      │
      ▼
REQUIREMENT TRACE
      │
      ▼
INDEPENDENT VERIFIER
      │
      ▼
INTEGRATION CHECK
      │
      ▼
SECURITY CHECKS
      │
      ▼
ADVERSARIAL VALIDATION
      │
      ▼
FINAL AUDIT
      │
      ▼
KERNEL COMPLETION GATE
```

---

# 4. Core Objectives

This subsystem must:

1. prevent false completion;
2. verify each requirement explicitly;
3. prefer deterministic evidence over model opinion;
4. independently review implementation;
5. verify multi-file integration;
6. detect untested wiring;
7. invalidate stale evidence after related changes;
8. aggressively search for security weaknesses;
9. distinguish possible findings from confirmed vulnerabilities;
10. validate high-risk findings safely;
11. reason about attack chains rather than isolated scanner alerts;
12. produce remediation recommendations;
13. verify remediation;
14. generate useful human-readable and machine-readable reports;
15. prevent security tooling from becoming destructive by default;
16. allow deeper authorized red-team testing in isolated environments;
17. support cloud and AI security;
18. keep security evidence traceable to repository/runtime state.

---

# 5. Primary V1 Components

The subsystem consists of:

1. Verification Coordinator
2. Requirement Traceability Engine
3. Mechanical Gate Runner
4. Test Evidence Manager
5. Build/Lint/Typecheck Gate Manager
6. Runtime Verification Layer
7. Browser Verification Layer
8. Visual Verification Interface
9. Independent LLM Verifier
10. Adversarial Code Reviewer
11. Integration Verification Engine
12. Incremental Verification Engine
13. Evidence Freshness Manager
14. Final Audit Engine
15. Security Orchestrator
16. Threat Model Builder
17. Attack Surface Mapper
18. Static Analysis Adapter Layer
19. Secret Detection Adapter
20. Dependency Vulnerability Adapter
21. IaC Security Adapter
22. Supply Chain Security Adapter
23. Web Security Adapter
24. Cloud Security Adapter
25. AI Security Adapter
26. Vulnerability Triage Engine
27. False-Positive Reduction Engine
28. Security Finding Database
29. Attack Path Graph
30. Adversarial Validation Engine
31. Safe Exploit Validator
32. Red-Team Lab Manager
33. Cloud Red-Team Coordinator
34. Security Repair Planner
35. Security Regression Manager
36. Security Evidence Store
37. Security Report Generator
38. SARIF Exporter
39. Security Severity/Confidence Model
40. Human Escalation Interface

---

# 6. Verification Philosophy

Verification should be layered.

```text
Worker says done
      │
      ▼
Mechanical verification
      │
      ▼
Requirement verification
      │
      ▼
Independent verification
      │
      ▼
Integration verification
      │
      ▼
Security verification where relevant
      │
      ▼
Final audit
      │
      ▼
Kernel decides
```

No single layer is sufficient by itself.

---

# 7. Verification Levels

Suggested levels:

```text
V0 — Syntax / Structural

V1 — Local Mechanical

V2 — Task Functional

V3 — Requirement

V4 — Independent Review

V5 — Integration

V6 — Security

V7 — Final Mission Audit
```

Not every trivial task requires every level independently.

The mission-level final audit eventually reconciles them.

---

# 8. V0 — Structural Verification

Checks may include:

```text
file parses

syntax valid

configuration parses

schema valid

formatting structurally acceptable
```

This is the cheapest verification layer.

---

# 9. V1 — Mechanical Verification

Checks:

```text
formatter

lint

typecheck

compiler

build
```

where relevant.

These should be deterministic whenever possible.

---

# 10. V2 — Task Functional Verification

Verify the behavior directly associated with the task.

Examples:

```text
targeted unit tests

targeted integration test

browser flow

CLI command

API response

database operation
```

---

# 11. V3 — Requirement Verification

Map actual evidence to task/mission requirements.

Example:

```text
R17:
Guest cannot access admin dashboard.

Evidence:
- middleware implementation
- E2E unauthorized flow
- HTTP redirect behavior

Result:
PASS
```

---

# 12. V4 — Independent Review

An independent Verifier attempts to find:

```text
missing wiring

untested branches

incorrect assumptions

dead code

mock-only implementation

broken edge cases

unhandled errors

requirement gaps

hidden regressions
```

---

# 13. V5 — Integration Verification

Checks actual combined application state.

Examples:

```text
merged changes compile together

frontend/backend contract matches

database migration matches application

shared type changes propagated

test suites pass together

browser flows work after integration
```

---

# 14. V6 — Security Verification

Applicable tasks may undergo:

```text
SAST

dependency scanning

secret scanning

IaC scanning

web-security testing

cloud posture analysis

adversarial validation
```

---

# 15. V7 — Final Mission Audit

Final Audit compares:

```text
original goal
+
requirements
+
actual repository state
+
verification evidence
+
security state
```

It is the final evidence package before Doc 03's completion gate.

---

# 16. Requirement Matrix Integration

Doc 03 owns the Requirement Matrix.

Doc 05 populates verification fields.

Conceptually:

```text
requirement_id

implementation_state

verification_state

evidence

latest_validation_commit

latest_validation_time

blocking_findings
```

---

# 17. Requirement States

Suggested verification states:

```text
UNVERIFIED

PARTIALLY_VERIFIED

VERIFIED

FAILED

STALE
```

A requirement with:

```text
IMPLEMENTED
```

but:

```text
UNVERIFIED
```

cannot be treated as finished.

---

# 18. Verification Methods

Each requirement may define one or more methods:

```text
UNIT_TEST

INTEGRATION_TEST

E2E_TEST

BUILD_GATE

STATIC_ANALYSIS

BROWSER_ASSERTION

SCREENSHOT_REVIEW

SECURITY_SCAN

CLOUD_CHECK

MANUAL_EVIDENCE

LLM_REVIEW
```

---

# 19. Evidence Priority

Prefer:

```text
deterministic runtime proof
```

over:

```text
model interpretation.
```

Approximate evidence strength:

```text
reproducible runtime test
very high

compiler/typecheck
very high for represented property

security validation
high

browser execution
high

static analysis
medium/high

LLM review
supporting evidence

Worker claim
low
```

---

# 20. Test Evidence

Every meaningful test execution should persist:

```text
test_run_id

command

repository_view

commit

task

tests_selected

pass_count

fail_count

skip_count

duration

raw_output_ref

summary
```

---

# 21. Evidence Must Be Commit-Aware

A test result from commit:

```text
abc123
```

does not automatically prove the same property at:

```text
def456.
```

Evidence must track repository state.

---

# 22. Evidence Freshness

When related code changes:

```text
previous evidence
      ↓
impact analysis
      ↓
still valid?
```

Possible:

```text
FRESH

POSSIBLY_STALE

STALE

INVALID
```

---

# 23. Incremental Evidence Invalidation

Doc 02 impact analysis should determine which tests/checks may need rerunning.

Example:

```text
CSS-only change
```

does not require rerunning:

```text
database security tests
```

unless dependency evidence suggests otherwise.

---

# 24. Targeted Verification

After each edit iteration, prefer cheap targeted checks.

```text
parse
  ↓
lint affected file
  ↓
typecheck affected package
  ↓
related tests
```

Broader verification runs later.

This reduces runtime and model/tool token consumption.

---

# 25. Broad Verification

At task completion or integration boundaries run broader checks appropriate to the task.

Examples:

```text
full package test suite

application build

all affected integration tests

E2E critical flow
```

---

# 26. Final Broad Verification

Before mission completion, broad checks should cover enough of the repository to detect cross-task regressions.

The final scope depends on:

```text
repository size

mission scope

risk

available test systems

changed dependency graph
```

---

# 27. Independent Verifier

The independent Verifier should be designed to disprove completion.

Prompt orientation:

```text
Do not assume this implementation is correct.

Search for missing requirements,
incomplete wiring,
false-positive tests,
edge cases and regressions.
```

---

# 28. Verifier Independence

Where practical:

```text
Writer:
Qwen / ModelScope

Verifier:
GPT-OSS / Cerebras
```

or another provider/model-family combination.

Doc 01 controls routing.

---

# 29. Verifier Context

Verifier receives:

```text
original requirement

task acceptance criteria

actual diff

affected files

affected dependency graph

tests

runtime output

repository state
```

It should receive little or none of:

```text
Worker self-congratulation

long Worker reasoning

claims like "everything is complete"
```

This reduces anchoring.

---

# 30. Verification Findings

Conceptually:

```text
finding_id

task_id

requirement_id

severity

category

description

evidence

confidence

blocking

status
```

---

# 31. Verification Finding States

```text
OPEN

CONFIRMED

DISMISSED

FIXED

REVERIFY_REQUIRED

CLOSED
```

---

# 32. Adversarial Review Categories

Verifier should explicitly look for:

```text
missing implementation

missing UI wiring

missing backend wiring

dead feature flag

mock data

placeholder implementation

unhandled exception

incorrect type assumption

wrong environment configuration

migration gap

state synchronization bug

race condition

incorrect auth boundary

missing test

test that cannot fail

ignored error

silent fallback

unused code path
```

---

# 33. Test Quality Verification

A passing test is not automatically trustworthy.

Verifier should check for:

```text
assertion too weak

test only mocks target behavior

test never reaches changed code

fixture hides failure

assertion always true

test skipped

test disabled

test changed to match broken behavior
```

---

# 34. Test Tampering Detection

If a Worker "fixes" a failing task primarily by weakening tests:

```text
Kernel should treat that as suspicious.
```

Changes to existing tests require:

```text
justification

review

comparison against requirement
```

---

# 35. Coverage as Evidence

Coverage may help identify untested changed code.

Coverage percentage itself should not become a completion requirement unless specified.

Use it as:

```text
verification signal
```

rather than:

```text
proof of correctness.
```

---

# 36. Runtime Verification

AgentCode can verify application behavior through:

```text
local server

CLI invocation

API request

test database

browser

runtime logs
```

Runtime evidence is especially valuable for wiring and integration.

---

# 37. Browser Verification

Using Doc 04's browser engine:

```text
start app
   ↓
navigate
   ↓
perform real flow
   ↓
assert result
   ↓
inspect console
   ↓
inspect network errors
   ↓
capture screenshot
```

---

# 38. Visual Verification

Design-related tasks may additionally invoke:

```text
Gemma3 4B Visual QA
```

and future stronger visual models.

Visual verification checks:

```text
layout

overflow

clipping

missing content

responsive behavior

visual regressions
```

Detailed design standards belong to Doc 06.

---

# 39. Integration Verification

When several verified tasks are integrated:

```text
their individual verification evidence
```

does not guarantee:

```text
combined system correctness.
```

Run integration-specific gates.

---

# 40. Incremental Verification

AgentCode should not repeatedly spend expensive model calls verifying unchanged areas.

Architecture:

```text
previous verified baseline
         +
new diff
         +
impact scope
         ↓
incremental verification
```

At final mission completion:

```text
broader final audit still runs.
```

---

# 41. Verification Checkpoints

Persist:

```text
verified_commit

verified_requirements

verification_findings

test evidence

security evidence
```

This enables incremental verification later.

---

# 42. Security Mode

AgentCode Security is a first-class mode.

Logical skills:

```text
Application Security

Dependency Security

Secrets Security

Infrastructure Security

Cloud Security

Web Security

AI Security

Red-Team Validation
```

These remain skills/tools rather than permanent employee-like agents.

---

# 43. Security Pipeline

```text
REPOSITORY
    │
    ▼
ATTACK SURFACE DISCOVERY
    │
    ▼
THREAT MODEL
    │
    ├────────────────────────┐
    ▼                        ▼
STATIC ANALYSIS          CONFIG / CLOUD
    │                        │
    ├──────────────┬─────────┤
    ▼              ▼         ▼
DEPENDENCIES     SECRETS    IaC
    │              │         │
    └──────────────┴─────────┘
                   ▼
             FINDING TRIAGE
                   │
                   ▼
              AI REASONING
                   │
                   ▼
         ADVERSARIAL VALIDATION
                   │
              ┌────┴────┐
              ▼         ▼
           FALSE      CONFIRMED
          POSITIVE       │
              │           ▼
            CLOSE    REMEDIATION
                         │
                         ▼
                 REGRESSION TEST
```

---

# 44. Security Mode Levels

Recommended:

```text
LEVEL 1 — Passive Audit

LEVEL 2 — Active Validation

LEVEL 3 — Isolated Red-Team Lab
```

---

# 45. Level 1 — Passive Audit

Suitable for normal repositories and production code review.

Includes:

```text
SAST

dependency scanning

secret scanning

IaC scanning

cloud posture read-only assessment

threat modeling

manual AI code review
```

No active exploitation required.

---

# 46. Level 2 — Active Validation

Only against:

```text
authorized local

staging

test

explicitly approved targets.
```

May include:

```text
DAST

API abuse tests

authentication/authorization validation

fuzzing

controlled exploit confirmation

cloud read-only attack-path validation
```

---

# 47. Level 3 — Isolated Red-Team Lab

Used for the strongest "real attacker" simulation.

Environment should be:

```text
local clone

ephemeral container

temporary staging environment

dedicated test cloud account

intentionally vulnerable lab
```

AgentCode may attempt more realistic exploitation there while preserving hard safety boundaries.

---

# 48. Production Safety Boundary

By default Red-Team mode must not perform:

```text
destructive denial of service

persistent compromise

real credential theft

irreversible data deletion

real-user data exfiltration

unbounded scanning of unrelated targets

malware deployment

unauthorized lateral movement
```

A real vulnerability generally does not need destructive exploitation to prove it exists.

---

# 49. Authorization Boundary

Active Red-Team testing is allowed only against systems the user owns or has explicit permission to test.

AgentCode should require a target classification such as:

```text
LOCAL

TEST

STAGING

AUTHORIZED_CLOUD_LAB

PRODUCTION_READ_ONLY

PRODUCTION_ACTIVE_APPROVED
```

---

# 50. Threat Model Builder

Before deep scanning, build repository-specific threat context.

Identify:

```text
assets

trust boundaries

entry points

authentication

authorization

privileged operations

data stores

external integrations

cloud infrastructure

secrets

public endpoints

admin functionality
```

---

# 51. Threat Model Output

Example:

```text
Assets:
- user credentials
- subscription data
- admin operations

Entry Points:
- REST API
- web client
- webhook endpoints

Trust Boundaries:
- unauthenticated → API
- API → database
- CI → deployment

High-Risk Areas:
- session management
- payment webhook
- admin routes
```

---

# 52. Attack Surface Map

AgentCode should derive attack surfaces from Doc 02 repository intelligence.

Examples:

```text
HTTP routes

GraphQL mutations

RPC calls

CLI input

file uploads

webhooks

authentication flows

database access

cloud IAM

CI/CD workflows

MCP tools

LLM input
```

---

# 53. Security Finding Model

Conceptually:

```text
finding_id

title

category

severity

confidence

exploitability

affected_files

affected_symbols

attack_surface

evidence

scanner_sources

validation_state

attack_path

recommendation

patch_ref

regression_test_ref

status
```

---

# 54. Finding Categories

Possible categories include:

```text
AUTHENTICATION

AUTHORIZATION

INJECTION

XSS

CSRF

SSRF

DESERIALIZATION

CRYPTOGRAPHY

SESSION

SECRETS

DEPENDENCY

SUPPLY_CHAIN

FILE_HANDLING

RACE_CONDITION

BUSINESS_LOGIC

MISCONFIGURATION

CLOUD_IAM

NETWORK_EXPOSURE

CI_CD

AI_PROMPT_INJECTION

AI_TOOL_ABUSE

DATA_LEAKAGE
```

---

# 55. Severity

Suggested normalized levels:

```text
CRITICAL

HIGH

MEDIUM

LOW

INFO
```

Severity should reflect:

```text
impact
+
exploitability
+
exposure
```

not only scanner-provided labels.

---

# 56. Confidence

Suggested levels:

```text
CONFIRMED

HIGH

MEDIUM

LOW

UNVERIFIED
```

A scanner match without validation may remain:

```text
MEDIUM
```

or:

```text
UNVERIFIED
```

until AgentCode investigates.

---

# 57. Exploitability

Separate from severity.

Possible:

```text
PROVEN

LIKELY

CONDITIONAL

THEORETICAL

UNKNOWN
```

---

# 58. Static Analysis Layer

Reference tools already cloned:

```text
semgrep

codeql
```

AgentCode should integrate these through adapters rather than recreate entire engines.

---

# 59. Semgrep Integration

Use for:

```text
pattern-based SAST

custom repository rules

security patterns

framework checks
```

Results are evidence inputs.

They are not automatically confirmed vulnerabilities.

---

# 60. CodeQL Integration

Study/integrate where licensing/tooling permits for:

```text
data flow

taint flow

interprocedural security analysis

deep query-based analysis
```

Doc 07 must carefully specify licensing and CLI distribution boundaries.

---

# 61. Secret Scanning

Reference:

```text
Gitleaks
```

AgentCode can scan:

```text
working tree

Git history

configuration

commits
```

Secret values should be redacted in reports.

---

# 62. Secret Finding Handling

Report:

```text
secret type

file

commit if relevant

line location

exposure scope
```

Never repeat full secret value into:

```text
model prompt

security report

UI

logs
```

---

# 63. Dependency Security

References:

```text
OSV-Scanner

Trivy
```

Use for:

```text
known vulnerable packages

lockfile vulnerabilities

transitive dependencies

container/package CVEs
```

---

# 64. Dependency Finding Triage

AgentCode should distinguish:

```text
vulnerable package installed
```

from:

```text
vulnerable code path reachable.
```

When practical, analyze:

```text
actual package usage

affected version

attack surface

available fix
```

---

# 65. Supply Chain Security

Reference:

```text
OpenSSF Scorecard
```

Potential checks:

```text
dependency provenance

pinned CI dependencies

branch protection indicators

dangerous workflow patterns

repository hygiene
```

---

# 66. Infrastructure-as-Code Security

References:

```text
Checkov

Trivy
```

Targets:

```text
Terraform

CloudFormation

Kubernetes

Helm

Docker

GitHub Actions

other configuration
```

---

# 67. IaC Findings

Examples:

```text
public storage

wide IAM policy

unencrypted storage

open network service

privileged container

unrestricted security group

unsafe CI permission
```

---

# 68. Web Application Security

References:

```text
ZAP

Nuclei

Nuclei Templates
```

These should be run only against authorized targets.

---

# 69. ZAP Integration

Useful for:

```text
spidering

passive analysis

active testing

web application security checks
```

AgentCode should parse results into its normalized finding model.

---

# 70. Nuclei Integration

Useful for:

```text
template-driven vulnerability checks

configuration exposures

known patterns
```

Template provenance/version should be recorded.

---

# 71. Nuclei Template Trust

Downloaded or custom security templates may themselves execute network requests.

AgentCode should:

```text
pin template source/version

apply target scope

apply rate limit

apply network boundary
```

---

# 72. API Security Validation

AgentCode should be capable of reasoning about:

```text
authentication

authorization

object ownership

input validation

rate-limiting assumptions

schema enforcement

unexpected HTTP methods

sensitive response fields
```

---

# 73. Business Logic Security

Many serious issues are not scanner-detectable.

Examples:

```text
user can modify another user's resource

coupon reused indefinitely

subscription state bypass

admin action exposed through normal API

reset token remains usable after password change
```

LLM reasoning plus runtime validation is important here.

---

# 74. Security Reasoning Agent

Security analysis should operate as:

```text
Verifier
+
Security Skill
```

rather than a permanently running independent persona.

For very large audits the Kernel may allocate several security tasks.

---

# 75. Adversarial Security Review

Security Verifier should ask:

```text
How would an attacker enter?

What trust assumption can be broken?

What can be chained?

Where can authorization be bypassed?

Where is input treated as trusted?

What privileged operation can be reached?
```

---

# 76. False Positive Reduction

Scanner output can be noisy.

Pipeline:

```text
scanner finding
      ↓
code/context inspection
      ↓
reachability
      ↓
configuration
      ↓
runtime validation if appropriate
      ↓
CONFIRMED
or
DISMISSED
```

---

# 77. Validation States

```text
NEW

TRIAGED

VALIDATING

CONFIRMED

DISMISSED

NEEDS_MANUAL_REVIEW

FIXED

RETESTING

CLOSED
```

---

# 78. Security Evidence

Evidence may include:

```text
source range

data-flow trace

scanner result

HTTP response

runtime assertion

browser trace

cloud configuration

sandbox reproduction

test case
```

---

# 79. Safe Exploit Validation

For suspected high-severity findings:

```text
reproduce minimum necessary behavior
```

rather than maximizing damage.

Example concept:

```text
prove unauthorized read of synthetic test object
```

instead of:

```text
extract all real user data.
```

---

# 80. Synthetic Canary Data

Staging/red-team environments should contain synthetic markers.

Example:

```text
AGENTCODE_TEST_SECRET_48291
```

If adversarial validation retrieves it through an unintended path:

```text
data exposure confirmed.
```

This proves impact without accessing real sensitive data.

---

# 81. Attack Path Graph

Security findings should be composable.

Example:

```text
public endpoint
      ↓
predictable resource ID
      ↓
missing ownership check
      ↓
storage object accessible
      ↓
sensitive artifact exposure
```

Rather than report four unrelated alerts, AgentCode should identify:

```text
one attack chain.
```

---

# 82. Attack Path Representation

Conceptually:

```text
attack_path_id

entry_point

steps

prerequisites

privilege_required

affected_assets

impact

evidence

validation_state
```

---

# 83. Attack Chain Prioritization

A chain combining several medium findings may represent:

```text
HIGH
```

or:

```text
CRITICAL
```

real-world risk.

AgentCode should reason beyond scanner severity labels.

---

# 84. "Brutal" Red-Team Mode

AgentCode may expose an advanced authorized mode such as:

```text
Adversarial Validation — Deep
```

Its goal is:

> **Attempt to break the application the way a capable real attacker might, but inside explicit safety and authorization boundaries.**

It may:

```text
map attack surface

combine weaknesses

probe auth boundaries

fuzz inputs

test privilege transitions

simulate cloud attack techniques

validate exploitability

search for lateral attack paths
```

---

# 85. Red-Team Mode Is Goal-Driven

It should not simply:

```text
run every security tool.
```

Instead:

```text
threat model
      ↓
attack hypotheses
      ↓
ranked attack plan
      ↓
safe validation
      ↓
new evidence
      ↓
adapt attack plan
```

This reduces wasted scans and improves depth.

---

# 86. Red-Team Boundaries

Before active testing AgentCode must know:

```text
target scope

allowed environment

allowed network ranges/domains

destructive action policy

credential policy

rate limits

time budget
```

---

# 87. Red-Team Lab Manager

For the deepest validation AgentCode should prefer isolated environments.

Possible:

```text
Docker

local VM

temporary application deployment

temporary test database

temporary cloud account

CloudGoat-style scenario
```

---

# 88. Red-Team Snapshot

Before aggressive testing:

```text
capture environment state
```

where possible.

After testing:

```text
tear down / restore.
```

---

# 89. Cloud Security

Security Mode should support cloud analysis through:

```text
Prowler

ScoutSuite

CloudSploit
```

and other future adapters.

---

# 90. Cloud Security Scope

Potential targets:

```text
AWS

Azure

GCP

Kubernetes

GitHub

Cloudflare

other supported platforms
```

Exact provider support depends on integrated tools.

---

# 91. Cloud Credentials

Default cloud security credentials should be:

```text
read-only
```

where possible.

Security audit should not require mutation rights.

---

# 92. Cloud Posture Analysis

Look for:

```text
public resources

weak IAM

overprivileged roles

open network paths

encryption issues

logging gaps

dangerous storage permissions

secret exposure

CI/CD trust issues
```

---

# 93. Cloud Attack Reasoning

AgentCode should build:

```text
principal
   ↓
permission
   ↓
resource
   ↓
possible escalation
```

rather than merely producing a compliance list.

---

# 94. Cloud Attack Graph

Example:

```text
CI role
  ↓ can assume
DeploymentRole
  ↓ can pass
AdminRole
  ↓ can read
SensitiveBucket
```

This represents a security path requiring prioritization.

---

# 95. Stratus Red Team

Reference:

```text
stratus-red-team
```

Study/integrate controlled cloud attack-technique simulation.

Use only in authorized environments.

---

# 96. Pacu

Reference:

```text
pacu
```

Useful primarily as an architectural and authorized AWS offensive-security integration reference.

Deep AWS exploitation should remain opt-in and scoped.

---

# 97. CloudGoat

Reference:

```text
cloudgoat
```

Useful for:

```text
isolated intentionally vulnerable cloud labs

testing AgentCode cloud red-team logic

security regression benchmarks
```

AgentCode should not need real production infrastructure to test its red-team subsystem.

---

# 98. Cloud Security Validation Levels

Recommended:

```text
POSTURE_ONLY

ATTACK_PATH_ANALYSIS

SAFE_ACTIVE_VALIDATION

LAB_EXPLOITATION
```

---

# 99. Production Cloud Default

Production cloud defaults to:

```text
POSTURE_ONLY
+
ATTACK_PATH_ANALYSIS
```

Active cloud techniques require explicit user authorization.

---

# 100. AI Security

AgentCode itself and applications built by AgentCode may include:

```text
LLMs

RAG

tool-calling agents

MCP

chatbots

AI workflows
```

These require specialized adversarial testing.

---

# 101. AI Security References

Already cloned:

```text
promptfoo

garak

PyRIT
```

Use as inspiration/integration sources.

---

# 102. AI Security Categories

Potential tests:

```text
direct prompt injection

indirect prompt injection

system prompt leakage

secret leakage

RAG poisoning

tool abuse

cross-agent manipulation

unsafe tool argument generation

MCP trust abuse

excessive agency

data boundary violations
```

---

# 103. AI Security Trigger

If Doc 02 detects:

```text
OpenAI SDK

Anthropic SDK

Gemini SDK

LangChain

LLM routes

MCP server/client

RAG infrastructure
```

AgentCode may recommend:

```text
AI Security Audit.
```

---

# 104. AI Security Validation

Testing should remain within test/sandbox infrastructure.

Synthetic secrets and dummy tools are ideal.

Example:

```text
tool:
delete_all_users()

security test expects:
agent refuses / policy prevents invocation
```

---

# 105. AgentCode Self-Security Testing

Because AgentCode itself is an autonomous agent, it should eventually test its own:

```text
prompt injection resistance

skill trust

MCP trust

tool permissions

secret isolation

sandbox escape prevention
```

using these frameworks.

---

# 106. Security Tools as Native Experience

Security tools may be separate upstream binaries internally.

From the user perspective:

```text
AgentCode
→ Security
→ Full Audit
```

should feel native.

The user should not manually orchestrate:

```text
Semgrep

Trivy

ZAP

Prowler

etc.
```

---

# 107. Why Not Vendor Everything Into AgentCode Source

AgentCode should not maintain forks of all scanner engines.

That would create:

```text
huge maintenance burden

security-update lag

license complexity

build complexity

duplicate engineering
```

Instead AgentCode owns:

```text
adapter

policy

orchestration

result normalization

AI triage

validation

reporting
```

---

# 108. Bundled Tools

Doc 07 will decide per tool whether AgentCode:

```text
bundles binary

downloads pinned version

uses system installation

invokes container

calls API
```

based on:

```text
license

platform support

size

update frequency

security
```

---

# 109. Security Adapter Contract

Conceptually:

```text
tool_id

version

supported_targets

scan_type

input_scope

risk_level

execute()

normalize_results()

raw_report_ref
```

---

# 110. Normalized Finding Format

All scanners should feed one AgentCode representation.

This lets AgentCode reason across:

```text
Semgrep

Trivy

ZAP

Prowler

Nuclei
```

without giving models five incompatible report formats.

---

# 111. Finding Deduplication

Multiple scanners may identify the same issue.

AgentCode should merge:

```text
same vulnerability

same affected location

same root cause
```

while retaining each source as supporting evidence.

---

# 112. Root Cause Grouping

Example scanner output:

```text
3 vulnerable routes
```

may share one cause:

```text
missing authorization middleware.
```

Report the root cause plus affected instances.

---

# 113. Security Report Structure

Every confirmed issue should include:

```text
ID

Title

Severity

Confidence

Exploitability

Category

Affected Component

Attack Surface

Root Cause

Evidence

Attack Path

Potential Impact

Validation Method

Recommended Fix

Suggested Regression Test

Status
```

---

# 114. Example Security Finding

```text
AC-SEC-014

Severity:
HIGH

Confidence:
CONFIRMED

Exploitability:
PROVEN

Title:
Expired password-reset tokens remain usable

Affected:
src/auth/reset.ts
src/auth/password.ts

Attack Surface:
POST /api/auth/reset

Root Cause:
Token state is validated only at issuance.

Validation:
Confirmed against local test deployment using synthetic account.

Impact:
Unauthorized account takeover if an old token is obtained.

Recommended Remediation:
Invalidate outstanding reset tokens after successful password change and enforce expiry during token verification.

Regression Test:
tests/security/reset-token-expiry.test.ts

Status:
OPEN
```

---

# 115. Security Reports

Generate:

```text
security-report.md

security-report.json

security-report.sarif
```

Optional:

```text
HTML report
```

later.

---

# 116. SARIF

SARIF output enables future integration with:

```text
code scanning interfaces

CI

developer tooling
```

AgentCode's normalized findings should map into SARIF where appropriate.

---

# 117. Security Executive Summary

Report top-level:

```text
Critical:
0

High:
2

Medium:
7

Low:
11

Confirmed:
8

Dismissed false positives:
19
```

Also include:

```text
highest-risk attack paths

systemic root causes

recommended remediation order
```

---

# 118. Security Repair Planning

After confirmed findings:

```text
Security Repair Planner
```

generates tasks grouped by:

```text
root cause

dependency

severity

risk of regression
```

---

# 119. Security Fix Priority

Suggested ordering:

```text
actively exploitable critical

actively exploitable high

privilege-escalation paths

secret exposure

public attack surface

high-impact misconfiguration

remaining medium/low
```

---

# 120. Security Remediation Workflow

```text
confirmed finding
      ↓
repair task
      ↓
Worker implementation
      ↓
normal tests
      ↓
security regression
      ↓
re-run relevant scanner
      ↓
Verifier
      ↓
finding CLOSED
```

---

# 121. Fix Verification

A finding cannot move:

```text
FIXED
→ CLOSED
```

until AgentCode verifies the underlying exploit/condition no longer exists.

---

# 122. Security Regression Tests

When practical, confirmed vulnerabilities should create permanent regression tests.

Examples:

```text
authorization test

malicious input test

expired token test

permission test

CI configuration policy
```

This prevents later reintroduction.

---

# 123. Security Evidence Freshness

If related code changes after a security finding is closed:

```text
security evidence may become stale.
```

Doc 02 impact analysis helps decide whether revalidation is required.

---

# 124. Vulnerability Suppression

Users may suppress findings.

Each suppression should record:

```text
finding

reason

user

date

scope

expiry optional
```

Suppressed does not mean:

```text
nonexistent.
```

---

# 125. Risk Acceptance

For a real issue the user chooses not to fix:

```text
status = ACCEPTED_RISK
```

with rationale.

Final mission completion policy determines whether accepted risk is allowed.

---

# 126. Security Gate Policies

Possible project policies:

```text
No Critical

No High

No confirmed public exploit

No hardcoded secrets

No known dependency CVSS above threshold

Custom
```

Exact implementation should not depend solely on CVSS.

---

# 127. Mission Security Gate

For production-readiness missions, default might require:

```text
Critical confirmed = 0

High blocking = 0

Secrets exposed = 0

Security verification completed
```

unless user explicitly accepts risk.

---

# 128. Not Every Mission Requires Full Security Scan

Example:

```text
rename one internal variable
```

does not require:

```text
cloud red-team engagement.
```

Security scope should be task/risk-aware.

---

# 129. Automatic Security Triggers

Examples:

```text
auth code changed
→ auth/security verification

dependency added
→ dependency scan

Terraform changed
→ IaC/cloud scan

user input path changed
→ injection/security checks

LLM tool added
→ AI security checks
```

---

# 130. Security Hook Integration

Doc 04 hooks may trigger:

```text
AfterDependencyAdded
→ dependency security scan

AfterAuthChange
→ targeted security verification

BeforeMissionComplete
→ unresolved blocking security findings check
```

---

# 131. Security Skills

Suggested:

```text
appsec

auth-security

cloud-security

aws-security

web-security

api-security

supply-chain-security

ai-security

red-team-validation
```

These load progressively.

---

# 132. Security Context Pack

Verifier receives:

```text
threat model

target component

relevant source

routes

data-flow relationships

auth boundary

scanner findings

existing tests

runtime evidence
```

Not the entire codebase by default.

---

# 133. Security Token Efficiency

Security audits can become extremely expensive.

Use staged narrowing:

```text
deterministic scanners
      ↓
rank findings
      ↓
high-value code slices
      ↓
strong model analysis
      ↓
active validation only when justified
```

Do not send:

```text
entire Semgrep raw log
+
entire repository
```

to a frontier model.

---

# 134. Parallel Security Tasks

Large audits may parallelize by attack surface:

```text
Auth

API

Dependencies

Cloud

CI/CD

AI
```

But final Security Verifier should reconcile cross-surface attack chains.

---

# 135. Cross-Surface Attack Chains

Example:

```text
GitHub Action overly privileged
        ↓
artifact writable
        ↓
deployment credential exposed
        ↓
cloud role accessible
```

No individual subsystem alone may reveal the complete risk.

---

# 136. Security Knowledge Persistence

Important facts should update:

```text
SECURITY_STATE.md
```

and structured database state.

Example:

```text
known attack surface

closed findings

accepted risks

security test commands

cloud scope

threat model version
```

---

# 137. `SECURITY_STATE.md`

Suggested:

```markdown
# AgentCode Security State

## Scope

...

## Threat Model

...

## Confirmed Open Findings

...

## Accepted Risks

...

## Closed Findings

...

## Security Tests

...

## Cloud Posture

...

## Active Attack Paths

...

## Last Full Audit

Commit:
Date:
Tools:
```

---

# 138. Security State Is Not Authoritative Over Code

Like `CONTEXT.md`:

```text
SECURITY_STATE.md
```

is a readable snapshot.

Actual repository/runtime/security evidence remains authoritative.

---

# 139. Red-Team Session Record

Conceptually:

```text
redteam_session_id

target

authorization_scope

environment

techniques

start_time

end_time

evidence

findings

cleanup_status
```

---

# 140. Cleanup Verification

After lab/red-team testing AgentCode must verify:

```text
temporary resources removed

temporary credentials revoked

test data removed where appropriate

services stopped

cloud lab torn down
```

---

# 141. Rate Limiting Active Security Tools

ZAP/Nuclei/custom probes must obey:

```text
request rate

concurrency

scope
```

to avoid accidental denial of service.

---

# 142. Network Scope Enforcement

Red-Team network requests must remain inside approved scope.

Example:

```text
allowed:
staging.example.com

not automatically allowed:
othercompany.com
```

Redirects should be scope-checked.

---

# 143. Cloud Account Scope

Cloud red-team credentials should map to explicitly approved:

```text
account

subscription

project

region
```

AgentCode must not roam into unrelated accounts reachable from local credentials.

---

# 144. Credential Safety During Security Testing

Security tools may require credentials.

Use Doc 04 Secret Broker.

Reports/logs should contain references, not raw secret values.

---

# 145. Sensitive Finding Handling

Security reports themselves may contain:

```text
internal URLs

architecture

vulnerability details
```

Store them as sensitive AgentCode artifacts.

Do not route them to:

```text
GENERIC_ONLY
```

providers under Doc 01.

---

# 146. Security Model Selection

Hard security reasoning should prefer:

```text
strong reasoning models

independent model families

trusted providers
```

Local small models can:

```text
summarize scan output

classify low-risk findings
```

but should not be the sole authority for severe vulnerability validation.

---

# 147. Independent Security Verifier

A severe finding should ideally be reviewed by a different model than:

```text
the model that discovered it.
```

This applies to both:

```text
confirming vulnerabilities
```

and:

```text
confirming remediation.
```

---

# 148. Security False-Negative Reduction

After scanner-driven analysis, use model reasoning to ask:

```text
What classes of vulnerability were not covered by the tools?
```

Examples:

```text
business logic

cross-service authorization

race conditions

unsafe workflow assumptions
```

---

# 149. Security False-Positive Reduction

After model reasoning:

```text
seek code/runtime evidence
```

before reporting severe issues as confirmed.

---

# 150. Confidence Escalation

Example:

```text
Semgrep finding
→ LOW/MEDIUM confidence

code tracing confirms path
→ HIGH

runtime validation succeeds
→ CONFIRMED
```

---

# 151. Security Model Cannot Directly Execute High-Risk Attack

Model proposes action.

Tool Policy Engine still decides:

```text
allowed?
```

Security mode does not bypass Doc 04 permissions.

---

# 152. Final Verification Pack

Before mission completion Final Verifier receives:

```text
original goal

all requirements

final repository diff

integration state

test matrix

build state

security state

open findings

accepted risks

known limitations

verification evidence
```

---

# 153. Final Audit Questions

Final audit should explicitly ask:

```text
Did we satisfy every original requirement?

Is anything implemented but not wired?

Is anything wired but not tested?

Did any test get weakened?

Did implementation introduce dead/mock code?

Do builds actually pass?

Does application run?

Are there unresolved blocking security findings?

Did parallel changes integrate correctly?

Are any known failures hidden in logs?

Did we silently change scope?
```

---

# 154. Final Audit Model Independence

Where feasible:

```text
Final Verifier
```

should use a different model/provider from the majority implementation model.

---

# 155. Final Audit Cannot Modify Code

Preferred flow:

```text
Final Audit
→ report

if failure:
→ repair task
```

This preserves independent review.

---

# 156. Final Audit Finding

If final audit discovers:

```text
missing requirement
```

Mission returns:

```text
REPAIR
```

not:

```text
COMPLETE WITH NOTE.
```

---

# 157. Security Findings as Tasks

Confirmed vulnerabilities should create task nodes.

Example:

```text
SEC-014
      ↓
Repair SEC-014
      ↓
Verify SEC-014
```

These participate in Doc 03 DAG execution.

---

# 158. Security Finding Dependencies

Example:

```text
auth middleware root fix
```

may close:

```text
SEC-014
SEC-015
SEC-018
```

after revalidation.

Root-cause-aware planning avoids duplicate fixes.

---

# 159. AppSec Reference Repositories

Primary:

```text
openhack

trailofbits-skills

semgrep

codeql

gitleaks

osv-scanner

trivy

checkov

zaproxy

nuclei

nuclei-templates

scorecard
```

---

# 160. OpenHack

Study heavily for:

```text
recon

hunting

validation

verification

sandboxed vulnerability confirmation

multi-step security workflows
```

Do not automatically inherit its complete runtime architecture.

---

# 161. Trail of Bits Skills

Study for:

```text
deep audit context

security reasoning workflows

false-positive validation

insecure-default identification

differential review

security-focused skill design
```

Doc 07 must respect license obligations.

---

# 162. Cloud Security Reference Repositories

```text
prowler

ScoutSuite

cloudsploit

stratus-red-team

pacu

cloudgoat
```

---

# 163. AI Security Reference Repositories

```text
promptfoo

garak

PyRIT
```

---

# 164. Browser Security Reference

```text
Playwright

browser-use

browser-harness
```

These support custom AgentCode adversarial browser flows.

---

# 165. Code Intelligence Integration

Doc 02 provides:

```text
routes

dependencies

symbols

test relationships

schemas

Git changes
```

Security Mode should consume these rather than re-indexing the repository separately.

---

# 166. Tool Integration

Doc 04 provides:

```text
scanner execution

browser

shell

sandbox

secret handling

network scope

hooks
```

Security Mode should not build a second tool runtime.

---

# 167. Kernel Integration

Doc 03 provides:

```text
tasks

retries

workers

requirements

evidence

completion gates
```

Security findings should become first-class Kernel state.

---

# 168. Model Router Integration

Doc 01 provides:

```text
security task routing

trusted provider filtering

model-family diversity
```

---

# 169. What We Must Not Do

Do not:

```text
trust Worker completion claims

treat passing build as full verification

treat scanner matches as automatically confirmed

treat clean scanners as proof of zero vulnerabilities

use only one model for implementation and final verification

weaken tests to make changes pass

hide failed checks

discard stale-evidence information

run active red-team operations outside authorized scope

use destructive exploitation when harmless validation is sufficient

exfiltrate real sensitive data to prove exposure

run uncontrolled high-rate scanners

give security models unrestricted cloud credentials

route sensitive security findings to untrusted providers

allow security tools to bypass AgentCode sandbox/policy

vendor/fork every scanner without reason

produce reports without remediation
```

---

# 170. Verification Acceptance Test — False Done

Worker reports:

```text
done
```

but a required test fails.

Expected:

```text
Task remains incomplete.
```

---

# 171. Requirement Trace Acceptance Test

Mission contains ten requirements.

Expected:

```text
every requirement has explicit state

every verified requirement has evidence

missing evidence prevents completion.
```

---

# 172. Weak Test Acceptance Test

Create a deliberately meaningless passing test.

Expected:

Independent Verifier identifies insufficient test quality.

---

# 173. Missing Wiring Acceptance Test

Implement backend service but intentionally omit route registration.

Expected:

Verifier/runtime testing catches incomplete wiring.

---

# 174. Incremental Verification Acceptance Test

Verify Change Set A.

Modify unrelated Change Set B.

Expected:

AgentCode does not unnecessarily rerun all expensive verification for A.

---

# 175. Evidence Staleness Acceptance Test

Verify authentication.

Change shared authentication middleware.

Expected:

affected authentication evidence becomes:

```text
STALE
```

or:

```text
REVERIFY_REQUIRED.
```

---

# 176. SAST Acceptance Test

Seed representative known insecure code in test repository.

Expected:

static-analysis adapter detects and normalizes findings.

---

# 177. Secret Detection Acceptance Test

Commit synthetic secret.

Expected:

```text
Gitleaks detects it

AgentCode redacts value

Git-history location recorded

finding created.
```

---

# 178. Dependency Acceptance Test

Add deliberately vulnerable test dependency.

Expected:

```text
OSV/Trivy finding normalized

fix recommendation generated.
```

---

# 179. IaC Acceptance Test

Create test infrastructure with deliberately unsafe public configuration.

Expected:

```text
Checkov/Trivy/cloud scanner identifies it.
```

---

# 180. False Positive Acceptance Test

Create scanner-detectable pattern that is unreachable or safely mitigated.

Expected:

```text
finding triaged

evidence inspected

false positive dismissed or downgraded.
```

---

# 181. Business Logic Security Acceptance Test

Create authorization flaw that ordinary SAST does not detect.

Expected:

Security Verifier/runtime validation identifies it.

---

# 182. Web Security Acceptance Test

Run test app containing seeded web vulnerability.

Expected:

```text
ZAP/Nuclei/custom validation detects candidate

AgentCode validates safely

finding reported.
```

---

# 183. Attack Chain Acceptance Test

Create lab with three individually moderate weaknesses that combine into high impact.

Expected:

AgentCode identifies combined path rather than reporting only isolated alerts.

---

# 184. Cloud Posture Acceptance Test

Use authorized test cloud environment with seeded misconfigurations.

Expected:

Prowler/ScoutSuite/CloudSploit results normalized and prioritized.

---

# 185. Cloud Attack Path Acceptance Test

Create lab where:

```text
Role A
→ can assume Role B
→ can access test sensitive resource
```

Expected:

AgentCode constructs attack path.

---

# 186. Red-Team Lab Acceptance Test

Deploy disposable vulnerable environment.

Expected:

```text
AgentCode scopes target

takes snapshot where appropriate

performs approved validation

creates findings

cleans up environment
```

---

# 187. Production Boundary Acceptance Test

Attempt Red-Team operation against target classified:

```text
PRODUCTION_READ_ONLY
```

Expected:

active exploit action blocked.

---

# 188. Network Scope Acceptance Test

Authorized target:

```text
staging.example.com
```

Security tool follows redirect toward unrelated domain.

Expected:

scope check prevents unauthorized testing.

---

# 189. AI Prompt Injection Acceptance Test

Create local agent application vulnerable to malicious retrieved text.

Expected:

Promptfoo/Garak/PyRIT/custom tests identify or exercise the weakness.

---

# 190. Security Repair Acceptance Test

Seed vulnerability.

Expected:

```text
detect
→ validate
→ repair
→ regression test
→ rescan
→ close finding
```

---

# 191. Security Report Acceptance Test

After audit generate:

```text
Markdown

JSON

SARIF
```

containing:

```text
severity

confidence

evidence

remediation

status.
```

---

# 192. V1 Completion Definition

This subsystem is V1-complete only when:

```text
✓ requirement traceability works

✓ mechanical verification gates work

✓ targeted tests work

✓ broad test gates work

✓ evidence persists

✓ evidence is commit-aware

✓ evidence freshness/invalidation works

✓ independent Verifier role works

✓ verifier receives independent context

✓ verifier can reject Worker completion

✓ test-quality review exists

✓ test weakening can be detected/reviewed

✓ runtime verification works

✓ browser verification works

✓ integration verification works

✓ incremental verification works

✓ final audit exists

✓ final audit can fail mission completion

✓ threat model can be generated

✓ attack surface map exists

✓ normalized security finding model exists

✓ severity/confidence/exploitability are separate

✓ Semgrep adapter works

✓ CodeQL integration path defined/implemented where appropriate

✓ Gitleaks adapter works

✓ OSV-Scanner adapter works

✓ Trivy adapter works

✓ Checkov adapter works

✓ ZAP adapter works

✓ Nuclei adapter works

✓ Nuclei templates are version/scoped

✓ OpenSSF Scorecard integration exists where useful

✓ scanner findings are deduplicated

✓ false-positive triage works

✓ application-security reasoning exists

✓ business-logic security review exists

✓ API security review exists

✓ security findings become Kernel tasks

✓ security repair workflow works

✓ security regression tests work

✓ SECURITY_STATE.md exists

✓ Markdown security reports work

✓ JSON security reports work

✓ SARIF export works

✓ Prowler integration works

✓ ScoutSuite/CloudSploit integration path works where useful

✓ cloud attack graph exists

✓ cloud credentials default to least privilege/read only

✓ Stratus integration/lab path defined

✓ Pacu lab integration path defined

✓ CloudGoat can serve as isolated benchmark/lab

✓ authorized Red-Team mode exists

✓ red-team target scope is enforced

✓ production defaults remain non-destructive

✓ synthetic canary validation works

✓ attack-chain reasoning works

✓ lab cleanup works

✓ AI Security mode exists

✓ Promptfoo integration works

✓ Garak integration works

✓ PyRIT integration path works

✓ security evidence obeys provider trust policy

✓ security tools obey Doc 04 sandbox/policy

✓ critical security findings can block mission completion

✓ accepted risk is explicit

✓ final mission report reflects unresolved/accepted security state
```

---

# 193. Locked V1 Architectural Principles

The following are locked:

1. Completion requires evidence.

2. Worker self-report is not completion evidence.

3. Requirements must be individually traceable.

4. Mechanical checks are necessary but insufficient.

5. Verification is layered.

6. Tests are evidence, not infallible truth.

7. Test quality can itself require verification.

8. Verification evidence is tied to repository state.

9. Related code changes may invalidate prior evidence.

10. Incremental verification should avoid repeated expensive review.

11. Final mission verification remains broader.

12. Independent verification should use different model/provider families where practical.

13. Verifiers should be adversarial, not agreeable.

14. Verifiers should not normally edit code directly.

15. Missing wiring is a first-class verification target.

16. Integrated application behavior matters more than isolated file correctness.

17. Security is a first-class verification dimension.

18. Scanner output is evidence, not automatic truth.

19. Clean scanners do not prove absence of vulnerability.

20. Threat models are repository-specific.

21. Attack surfaces should be mapped before deep testing.

22. Security findings separate severity, confidence and exploitability.

23. False-positive reduction is mandatory.

24. High-severity findings should be validated when safely possible.

25. Security validation should prove the minimum necessary impact.

26. Real user data should not be exfiltrated merely for demonstration.

27. Synthetic canary data should be preferred.

28. Red-Team testing is restricted to authorized systems.

29. Production defaults to non-destructive assessment.

30. Deep exploitation belongs in isolated labs/staging unless explicitly approved.

31. Attack chains matter more than isolated findings.

32. Cloud security includes posture and attack-path reasoning.

33. Cloud credentials default to least privilege.

34. Active cloud attack simulation is opt-in.

35. AI applications receive AI-specific security testing.

36. AgentCode should eventually red-team its own agent/security boundaries.

37. Security tools must obey the same Tool Broker and sandbox policies.

38. Security tools should be integrated, not unnecessarily rewritten.

39. Findings should identify root causes.

40. Confirmed vulnerabilities should produce regression tests where practical.

41. A fixed vulnerability is not closed until revalidated.

42. Accepted risk must be explicit.

43. Security reports should provide concrete remediation.

44. Security reports should be machine-readable and human-readable.

45. Sensitive security data follows Doc 01 provider-trust rules.

46. Security tasks participate in the same durable Kernel DAG.

47. Security context should remain relevance-selected.

48. Security auditing should optimize depth, not simply number of scanners executed.

49. Final Audit must inspect unresolved security state.

50. No mission should be labeled production-ready while blocking confirmed security findings remain unresolved unless explicitly accepted under policy.

---

# 194. Final Architecture

```text
                          IMPLEMENTATION
                                │
                                ▼
                    MECHANICAL VERIFICATION
                                │
            ┌───────────────────┼───────────────────┐
            ▼                   ▼                   ▼
          BUILD               TESTS            TYPE/LINT
            │                   │                   │
            └───────────────────┼───────────────────┘
                                ▼
                     REQUIREMENT TRACEABILITY
                                │
                                ▼
                    INDEPENDENT VERIFIER
                                │
                                ▼
                    INTEGRATION VERIFICATION
                                │
                                ▼
                         SECURITY ENGINE
                                │
        ┌───────────────────────┼────────────────────────┐
        ▼                       ▼                        ▼
      SAST                 DEPENDENCIES               SECRETS
 Semgrep/CodeQL           OSV/Trivy                 Gitleaks
        │                       │                        │
        ├───────────────────────┼────────────────────────┤
        ▼                       ▼                        ▼
       IaC                    WEB                     CLOUD
 Checkov/Trivy          ZAP / Nuclei        Prowler/ScoutSuite
        │                       │                        │
        └───────────────────────┼────────────────────────┘
                                ▼
                         THREAT MODEL
                                │
                                ▼
                     SECURITY REASONER
                                │
                                ▼
                         ATTACK GRAPH
                                │
                 ┌──────────────┴───────────────┐
                 ▼                              ▼
           SAFE VALIDATION                AI SECURITY
              / LAB                 Promptfoo/Garak/PyRIT
                 │                              │
                 └──────────────┬───────────────┘
                                ▼
                      CONFIRMED FINDINGS
                                │
                                ▼
                        REPAIR PLANNER
                                │
                                ▼
                             WORKER
                                │
                                ▼
                        SECURITY RETEST
                                │
                                ▼
                         FINAL AUDIT
                                │
                                ▼
                    KERNEL COMPLETION GATE
                          /             \
                        FAIL            PASS
                         │                │
                         ▼                ▼
                       REPAIR       MISSION COMPLETE
```

---

# 195. Final Statement

AgentCode should not simply write software.

It should be capable of **challenging its own software**.

When a Worker says:

```text
"I implemented the feature."
```

AgentCode should ask:

```text
"Where is the evidence?"
```

When the tests pass:

```text
AgentCode should ask whether the tests are actually meaningful.
```

When an implementation looks complete:

```text
the Verifier should search for missing wiring.
```

When scanners show vulnerabilities:

```text
AgentCode should determine which are actually real.
```

When scanners show nothing:

```text
AgentCode should still reason about business logic and trust boundaries.
```

When several weak security findings exist:

```text
AgentCode should determine whether they combine into a serious attack path.
```

When a serious weakness is suspected:

```text
AgentCode should validate it safely in an authorized environment.
```

When a fix is made:

```text
AgentCode should attempt the same attack again and verify that it no longer succeeds.
```

When cloud infrastructure exists:

```text
AgentCode should reason about permissions and escalation paths, not merely compliance checkboxes.
```

When an AI-powered application exists:

```text
AgentCode should test prompt injection, tool abuse and data leakage.
```

And when everything appears finished:

```text
a final independent audit should compare the actual repository and evidence against the user's original goal one more time.
```

The intended result is:

> **A verification and security system that turns AgentCode from an autonomous code generator into an autonomous software-engineering system capable of proving correctness, detecting incomplete implementation, identifying and validating meaningful security weaknesses, reasoning about realistic attack chains, safely simulating adversarial behavior in authorized environments, repairing confirmed vulnerabilities, and refusing to declare a mission complete until the available evidence supports that conclusion.**

This document is the **V1 source of truth for AgentCode's Verification, Security and Red-Team subsystem.**


---

# HARDENING ADDENDUM — V2 ARCHITECTURE EXPANSION

## Purpose of This Revision

This revision preserves the original Doc 05 architecture and strengthens it with the missing implementation-level details required for a production-grade autonomous coding environment.

The original document correctly establishes that AgentCode must optimize for **PROVEN COMPLETION** rather than plausible completion. The hardened architecture extends that principle into concrete evidence ownership, verification lifecycle management, security operations, adversarial validation, and mission completion rules.

---

# 161. Verification Must Be Evidence-Graph Based

A simple list of passed tests is insufficient.

AgentCode should maintain an evidence graph:

```
Requirement
    |
    +--> Implementation Evidence
    |
    +--> Test Evidence
    |
    +--> Runtime Evidence
    |
    +--> Security Evidence
    |
    +--> Review Evidence
    |
    +--> Final Decision
```

Every completion claim must resolve through this graph.

A requirement with missing evidence remains incomplete.

---

# 162. Evidence Ownership

Every evidence object must have:

```
evidence_id
producer
timestamp
repository_commit
environment
command_or_action
raw_artifact_reference
interpretation
confidence
expiry_state
```

Evidence without provenance should not influence completion decisions.

---

# 163. Verification Cannot Trust the Implementer

The Worker implementing a feature is not the authority deciding that the feature is complete.

Required separation:

```
Implementation Agent
        |
        v
Verification System
        |
        v
Independent Reviewer
        |
        v
Kernel Completion Gate
```

The system must actively search for reasons to reject completion.

---

# 164. Verification Anti-Patterns

The verifier must detect:

- "implemented" features without reachable code paths;
- tests that only validate mocks;
- feature flags permanently disabling functionality;
- commented-out unfinished logic;
- fallback paths hiding failures;
- swallowed exceptions;
- environment-specific accidental success;
- tests modified only to match broken behavior;
- documentation claiming support that code does not provide.

---

# 165. Verification Confidence Model

Every completion decision should include:

```
Implementation Confidence
Verification Confidence
Security Confidence
Integration Confidence
Overall Confidence
```

A high implementation confidence with weak verification confidence is not completion.

---

# 166. Mechanical Gate Trust Rules

Mechanical checks prove only what they measure.

Examples:

Build passing proves:

- compilation succeeds.

It does not prove:

- correct business behavior;
- security;
- usability;
- integration completeness.

Tests passing prove:

- tested scenarios pass.

They do not prove:

- missing scenarios do not fail.

---

# 167. Security Architecture Expansion

Security analysis should operate through five layers:

```
Asset Discovery
        |
Threat Modeling
        |
Detection
        |
Validation
        |
Remediation Verification
```

Skipping validation creates noisy security reports.

---

# 168. Security Finding Lifecycle

A finding moves through:

```
DISCOVERED
      |
TRIAGED
      |
VALIDATING
      |
CONFIRMED / DISMISSED
      |
REPAIR_PLANNED
      |
FIX_IMPLEMENTED
      |
REGRESSION_VERIFIED
      |
CLOSED
```

No scanner result should directly become a confirmed vulnerability.

---

# 169. Attack Path Reasoning

AgentCode must prioritize realistic attack chains.

Example:

```
Low privilege account
        |
        v
Missing ownership validation
        |
        v
Unauthorized object access
        |
        v
Sensitive information exposure
```

The chain matters more than isolated alerts.

---

# 170. Red-Team Safety Model

Red-team capabilities require explicit boundaries.

Before execution:

```
Target
Authorization
Environment
Scope
Allowed Techniques
Cleanup Requirement
```

must exist.

The red-team subsystem must never silently expand scope.

---

# 171. Safe Validation Philosophy

The goal is:

```
prove vulnerability exists
```

not:

```
maximize damage
```

Preferred validation:

- synthetic accounts;
- synthetic secrets;
- isolated databases;
- disposable environments;
- minimal reproduction cases.

---

# 172. AI Security Expansion

AgentCode must treat AI systems as security-sensitive components.

Required checks:

- prompt injection resistance;
- tool permission boundaries;
- retrieval poisoning;
- secret exposure;
- unsafe autonomous actions;
- MCP trust boundaries;
- excessive agent authority.

---

# 173. AgentCode Self-Audit

Because AgentCode is itself autonomous, it must eventually audit:

```
Tool permissions
Sandbox boundaries
Secret handling
Skill trust
MCP trust
Prompt isolation
Memory integrity
```

---

# 174. Final Mission Completion Rules

AgentCode must refuse:

```
COMPLETE
```

when:

- blocking requirements remain unverified;
- critical security findings remain open;
- evidence is stale;
- integration verification failed;
- implementation differs from stated requirements.

Correct state:

```
MISSION COMPLETE
ONLY WHEN:
Requirement satisfied
+
Evidence available
+
Verification passed
+
Security acceptable
+
No hidden blockers
```

---

# 175. Production Readiness Checklist

Before declaring a mission production-ready:

- requirement matrix reconciled;
- final diff reviewed;
- tests executed;
- runtime behavior confirmed;
- security assessment completed;
- secrets checked;
- dependencies reviewed;
- deployment configuration checked;
- known risks documented;
- evidence package stored.

---

# Revision Summary

This hardened version keeps the original Doc 05 structure while adding:

- stronger evidence architecture;
- stricter verifier independence;
- evidence provenance;
- anti-false-completion controls;
- expanded security lifecycle;
- attack-chain prioritization;
- safer red-team execution;
- AI-agent security validation;
- final mission completion enforcement.


