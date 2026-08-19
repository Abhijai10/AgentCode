# AgentCode  
# 03 — Autonomy Kernel & Agent Runtime Architecture

**Document Status:** V1 — Architecture Locked for Initial Implementation; Hardening Revision 2  
**Date:** 19 August 2026  
**Project:** AgentCode  
**Document Type:** Core Architecture Specification  
**Hardening Basis:** Original Doc 03 preserved; canonical implementation contracts added and contradictions normalized  
**Depends On:**  
- `01 — Model, Provider, Routing & Reliability Architecture`
- `02 — Code Intelligence, Context & Persistent Memory Architecture`

**Scope:** Missions, goals, immutable requirements, planning, task DAGs, Planner/Worker/Researcher/Verifier runtime, durable state, leases, heartbeats, worker lifecycle, retries, checkpoints, crash recovery, provider/model replacement, task scheduling, concurrency, work queues, event log, mission resumption, background daemon, human escalation, autonomous Goal Mode, agent communication, failure classification, resource coordination and deterministic mission completion.


---

# 0. Hardening Revision — Normative Implementation Contract

This hardening revision preserves the architecture and intent of the original Doc 03 while turning the most implementation-critical concepts into explicit contracts. The original document already established the correct high-level principles: the Kernel is the source of mission truth; LLMs are replaceable workers; the daemon survives UI closure; requirements, tasks, leases, checkpoints, retries, events, verification and final completion are durable Kernel-managed state; and provider/model failure must not destroy mission progress.

The hardening pass adds the layer that an implementation team would otherwise be forced to invent:

- canonical identifiers and record ownership;
- authoritative Mission, Requirement, Plan, Task, Attempt, Worker, Session, Lease, Checkpoint, Message, Event, Resource Lock and Human Request records;
- formal task and mission state machines;
- scheduler and work-queue semantics;
- lease fencing and zombie-worker prevention;
- durable Agent Runtime/session semantics;
- explicit failure levels;
- retry/recovery algorithms;
- restart reconciliation order;
- transaction catalog;
- logical SQLite schema;
- daemon singleton/IPC semantics;
- pause/resume/cancel behavior;
- human escalation state;
- resource admission;
- integration and verification coordination;
- security/trust boundaries;
- observability;
- deterministic benchmark and acceptance-test catalogs.

Where a later canonical contract in Section 0 conflicts with a shorter conceptual example in the original explanatory sections, **the canonical contract in this hardening section is normative for V1**. The original sections remain useful explanation and architectural rationale.

---

## 0.1 Normative Decision Classes

Not every value in this document has the same stability. Every important implementation decision belongs to one of the following classes.

| Class | Meaning | Examples in Doc 03 |
|---|---|---|
| `LOCKED_ARCHITECTURE` | Core architecture. Changing it requires an ADR and reconciliation of dependent docs. | Kernel owns mission/task/completion truth; SQLite is the V1 durable control store; LLMs are replaceable; one authoritative Kernel writer; task leases; final mission completion is deterministic. |
| `CONSTRAINED_IMPLEMENTATION_DECISION` | The design space is bounded, but the exact library/encoding/transport may be chosen during implementation. | Exact local IPC transport; concrete SQLite indexes; daemon process wrapper; timer implementation. |
| `BENCHMARK_PENDING` | A starting policy is defined but numerical tuning may change after measurement. | lease TTL; heartbeat cadence; scheduler weights; stall windows; default worker concurrency. |
| `DYNAMIC_RUNTIME_DATA` | Runtime state that must be observed, never treated as a static architecture constant. | active provider capacity, resource pressure, current worktree HEAD, task queue depth, active worker count. |
| `OPTIONAL_V1` | V1 architecture supports it, but V1 correctness must not depend on it. | local Sentinel summaries, mission templates, advanced scheduler heuristics. |
| `POST_V1` | Explicitly outside the required V1 implementation unless pulled forward by ADR. | distributed remote workers, organization collaboration, learned scheduler, mission forking UI, remote Kernel clustering. |

The following are `LOCKED_ARCHITECTURE`:

1. The **Kernel**, not an LLM, owns authoritative mission, task and completion state.
2. The **daemon** is independent of the desktop window.
3. The **original user goal is immutable** once the mission is created.
4. Requirements and plans are versioned and auditable.
5. Task execution is represented by a persistent DAG and explicit task states.
6. A Worker is replaceable; a model session is even more replaceable.
7. Active task ownership uses a lease plus fencing token.
8. Provider/model failures do not automatically fail the task.
9. Ordinary retry/recovery is deterministic and bounded.
10. Task completion requires independent verification where policy requires it.
11. Mission completion requires mission-level Final Audit plus deterministic completion gates.
12. SQLite is the V1 transactional control-plane store.
13. Git/worktrees provide code-state evidence but are not the mission-state database.
14. All authoritative state transitions are validated and emitted as durable events.
15. Routine local work should proceed without repetitive human approval.
16. The Kernel must remain correct even if every LLM and local helper model is unavailable.

---

## 0.2 Responsibility and Non-Ownership Matrix

A central objective of AgentCode is to avoid duplicated authorities. Each subsystem owns one class of decisions and explicitly does **not** own adjacent classes.

| Subsystem | Owns | Must Never Own |
|---|---|---|
| **Background Daemon** | Process lifetime, singleton instance, IPC server, Kernel hosting, subsystem lifecycle, restart orchestration | Mission semantics, model ranking, repository facts |
| **Autonomy Kernel** | Mission/requirement/plan/task state, scheduling, leases, attempts, retries, recovery, resource admission, completion | Provider scoring, prompt/context retrieval internals, Git/edit mechanics, verifier reasoning |
| **Mission Manager** | Mission record lifecycle and mission-level commands | Task implementation |
| **Requirement Manager** | Requirement versions, precedence, traceability, supersession | Repository implementation facts |
| **Planner Runtime** | Proposed plan/task decomposition | Authoritative task creation without Kernel validation |
| **Scheduler** | Which READY task should receive execution capacity next | Which model/provider is best |
| **Work Queue** | Persistent/derived dispatch ordering and backpressure | Task truth |
| **Agent Runtime** | Session execution, model turns, tool-call loop, cancellation, session replacement | Task completion authority |
| **Worker Registry** | Logical worker/session bookkeeping | Model selection |
| **Lease Manager** | Exclusive task ownership and fencing | Task semantics |
| **Recovery Engine** | Reconciliation and recovery decision execution | Provider routing internals |
| **Retry Controller** | Attempt/task retry budgets and escalation | Inference-request retries inside a provider adapter |
| **Model Broker / Doc 01** | Model/provider candidate selection, routing, provider health/quota/budget details | Task DAG, mission completion |
| **Context Engine / Doc 02** | Context construction, repository intelligence, handoff packs, freshness | Task lifecycle |
| **Tool/Edit/Git Engine / Doc 04** | Tool execution, sandbox, transactional edits, worktrees, Git mechanics | Mission/task truth |
| **Verification Engine / Doc 05** | Verification procedure, evidence interpretation, findings, security validation | Kernel state mutation without validated result |
| **Desktop UI** | User presentation and user-issued commands | Direct SQLite mutation, mission authority |

The daemon may *coordinate* model routing, indexing, browser processes, tools and worktrees, but coordination does not transfer ownership.

---

## 0.3 Canonical Identifier Model

Every durable object uses a stable opaque identifier. Human-readable titles are not identifiers.

Recommended form is UUIDv7/ULID-like sortable opaque IDs or another collision-resistant format selected by ADR. Exact encoding is a `CONSTRAINED_IMPLEMENTATION_DECISION`.

Canonical IDs:

```text
mission_id
requirement_id
requirement_version_id
plan_id
plan_version_id
task_id
task_attempt_id
worker_id
agent_session_id
lease_id
checkpoint_id
message_id
blackboard_entry_id
event_id
human_request_id
resource_lock_id
operation_id
idempotency_key
recovery_record_id
notification_id
```

Rules:

- IDs never depend on titles or array positions.
- IDs are never reused after deletion/supersession.
- `task_id` remains stable across task retries.
- `task_attempt_id` changes for each materially new execution attempt.
- `worker_id` represents the logical Worker assigned to a task.
- `agent_session_id` represents one concrete model/runtime session and may change while the same logical Worker continues.
- `lease_id` is unique per ownership acquisition.
- Every lease also carries a monotonically increasing **fencing epoch** for that task.
- Events carry `correlation_id` and `causation_id` when available.
- UI/IPC clients may submit `idempotency_key` for commands that could otherwise be duplicated on reconnect.

No model-generated payload may choose an existing authoritative ID unless the Kernel explicitly asked it to reference that ID.

---

## 0.4 Canonical Mission Record

A Mission is the highest-level durable unit of work.

Canonical logical record:

```text
Mission {
  mission_id
  repository_scope[]           // repo/view identifiers, immutable except explicit scope revision
  original_goal                // verbatim user text, immutable
  interpreted_goal             // structured interpretation, versioned
  status                       // MissionState
  risk_class
  autonomy_policy_ref
  verification_policy_ref
  security_policy_ref
  budget_policy_ref
  current_requirement_version
  current_plan_version
  created_at
  started_at?
  paused_at?
  completed_at?
  cancelled_at?
  failed_at?
  blocked_reason?
  blocked_request_id?
  last_state_change_at
  final_audit_evidence_ref?
  completion_summary_ref?
}
```

### Mission invariants

1. `original_goal` is append-never / overwrite-never.
2. Every requirement version is traceable to the mission and prior requirement version.
3. Every plan version is traceable to the requirement version it was built against.
4. A mission cannot enter `EXECUTING` without at least one published requirement version and plan.
5. A mission cannot enter `COMPLETE` if any mandatory requirement is not in an accepted terminal requirement state.
6. A mission cannot enter `COMPLETE` while a required task is nonterminal, a blocking finding exists, evidence is stale, integration is incomplete, or Final Audit failed.
7. A mission may be `PAUSED` while preserving its prior resumable phase.
8. `FAILED` is terminal and rare. Ordinary execution failures use `RECOVERING` or `BLOCKED`.
9. Mission completion and completion-notification publication occur only after the completion transaction commits.

---

## 0.5 Requirement System — Canonical Contract

Requirements are the durable bridge between the user goal and the task DAG. They prevent long missions from silently narrowing scope.

Canonical requirement record:

```text
Requirement {
  requirement_id
  mission_id
  introduced_in_requirement_version
  superseded_in_requirement_version?
  source_type
  source_ref?
  title
  description
  type
  priority
  blocking
  explicitness              // EXPLICIT | IMPLIED | DERIVED
  verification_profile
  status
  accepted_risk_ref?
  approved_out_of_scope_ref?
  evidence_refs[]
  created_at
  updated_at
}
```

Canonical requirement statuses:

```text
PROPOSED
ACTIVE
IN_PROGRESS
IMPLEMENTED_UNVERIFIED
VERIFIED
FAILED
BLOCKED
SUPERSEDED
ACCEPTED_RISK
APPROVED_OUT_OF_SCOPE
```

Only these count as satisfied for mission completion:

```text
VERIFIED
ACCEPTED_RISK
APPROVED_OUT_OF_SCOPE
```

`ACCEPTED_RISK` and `APPROVED_OUT_OF_SCOPE` require explicit durable authorization according to project/user policy. A Planner cannot create those statuses by itself.

### Requirement precedence

For conflicting requirements:

```text
new explicit user instruction
>
older explicit user instruction
>
accepted architecture/project policy
>
approved scoped project instruction
>
user-implied requirement
>
planner-derived requirement
```

When a newer explicit instruction replaces an older one, the old requirement is not deleted. It becomes `SUPERSEDED` with a link to the replacement and a reason.

### Requirement versions

A requirement version is an immutable snapshot of the active requirement set at a point in mission time.

```text
RequirementVersion {
  requirement_version_id
  mission_id
  sequence_number
  parent_version_id?
  created_at
  cause
  source_event_id
  active_requirement_ids[]
  content_hash
}
```

Any addition/removal/replacement that changes mission obligations publishes a new requirement version. Existing tasks are reconciled against that new version; already valid evidence is preserved unless the new requirement or repository changes invalidate it.

---

## 0.6 Goal Interpretation Contract

Goal interpretation may use an LLM, but its output is advisory structured data.

Input:

```text
GoalInterpretationRequest {
  mission_id
  original_goal
  repository_scope
  project_constraints
  known architecture decisions
  policy_summary
}
```

Output proposal:

```text
GoalInterpretationProposal {
  interpreted_goal
  proposed_requirements[]
  explicit_unknowns[]
  ambiguity_items[]
  destructive_or_external_risks[]
  likely_task_categories[]
  repository_coverage
  suggested_verification_profile
  suggested_security_profile
  required_human_decisions[]
}
```

Kernel validation must reject:

- requirements outside repository/mission scope without justification;
- planner-generated constraints that conflict with explicit user instructions;
- empty or unverifiable acceptance language for mandatory requirements;
- hidden destructive operations;
- silent removal of user-specified requirements;
- invented credentials, permissions or external authority.

Human clarification is requested only when an ambiguity materially changes irreversible behavior, external mutation, legal/security scope, paid budget, or the definition of success. Ordinary implementation ambiguity should be resolved autonomously through repository evidence, research or reversible experimentation.

---

## 0.7 Planning Contract and Plan Publication

A Planner does not directly create authoritative tasks. It submits a plan proposal.

Canonical request:

```text
PlanRequest {
  mission_id
  requirement_version_id
  repository_generation_refs[]
  current_plan_version?
  current_task_summary?
  known_blockers[]
  accepted_decisions[]
  resource_policy
}
```

Canonical proposed task:

```text
TaskProposal {
  local_proposal_id
  title
  description
  task_type
  requirement_refs[]
  acceptance_criteria[]
  dependency_proposal_refs[]
  priority
  risk_class
  required_skills[]
  required_tools[]
  required_context_profile
  expected_read_scope[]
  expected_write_scope[]
  expected_outputs[]
  verification_expectations[]
  resource_needs[]
  interface_contract_refs[]
  estimated_scope
}
```

Canonical plan publication:

```text
PlanVersion {
  plan_version_id
  mission_id
  sequence_number
  parent_plan_version_id?
  requirement_version_id
  created_at
  planner_session_id
  rationale_summary
  task_ids[]
  superseded_task_ids[]
  validation_result
  content_hash
}
```

Before publication, the Kernel validates:

1. every mandatory active requirement maps to one or more implementation/verification obligations;
2. all dependency references resolve;
3. the graph is acyclic;
4. no task is both its own ancestor and descendant;
5. task scopes are within mission repository scope;
6. destructive/external work has appropriate policy markers;
7. required verification/integration/final-audit obligations exist;
8. tasks are not silently duplicated;
9. tasks do not hide unresolved user questions inside implementation descriptions.

A rejected plan proposal remains evidence but is never made current.

---

## 0.8 Task DAG Semantics

The task graph is a persistent DAG of work obligations.

Dependency edge:

```text
TaskDependency {
  predecessor_task_id
  successor_task_id
  dependency_type
  required_predecessor_state
  created_in_plan_version
  superseded_in_plan_version?
}
```

Recommended dependency types:

```text
HARD_COMPLETION       // successor cannot start until predecessor passes/integrates as specified
CONTRACT_READY        // successor may start when a shared interface/contract checkpoint is published
EVIDENCE_READY        // successor requires research/verification evidence
INTEGRATION_BARRIER   // successor is an integration stage
RESOURCE_SERIAL       // ordering imposed by exclusive shared resource
```

`HARD_COMPLETION` is the default.

Optional or conditional tasks must be explicitly labeled and linked to the condition that activates them. They are not silently omitted from the plan.

Dynamic task insertion:

```text
discovery
→ proposal
→ Kernel validates mission relevance
→ requirement/plan reconciliation if needed
→ new task inserted
→ dependencies recomputed
```

Completed tasks are never deleted during replanning. They may become `SUPERSEDED`, and their still-valid evidence remains available.

---

## 0.9 Canonical Task Record

```text
Task {
  task_id
  mission_id
  created_in_plan_version
  superseded_in_plan_version?
  title
  description
  task_type
  status
  priority
  critical_path_score
  risk_class
  blocking_value
  requirement_refs[]
  acceptance_criteria[]
  dependency_summary
  expected_read_scope[]
  expected_write_scope[]
  required_skills[]
  required_tools[]
  required_context_profile
  resource_needs[]
  policy_profile_ref
  task_budget
  repository_view_ref
  worktree_ref?
  assigned_worker_id?
  current_attempt_id?
  active_lease_id?
  latest_checkpoint_id?
  evidence_refs[]
  blocked_reason?
  terminal_reason?
  created_at
  ready_at?
  started_at?
  implemented_at?
  passed_at?
  integrated_at?
  completed_at?
  updated_at
}
```

Immutable after creation except through explicit plan reconciliation:

- `task_id`
- `mission_id`
- original task title/description snapshot
- creation plan lineage.

Mutable through controlled Kernel transitions:

- priority;
- status;
- current attempt/worker/lease;
- scope expansion;
- evidence;
- timestamps;
- blocking reason.

No Worker writes the Task record directly.

---

## 0.10 Canonical Task State Machine

The canonical persisted V1 task states are:

```text
PLANNED
READY
RUNNING
IMPLEMENTED
VERIFYING
REPAIR
PASSED
INTEGRATING
INTEGRATED
COMPLETE

BLOCKED
RECOVERING
CANCELLED
SUPERSEDED
FAILED
```

`FAILED` means **terminal task failure after recovery/escalation is exhausted or the required work is impossible**. It is not used for an ordinary failed model call, failed test during implementation, or failed verification round.

`REPAIR` means implementation exists but verification/integration found a correctable defect. The task or a linked repair task returns to implementation from that state.

`RECOVERING` means execution ownership/session/tool/worktree state must be reconciled before useful work continues.

### Allowed transitions

| From | To | Typical trigger | Preconditions |
|---|---|---|---|
| `PLANNED` | `READY` | dependencies satisfied | active plan, no blocker, resource-independent readiness |
| `READY` | `RUNNING` | lease acquired | scheduler admission + worktree/session setup |
| `RUNNING` | `IMPLEMENTED` | Worker completion claim accepted for verification | implementation artifacts exist; no active unsafe operation |
| `RUNNING` | `RECOVERING` | worker/session/runtime interruption | durable state available |
| `RUNNING` | `BLOCKED` | external/human prerequisite | blocker object exists |
| `RUNNING` | `CANCELLED` | cancellation | cancellation policy applied |
| `IMPLEMENTED` | `VERIFYING` | verification scheduled | implementation revision fixed/pinned |
| `VERIFYING` | `PASSED` | verifier/gates pass | required task evidence fresh |
| `VERIFYING` | `REPAIR` | verifier finds correctable defect | finding persisted |
| `VERIFYING` | `FAILED` | unrecoverable verification failure | recovery exhausted/impossible |
| `REPAIR` | `RUNNING` | repair lease acquired | repair strategy exists |
| `PASSED` | `INTEGRATING` | integration admission | prerequisites passed |
| `INTEGRATING` | `INTEGRATED` | merge/reconcile succeeds | integration evidence recorded |
| `INTEGRATING` | `REPAIR` | conflict/integration regression | finding/repair task persisted |
| `INTEGRATED` | `COMPLETE` | task-level gates satisfied | task requirements/evidence accepted |
| any nonterminal | `SUPERSEDED` | plan reconciliation | replacement lineage recorded |
| any resumable state | `BLOCKED` | hard external dependency | blocker durable |
| `BLOCKED` | prior resumable state or `READY` | blocker resolved | state reconciliation |
| `RECOVERING` | `RUNNING`/`READY`/`REPAIR` | recovery success | lease/session/worktree reconciled |
| `RECOVERING` | `BLOCKED` | recovery requires user/external prerequisite | human request/blocker created |
| `RECOVERING` | `FAILED` | unrecoverable corruption/impossibility | explicit terminal reason |

Illegal examples:

```text
READY -> COMPLETE
RUNNING -> PASSED
IMPLEMENTED -> COMPLETE
VERIFYING -> INTEGRATED
```

Every transition is performed through a Kernel transition function inside a transaction that validates preconditions and appends the corresponding event.

**Mission-level `FINAL_AUDIT` is not a task state.** A task of type `FINAL_AUDIT` may exist as an execution unit, but the mission's `FINAL_AUDIT` phase remains the authoritative mission-level gate.

---

## 0.11 Canonical Mission State Machine

Canonical mission states:

```text
CREATED
ANALYZING
PLANNING
EXECUTING
VERIFYING
FINAL_AUDIT
COMPLETE

PAUSED
BLOCKED
RECOVERING
CANCELLED
FAILED
```

Mission state is a high-level orchestration phase, not the sum of task strings.

Examples:

- Mission may remain `EXECUTING` while some tasks are individually `VERIFYING`.
- Mission enters `VERIFYING` when implementation/integration work is materially complete and broad mission verification is in progress.
- Mission enters `FINAL_AUDIT` only when current requirements and integration state are ready for final independent audit.
- If Final Audit finds missing work, mission returns to `EXECUTING` or `VERIFYING` with repair tasks.
- If new user requirements are added during `FINAL_AUDIT`, the mission leaves `FINAL_AUDIT`, publishes a new requirement version, replans, and continues.
- `PAUSED`, `BLOCKED`, and `RECOVERING` preserve a `resume_phase` so the Kernel knows the valid phase after recovery.

Mission `COMPLETE` is terminal for that mission version. A later new user objective creates a new mission or explicit mission extension policy rather than mutating a completed mission invisibly.

---

## 0.12 Scheduler and Ready-Set Algorithm

The Scheduler selects **tasks**, not models.

The ready-set computation is deterministic:

```text
1. Load tasks in active plan whose status = PLANNED/READY.
2. Remove superseded/cancelled/terminal tasks.
3. Evaluate dependency predicates.
4. Evaluate blocking human/external prerequisites.
5. Evaluate active resource locks/conflict policy.
6. Evaluate repository/worktree availability.
7. Evaluate mission/task budget admission.
8. Evaluate Resource Governor admission.
9. Mark newly satisfiable tasks READY transactionally.
10. Rank READY candidates.
11. Attempt lease acquisition on highest-ranked admissible candidate.
12. Ask Model Broker for execution route after task admission.
```

Initial ranking score (`BENCHMARK_PENDING`) may combine:

```text
priority_weight
critical_path_weight
blocking_value_weight
age/fairness_weight
requirement_criticality_weight
risk_urgency_weight
resource_efficiency_weight
```

Rules:

- Explicit user priority outranks ordinary planner preference.
- Aging prevents indefinite starvation.
- Tasks that unblock many descendants receive higher blocking/critical-path value.
- Security-critical blocking repair may outrank cosmetic work.
- The Scheduler must not increase concurrency merely because more tasks are READY.
- Tie-breaking must be deterministic, e.g. score → ready timestamp → task ID.
- Scheduler state must be reconstructible after restart from durable task/dependency/resource state.

### Work queue

The "queue" is not an independent source of task truth. V1 may materialize a persistent queue table for efficiency, but `READY` state plus scheduling metadata remains authoritative.

Queue invariants:

- one logical queue entry per schedulable task;
- duplicate enqueue is idempotent;
- stale entries are ignored/reconciled after restart;
- claiming requires transactional lease acquisition;
- backpressure may delay dispatch without changing task correctness.

---

## 0.13 Concurrency and Conflict Admission

Parallel execution is permitted only when expected benefit exceeds conflict/resource risk.

Conflict inputs from Doc 02 may include:

```text
expected write scope overlap
expected read/write overlap
same symbol or package
same lockfile
same schema/migration directory
same generated output
dependency distance
shared interface still unstable
```

Conflict categories:

```text
SAFE_PARALLEL
LOW_RISK_PARALLEL
SERIALIZE
REQUIRES_CONTRACT_CHECKPOINT
EXCLUSIVE_RESOURCE
```

A task with an unknown write set is not automatically blocked, but its concurrency score is conservative until the Worker refines scope.

### Shared contract checkpoints

Parallel downstream work may begin before an upstream task fully completes only if the upstream task publishes an explicit stable contract artifact, such as:

```text
OpenAPI schema
TypeScript interface
database migration contract
protobuf message
public trait/interface
```

The contract checkpoint includes a hash/version. If the contract later changes, dependent tasks are invalidated or notified.

---

## 0.14 Resource Lock Contract

Logical locks protect shared resources that worktrees alone cannot isolate.

```text
ResourceLock {
  resource_lock_id
  mission_id
  resource_key
  mode
  owner_task_id
  owner_lease_id
  fencing_epoch
  acquired_at
  expires_at
}
```

Modes:

```text
READ
WRITE
EXCLUSIVE
```

Examples:

- `package-lock.json` → usually `EXCLUSIVE`;
- database migration sequence → `EXCLUSIVE`;
- generated client output directory → `EXCLUSIVE`;
- release configuration → `EXCLUSIVE`;
- shared architecture contract → `READ` for consumers / `WRITE` for updater.

Deadlock prevention:

1. Task declares expected resources before acquisition where possible.
2. Multiple locks are acquired in deterministic sorted `resource_key` order.
3. A task that cannot obtain the next lock releases provisional locks or waits according to policy.
4. Locks are lease-bound and fenced.
5. Expired locks cannot be renewed by a Worker whose task lease is no longer current.

---

## 0.15 Agent Runtime and Session Lifecycle

The Agent Runtime is the common execution harness for Planner, Worker, Researcher and Verifier.

Canonical session record:

```text
AgentSession {
  agent_session_id
  mission_id
  task_id?
  worker_id?
  role
  model_route_ref
  context_pack_id
  permission_profile_ref
  skill_refs[]
  status
  started_at
  last_runtime_heartbeat_at
  last_model_turn_at
  current_operation_id?
  stop_reason?
  predecessor_session_id?
}
```

Runtime session states:

```text
CREATED
STARTING
ACTIVE
WAITING_MODEL
WAITING_TOOL
CHECKPOINTING
STOPPING
STOPPED
CRASHED
```

A session state is **not** a task state.

Typical model-turn lifecycle:

```text
Kernel/Worker host requests next turn
→ Context Engine supplies/refreshes pack
→ Model Broker supplies route
→ Agent Runtime opens inference attempt
→ model returns text/tool call/structured result
→ runtime validates message
→ tool request forwarded to Tool Broker
→ tool result returned to session
→ next model turn
→ Worker emits progress/checkpoint/completion proposal
```

Cancellation must interrupt at the safest available boundary. A session may be replaced without changing task ownership if the logical Worker and lease remain valid.

---

## 0.16 Role Contracts

### Planner

Allowed:

- read mission/requirements/current DAG;
- query repository/context;
- propose plan versions, tasks, dependencies, risk, acceptance criteria;
- propose replan/supersession.

Not allowed:

- directly mutate repository files;
- mark tasks complete;
- mutate requirements without Requirement Manager validation;
- bypass Kernel plan validation.

Structured output: `PlanProposal` or `ReplanProposal`.

### Worker

Allowed:

- inspect repository;
- use assigned tools under Doc 04 policy;
- modify assigned worktree;
- run tests/build/debug tools;
- request research;
- request scope expansion;
- create checkpoints;
- submit implementation-complete claim and evidence.

Not allowed:

- mark itself `PASSED`, `INTEGRATED`, `COMPLETE`;
- modify another Worker's worktree without explicit integration task;
- bypass lease/fencing;
- silently broaden mission requirements.

### Researcher

Allowed:

- repository read/search;
- external research where policy permits;
- structured evidence and recommendation.

Normally not allowed:

- repository mutation;
- task completion authority.

### Verifier

Allowed:

- inspect actual repository state/diff;
- run deterministic checks/tests;
- consume requirement/evidence context;
- emit findings/verdict.

Normally read/test-only. It reports failure; the Kernel creates/assigns repair work.

Specializations such as Security, React, Rust, Database, Browser QA, Performance and Accessibility remain **skills/modes applied to these roles**, not permanent autonomous personas.

---

## 0.17 Logical Worker vs Agent Session

`worker_id` represents a logical execution actor assigned to a task. It can outlive a specific model session.

Example:

```text
worker-T42
  session-1 = Qwen via Provider A
       ↓ inference/provider failure
  session-2 = different model/provider
```

The same Worker identity may continue if:

- the task lease is still valid;
- worktree/checkpoint state is consistent;
- replacement is a session-level recovery;
- no competing Worker has acquired a newer lease epoch.

A new `worker_id` is created when:

- the previous Worker lease expired and the task is re-acquired after recovery;
- ownership is intentionally transferred;
- the previous worker runtime is considered invalid/zombie;
- policy wants a clean replacement actor for audit clarity.

Regardless of identity reuse, every model/runtime session is separately recorded.

---

## 0.18 Lease Acquisition, Renewal and Fencing

A lease prevents two Workers from authoritatively owning one exclusive task.

Canonical record:

```text
Lease {
  lease_id
  task_id
  worker_id
  fencing_epoch
  state
  issued_at
  last_renewed_at
  expires_at
  revoked_at?
  revoke_reason?
}
```

Lease states:

```text
ACTIVE
EXPIRED
REVOKED
RELEASED
```

### Acquisition transaction

Conceptually:

```text
BEGIN
  assert task.status == READY or authorized RECOVERING/REPAIR
  assert no ACTIVE nonexpired lease exists
  next_epoch = task.last_fencing_epoch + 1
  create lease(task, worker, next_epoch)
  set task.active_lease_id
  set task.assigned_worker_id
  set task.status = RUNNING
  append TaskLeaseAcquired + TaskRunning events
COMMIT
```

Only after commit may the Worker perform authoritative write operations.

### Fencing token

Every Worker mutation request that can affect task code/state includes:

```text
task_id
lease_id
fencing_epoch
```

The Kernel/Tool/Edit integration rejects requests whose epoch is older than the current task epoch. This prevents a zombie Worker from waking after network/process delay and committing changes after another Worker already took over.

### Renewal

Heartbeat renewal is accepted only if:

- lease is ACTIVE;
- lease ID matches task active lease;
- fencing epoch matches current task epoch;
- Worker identity matches;
- task is in a state where ownership is valid.

A stale heartbeat cannot resurrect an expired/revoked lease.

Lease TTL and heartbeat cadence are `BENCHMARK_PENDING`. Tests should use accelerated virtual timers.

---

## 0.19 Heartbeats and Long Operations

Runtime heartbeats are emitted by the Worker host/runtime process, **not by asking the LLM to say it is alive**.

Heartbeat payload:

```text
WorkerHeartbeat {
  worker_id
  task_id
  lease_id
  fencing_epoch
  agent_session_id
  runtime_status
  current_operation_id?
  operation_type?
  last_progress_watermark
  latest_checkpoint_id?
  model_route_ref?
  tool_process_ref?
  observed_at
}
```

A long-running build/test/model stream must not look dead merely because no model message has arrived. The Worker host continues heartbeat emission while the operation remains known and responsive.

Kernel distinguishes:

```text
runtime liveness
operation liveness
semantic progress
```

A Worker can be alive but stalled.

---

## 0.20 Progress Watermark

Each task maintains a progress watermark representing the latest meaningful advancement.

Progress events include:

- new relevant repository understanding that changes strategy;
- meaningful diff revision;
- test state improvement;
- new evidence;
- requirement subcriterion satisfied;
- failure resolved;
- stable contract checkpoint;
- checkpoint produced;
- scope clarification accepted.

Non-progress examples:

- rereading the same file;
- repeating the same grep;
- rerunning the same failing test without change;
- model chatter;
- repeated summaries.

Canonical progress record:

```text
ProgressObservation {
  task_id
  attempt_id
  kind
  fingerprint
  significance
  observed_at
  evidence_ref?
}
```

The task's `last_progress_at` advances only for observations meeting the configured significance threshold.

---

## 0.21 Stall and Loop Detection

Stall detection uses multiple signals rather than one timeout.

Signals:

```text
time_since_last_progress
repeated_action_fingerprint_count
same_error_fingerprint_count
same_diff_fingerprint_count
tool_calls_without_progress
model_turns_without_progress
attempts_without_requirement_advancement
```

Legitimate long operations are exempt while their operation process reports healthy progress.

Suggested escalation levels (`BENCHMARK_PENDING`):

```text
SUSPECTED
CONFIRMED
SEVERE
```

Example policy:

- `SUSPECTED`: ask Worker to self-diagnose and restate blocked hypothesis.
- `CONFIRMED`: require strategy change, broader context, alternate tool/skill/model.
- `SEVERE`: checkpoint, terminate session, invoke Planner/Researcher or replace Worker.

### Loop fingerprints

Action fingerprint should normalize insignificant differences such as timestamps while retaining semantic operation identity:

```text
tool_name + normalized_args + target_files + error_class + diff_hash
```

Detect patterns such as:

```text
A B A B A B
edit -> test fail -> revert -> same edit
grep X -> read F -> grep X -> read F
message ping-pong between two workers
```

Retry budgets cap livelock even when pattern detection misses it.

---

## 0.22 Canonical Checkpoint Contract

A checkpoint is a **recovery-consistent snapshot**, not merely a note.

```text
Checkpoint {
  checkpoint_id
  mission_id
  task_id
  attempt_id
  worker_id
  lease_id
  fencing_epoch
  checkpoint_type
  consistency_state
  worktree_ref
  git_head
  base_commit
  diff_ref?
  task_state_snapshot_ref
  context_snapshot_ref?
  handoff_packet_ref?
  test_state_ref?
  latest_failure_ref?
  failed_approach_refs[]
  important_decision_refs[]
  created_at
}
```

Consistency states:

```text
CONSISTENT_CLEAN
CONSISTENT_WITH_DIFF
DIRTY_UNSAFE
INVALID
```

Checkpoint creation should occur after any in-flight transactional edit is complete or rolled back. A checkpoint must not claim consistency while a Doc 04 ChangeSet is half-applied.

For Git-backed task worktrees, a checkpoint commit is preferred when appropriate. Uncommitted diffs may still be checkpointed as an artifact if the system can restore them reliably.

---

## 0.23 Task Attempt Model

A Task may have many attempts.

```text
TaskAttempt {
  task_attempt_id
  task_id
  ordinal
  worker_id
  starting_checkpoint_id?
  ending_checkpoint_id?
  strategy_fingerprint
  strategy_summary
  agent_session_ids[]
  route_refs[]
  context_pack_refs[]
  tool_summary_ref?
  result
  failure_ref?
  evidence_refs[]
  started_at
  ended_at?
}
```

Attempt results:

```text
SUCCEEDED_IMPLEMENTATION
INTERRUPTED_RECOVERABLE
FAILED_STRATEGY
BLOCKED
CANCELLED
UNRECOVERABLE
```

An inference call failure is normally only an event inside an attempt. A new task attempt is created when the strategy or ownership materially changes, not for every provider HTTP retry.

---

## 0.24 Failure Levels

AgentCode must distinguish failure scopes.

```text
InferenceFailure
    ↓ may be absorbed by Model Broker/runtime
AgentSessionFailure
    ↓ may be replaced under same Worker
WorkerRuntimeFailure
    ↓ may require Worker/session recovery
TaskAttemptFailure
    ↓ may trigger new attempt/strategy
VerificationFailure
    ↓ causes REPAIR
TaskFailure
    ↓ terminal after recovery exhausted/impossible
MissionFailure
    ↓ rare terminal condition
```

Canonical failure envelope:

```text
Failure {
  failure_id
  mission_id
  task_id?
  attempt_id?
  worker_id?
  session_id?
  operation_id?
  failure_class
  failure_scope
  retryability
  source_subsystem
  summary
  structured_details
  raw_evidence_ref?
  first_seen_at
  last_seen_at
  fingerprint
}
```

No model-provided string alone determines failure class. Deterministic subsystem signals are preferred.

---

## 0.25 Failure and Recovery Matrix

| Failure | Normal scope | First response | Task state effect |
|---|---|---|---|
| provider 429/quota | inference/session | Doc 01 reroute/cooldown | usually none |
| provider timeout/5xx/network | inference/session | retry/reroute | usually none |
| auth revoked | route capability | alternate credential/provider; maybe human | `BLOCKED` only if no route |
| invalid structured model output | inference turn | correction/retry/alternate model | usually none |
| premature model stop | session | resume/correct/replace session | usually none |
| context limit | session | checkpoint/compact/rebuild context | `RECOVERING` only if session must restart |
| stream truncation | inference/session | detect incomplete turn; reroute/retry safely | usually none |
| safety/content refusal | inference | alternate appropriate model if task allowed | `BLOCKED` only if policy/capability absent |
| tool timeout | operation | cancel/restart tool | Worker stays alive where possible |
| tool process crash | operation | restart/reconcile | usually none |
| command nonzero | implementation evidence | Worker reasons/debugs | no failure by itself |
| expected failing test | implementation evidence | Worker repairs | no failure by itself |
| repeated unchanged test failure | attempt | stall/retry strategy | may create new attempt |
| build failure | implementation evidence | Worker repairs | no failure by itself |
| edit transaction failure | operation | Doc 04 rollback/reconcile | `RECOVERING` if state uncertain |
| merge conflict | integration | integration repair task | `REPAIR`/integration task |
| worktree missing | task runtime | recovery/recreate/restore | `RECOVERING` |
| worktree HEAD mismatch | task runtime | reconcile against checkpoint | `RECOVERING` |
| stale/degraded code index | context | Doc 02 rebuild/fallback | usually none |
| resource exhaustion | runtime | Resource Governor pauses/reduces load | `READY`/`RECOVERING`/`BLOCKED` depending severity |
| disk full | runtime/storage | stop writes, preserve DB/Git, human action if needed | `BLOCKED` |
| SQLite transaction failure | Kernel | rollback transaction/retry safe op | no transition unless persistent |
| SQLite corruption signal | Kernel | stop scheduling, integrity/recovery flow | mission `RECOVERING`/`BLOCKED` |
| Worker process kill | worker | lease expiry/revoke, replacement | `RECOVERING` |
| daemon crash/reboot | system | boot reconciliation | mission `RECOVERING` as needed |
| user concurrent edit | repository | invalidate assumptions/preconditions | task `RECOVERING`/scope re-evaluation |
| user cancel | command | safe cancellation | `CANCELLED` |
| verifier rejects implementation | verification | persist finding, create repair | `REPAIR` |
| all recovery exhausted | task | terminal reason + possible mission replan | `FAILED` |

---

## 0.26 Recovery Engine State Machine

Recovery is deterministic orchestration around durable state.

Recovery phases:

```text
DETECTED
→ QUIESCE
→ SNAPSHOT_REALITY
→ CLASSIFY
→ RECONCILE
→ SELECT_RECOVERY
→ APPLY_RECOVERY
→ VALIDATE
→ RESUME | BLOCK | FAIL
```

### Snapshot reality

Depending on failure, collect:

- current task/attempt/lease records;
- current worker/session process state;
- worktree existence and HEAD;
- current diff;
- pending Doc 04 edit/tool operation state;
- latest checkpoint;
- code-intelligence generation/freshness;
- provider route availability from Doc 01;
- evidence freshness;
- resource locks;
- user external modifications.

### Recovery choices

```text
continue same session
replace model route inside session
start replacement session under same logical Worker
replace Worker under new lease epoch
restore/reconcile worktree
rebuild context
create focused repair task
replan
request human input
terminal fail
```

Recovery decision must be recorded in a `RecoveryRecord` with reason and evidence.

---

## 0.27 Retry Controller

Doc 01 controls retries inside a provider/inference request. Doc 03 controls retries at **session/attempt/task** level.

Retry budget hierarchy:

```text
inference request retry          -> Doc 01
agent-session correction turns   -> Agent Runtime policy
task attempt retries             -> Retry Controller
task repair rounds               -> Kernel + Verification
mission-level replan count       -> Kernel policy
```

A retry should normally change at least one of:

```text
context
strategy
model
provider
skill
tool
task decomposition
research evidence
```

Repeated identical retries are rejected after a small limit.

Retry accounting includes:

```text
attempt count
elapsed time
model tokens/cost (from Doc 01)
tool/runtime time
same-strategy count
same-failure count
verification rejection count
```

Budget exhaustion does **not** permit skipping verification or claiming partial success. If no valid route remains, create a `BLOCKED` human request or fail explicitly according to mission policy.

---

## 0.28 Provider / Model Replacement Handshake

Kernel does not choose providers.

Typical route-failure sequence:

```text
Agent Runtime detects/receives inference failure
→ Model Broker/Doc 01 classifies route availability
→ if alternate route exists:
     Broker returns replacement route
     Context Engine verifies pack compatibility
     Agent Runtime resumes/restarts session
     Kernel records route/session event
→ if route replacement requires session restart:
     checkpoint/handoff as needed
→ if no allowed route:
     Kernel evaluates budget/trust/capability policy
     BLOCKED or human escalation
```

A transient provider 429 **does not automatically require a task checkpoint** if all meaningful Worker state is already durable and the runtime can safely retry the same turn. Checkpointing is required when replacement could lose unpersisted reasoning/progress, before context compaction, before Worker/session replacement with meaningful local state, or when a task is otherwise entering recovery.

---

## 0.29 Durable Agent Communication

Canonical message:

```text
AgentMessage {
  message_id
  mission_id
  from_actor
  to_actor
  task_id?
  message_type
  priority
  payload
  dedupe_key?
  created_at
  expires_at?
  delivered_at?
  acknowledged_at?
  provenance
}
```

Recipients may be:

```text
KERNEL
TASK:<id>
WORKER:<id>
ROLE:PLANNER
ROLE:RESEARCHER
ROLE:VERIFIER
MISSION_BROADCAST
```

Messages are persisted before considered delivered. Duplicate messages with the same dedupe key are idempotent.

If a recipient Worker dies, unread task-scoped messages remain attached to the task and may be delivered to the replacement Worker.

Important mission decisions must never exist only in messages; they must be promoted to requirement/decision/plan state.

### Blackboard entry

```text
BlackboardEntry {
  blackboard_entry_id
  mission_id
  scope
  entry_type
  owner_actor
  payload
  blocking
  freshness
  dedupe_key?
  created_at
  expires_at?
  resolved_at?
}
```

Types may include:

```text
BLOCKER
CROSS_TASK_CONSTRAINT
RESEARCH_FINDING
INTEGRATION_WARNING
SECURITY_WARNING
SHARED_CONTRACT
```

The blackboard is retrieved by relevance. It is never automatically dumped into every prompt.

---

## 0.30 Canonical Event Envelope and Event Catalog

Every authoritative mutation emits a typed event.

```text
KernelEvent {
  event_id
  event_version
  event_type
  occurred_at
  mission_id?
  task_id?
  worker_id?
  session_id?
  operation_id?
  correlation_id?
  causation_id?
  payload
  redaction_class
}
```

Core event groups:

### Mission

```text
MissionCreated
MissionAnalysisStarted
MissionPlanningStarted
MissionExecutionStarted
MissionVerificationStarted
MissionFinalAuditStarted
MissionPaused
MissionResumed
MissionBlocked
MissionRecoveryStarted
MissionRecoveryCompleted
MissionCancelled
MissionFailed
MissionCompleted
```

### Requirements / planning

```text
RequirementVersionPublished
RequirementAdded
RequirementSuperseded
RequirementStatusChanged
PlanProposed
PlanRejected
PlanPublished
TaskAddedByReplan
TaskSuperseded
```

### Task / worker

```text
TaskReady
TaskLeaseAcquired
TaskRunning
TaskImplemented
TaskVerificationStarted
TaskVerificationFailed
TaskRepairStarted
TaskPassed
TaskIntegrationStarted
TaskIntegrated
TaskCompleted
TaskBlocked
TaskRecoveryStarted
TaskRecovered
TaskFailed
TaskCancelled
```

### Runtime / leases

```text
WorkerCreated
WorkerHeartbeat
WorkerStallSuspected
WorkerLoopDetected
WorkerReplaced
AgentSessionStarted
AgentSessionStopped
AgentSessionCrashed
LeaseRenewed
LeaseExpired
LeaseRevoked
ZombieWorkerRejected
```

### Recovery / checkpoint

```text
CheckpointCreated
RecoveryDecisionRecorded
WorktreeReconciled
ContextRebuilt
ProviderRouteReplaced
```

### Human / budget / resource

```text
HumanRequestCreated
HumanRequestAnswered
ResourceLockAcquired
ResourceLockReleased
ResourceAdmissionDelayed
BudgetThresholdReached
BudgetBlocked
```

Current relational state mutation and its required event append should occur in the **same SQLite transaction** whenever the event describes that authoritative state transition.

Event ordering is guaranteed by the local committed event sequence, not wall-clock timestamps alone.

---

## 0.31 Logical SQLite Control-Plane Schema

The exact physical DDL is a `CONSTRAINED_IMPLEMENTATION_DECISION`, but the logical tables and ownership are normative.

Required logical tables:

```text
missions
requirement_versions
requirements
requirement_version_members
plans
plan_task_members
tasks
task_dependencies
task_attempts
workers
agent_sessions
leases
checkpoints
messages
blackboard_entries
events
evidence_refs
human_requests
resource_locks
recovery_records
mission_budgets
notifications
idempotency_records
schema_migrations
```

Key invariants/indexes conceptually:

- `missions(mission_id)` unique primary key.
- `requirement_versions(mission_id, sequence_number)` unique.
- `plans(mission_id, sequence_number)` unique.
- `tasks(task_id)` unique; index on `(mission_id, status, priority)`.
- `task_dependencies(predecessor, successor, type)` unique.
- one active lease per task enforced transactionally/partial unique index where practical.
- `leases(task_id, fencing_epoch)` unique and monotonic.
- `workers(task_id, worker_id)` indexed.
- `agent_sessions(worker_id, status)` indexed.
- `events` indexed by mission/task/event sequence and correlation ID.
- `messages` indexed by recipient and acknowledgement status.
- resource locks indexed by resource key and active state.
- idempotency records unique by `(client_scope, idempotency_key)`.

### WAL / concurrency

V1 should use SQLite WAL mode or equivalent proven configuration to allow concurrent readers while the single authoritative Kernel writer performs short transactions.

Workers and UI clients do not write core tables directly. They call daemon/Kernel APIs.

### Migrations and integrity

- migrations run before normal scheduling begins;
- migration version is recorded;
- failed migration blocks mission execution safely;
- periodic backups/checkpoints of critical state are recommended;
- SQLite integrity failure enters controlled recovery rather than continuing with possibly poisoned mission state;
- Doc 02 derived intelligence may be rebuilt independently and must not be able to delete mission/requirement truth.

---

## 0.32 Atomic Transaction Catalog

The following operations are explicit transactional units.

### Create mission

Touches:

```text
missions
requirement version 0 or analysis placeholder
events
idempotency record
```

Emits `MissionCreated`.

### Publish requirement version

Touches:

```text
requirement_versions
requirements / supersession links
requirement_version_members
missions.current_requirement_version
events
```

### Publish plan

Touches:

```text
plans
tasks
task_dependencies
plan_task_members
missions.current_plan_version
events
```

Only after validation.

### Make task READY

Touches task state/ready timestamp and appends `TaskReady`.

### Acquire task

Touches:

```text
task
worker
lease
fencing epoch
attempt bootstrap
events
```

Atomically.

### Renew lease

Validates worker/task/lease/epoch; updates lease and heartbeat metadata.

### Checkpoint

Persists checkpoint metadata and task pointer only after checkpoint artifact consistency is confirmed.

### Revoke/release lease

Updates lease state and task ownership; emits event.

### Transition task

Validates allowed transition, required evidence/preconditions, updates timestamps, appends event.

### Add dynamic task / replan

Publishes new plan lineage rather than mutating history invisibly.

### Human request

Creates request, optionally moves task/mission to BLOCKED, emits event.

### Begin Final Audit

Pins requirement version, integration HEAD/evidence generation and mission audit inputs.

### Complete mission

Validates all deterministic gates, writes mission completion timestamp/summary reference, appends `MissionCompleted`. Notification occurs only after commit.

---

## 0.33 Daemon Singleton and Local IPC Contract

There must be at most one authoritative daemon per AgentCode state directory.

Startup:

```text
acquire instance lock
→ detect stale lock/process
→ open SQLite
→ run migrations/integrity checks
→ boot reconciliation
→ start IPC
→ publish daemon READY
```

A second client process connects to the existing daemon instead of starting another writer.

Exact transport is a `CONSTRAINED_IMPLEMENTATION_DECISION` (e.g. local domain socket, named local transport or tightly scoped localhost protocol), but the logical API is normative.

Core IPC methods:

```text
health()
get_daemon_info()

create_mission(...)
start_mission(mission_id)
get_mission(mission_id)
list_missions(...)
pause_mission(mission_id)
resume_mission(mission_id)
cancel_mission(mission_id)

add_user_instruction(mission_id, ...)
get_requirements(mission_id)
get_plan(mission_id)
get_tasks(mission_id)
get_task(task_id)

list_human_requests(mission_id)
answer_human_request(request_id, response)

subscribe_events(scope, from_sequence?)
get_event_snapshot(scope)

get_diagnostics()
```

Request envelope:

```text
protocol_version
request_id
idempotency_key?
client_id
method
payload
```

Response envelope:

```text
request_id
status
result?
error?
server_sequence?
```

Reconnecting UI follows **snapshot then stream**:

```text
fetch current mission snapshot
→ subscribe from snapshot event sequence
```

so it cannot miss events between a historical read and subscription.

Slow UI clients must not block the Kernel event writer; event streams use bounded buffers/backpressure/resume sequences.

---

## 0.34 Pause, Resume and Cancel Semantics

### Pause

Default pause is a **safe soft pause**:

1. mission enters PAUSED intent;
2. Scheduler stops new task admission;
3. active Workers are asked to reach a safe checkpoint;
4. interruptible model/tool operations may be cancelled;
5. noninterruptible local operations are allowed to finish or are handled by Doc 04 policy;
6. active leases are either retained for a bounded pause grace period or safely released according to checkpoint state;
7. state is persisted.

An emergency stop may exist later for urgent security/destructive conditions, but it must be clearly distinct from ordinary pause.

### Resume

Resume always reconciles first:

```text
DB/migration/integrity
repository/worktree state
pending edits/tools
external modifications
Doc 02 index freshness
leases/workers
provider/model capability availability
evidence freshness
resource locks
human blockers
```

Only then does the mission return to its resumable phase and dispatch work.

### Cancel

Cancellation is idempotent.

Default behavior:

- stop scheduling;
- request active sessions/tools to terminate safely;
- checkpoint recoverable work;
- release resource locks/leases;
- preserve code changes and Git worktrees;
- mark remaining nonterminal tasks cancelled/superseded as policy dictates;
- mark mission `CANCELLED`.

Cancellation does **not** automatically roll back useful repository changes. Destructive rollback requires explicit user instruction/policy.

---

## 0.35 External Repository Modification

The repository is not exclusively owned by AgentCode.

Doc 02 may emit an external-change event. Kernel response:

```text
detect changed paths/symbols
→ identify impacted running tasks
→ invalidate stale scope/context/evidence
→ Doc 04 precondition protection prevents stale overwrite
→ decide:
    continue (unrelated change)
    refresh context
    pause/recover task
    request scope expansion/replan
    block on conflict
```

A user edit always outranks stale Worker assumptions. AgentCode must not silently overwrite it merely because the Worker started earlier.

---

## 0.36 Human Escalation State Machine

Canonical request:

```text
HumanRequest {
  human_request_id
  mission_id
  task_id?
  request_type
  severity
  blocking
  reason
  context_summary
  options[]
  recommended_option?
  security_or_cost_impact?
  status
  dedupe_key?
  created_at
  expires_at?
  answered_at?
  response?
  applied_at?
}
```

Statuses:

```text
OPEN
ANSWERED
APPLIED
CANCELLED
EXPIRED
```

Kernel should deduplicate equivalent open requests so the user is not spammed.

Common valid escalation reasons:

```text
missing authentication/credential
irreversible external mutation
ambiguous user intent with material consequence
paid budget exhausted
legal/licensing decision
production/security authorization
all autonomous recovery exhausted
```

Bad escalation:

```text
"Tests failed. What should I do?"
```

when the Worker can inspect and repair them autonomously.

A blocking request moves the relevant task/mission to `BLOCKED`; nonblocking requests do not prevent unrelated tasks from continuing.

---

## 0.37 Resource Governor and Admission

The Kernel Resource Governor coordinates mission-level resource admission. Doc 02 owns code-intelligence internal resource decisions; Doc 04 owns tool/process details.

Resource classes:

```text
CPU_HEAVY
MEMORY_HEAVY
LOCAL_MODEL
BROWSER
LSP_HEAVY
INDEX_HEAVY
BUILD_HEAVY
TEST_HEAVY
NETWORK_HEAVY
DISK_HEAVY
```

Admission record:

```text
ResourceReservation {
  task_id
  resource_class
  amount_or_slot
  admitted_at
  released_at?
}
```

Pressure states:

```text
NORMAL
PRESSURE
CRITICAL
```

Under pressure the Kernel may:

- stop admitting new heavy tasks;
- reduce parallel Workers;
- ask Doc 02 to pause optional indexing;
- avoid loading another local model;
- serialize browser/build/test work;
- preserve currently safe task state.

Initial V1 default on the 8 GB target should be conservative, approximately **two cloud-backed Worker tasks at once**, with local-model/browser/heavy-build concurrency further restricted. Exact numbers are `BENCHMARK_PENDING`.

---

## 0.38 Mission and Task Budgets

Kernel tracks mission/task policy limits, while Doc 01 remains authoritative for model/provider cost accounting.

Possible mission budget fields:

```text
max_paid_spend
max_duration
max_parallel_workers
max_task_attempts_default
max_replans
provider_trust_policy_ref
paid_fallback_allowed
```

Possible task budget fields:

```text
max_attempts
max_elapsed_time
max_paid_spend_share
max_verification_rounds
```

Budget thresholds are not an excuse to reduce acceptance quality. If the remaining budget cannot satisfy required verification, the mission becomes `BLOCKED` or requests user action; it never silently skips gates.

---

## 0.39 Scope Drift and Scope Expansion

Task scope is represented using:

```text
expected_read_scope
expected_write_scope
requirement_refs
impact graph
repository/package boundaries
```

Unexpected access outside scope increases scope-drift suspicion.

A Worker may submit:

```text
ScopeExpansionRequest {
  task_id
  reason
  discovered_dependency
  proposed_read_scope[]
  proposed_write_scope[]
  affected_requirements[]
  whether_new_task_preferred
}
```

Kernel evaluates:

- whether expansion is necessary to satisfy existing requirement;
- conflict with parallel tasks;
- resource/lock implications;
- whether Planner should add a separate task;
- whether expansion actually changes mission scope and therefore requirements.

No Worker silently changes mission obligations.

---

## 0.40 Verification and Repair Handshake

Doc 05 owns verification mechanics. Doc 03 owns state coordination.

Canonical sequence:

```text
Worker submits implementation-complete proposal
→ Kernel validates artifacts/checkpoint
→ task RUNNING -> IMPLEMENTED
→ Kernel schedules independent verification
→ task -> VERIFYING
→ Verification Engine returns structured result
     PASS:
       task -> PASSED
     FAIL_CORRECTABLE:
       finding persisted
       task -> REPAIR
       repair Worker/task scheduled
     FAIL_UNRECOVERABLE:
       task recovery/replan or terminal FAILED
```

Worker self-report is never verification.

Verifier diversity preference is delegated to Doc 01.

Evidence freshness changes from Doc 02/05 may move a previously passed/integrated task back into a verification-required state before mission completion.

---

## 0.41 Integration Coordination

`PASSED` means the task is correct in its isolated verified task view.

`INTEGRATED` means its changes have been reconciled into the mission integration target and survived required post-integration checks.

Flow:

```text
PASSED
→ integration admission
→ Doc 04 merge/reconcile
→ integration checks
→ INTEGRATED
→ task-level completion checks
→ COMPLETE
```

Merge conflict or integration regression:

```text
INTEGRATING
→ REPAIR
```

or creates a dedicated integration-repair task linked to affected tasks.

Task `COMPLETE` does **not** imply mission completion. Mission Final Audit remains separate.

Integration may invalidate earlier evidence when combined changes alter covered code. Doc 05 determines re-verification requirements; Kernel coordinates the resulting state.

---

## 0.42 Mission Final Audit and Completion Transaction

Final Audit is mission-level.

Pinned inputs:

```text
mission original_goal
current requirement_version
current plan_version
integration repository HEAD/view
combined mission diff
fresh requirement evidence
test/build/typecheck state
security state if applicable
accepted risks
approved out-of-scope items
known limitations
```

Final Audit must be performed against a stable integration state. If repository HEAD changes during audit, audit evidence becomes stale and must rerun or reconcile.

A Final Audit failure:

```text
persist finding
→ leave FINAL_AUDIT
→ create/activate repair work
→ reverify/integrate
→ rerun Final Audit
```

### Completion transaction

Before `COMPLETE`, Kernel deterministically checks:

1. every mandatory active requirement is `VERIFIED`, `ACCEPTED_RISK`, or `APPROVED_OUT_OF_SCOPE`;
2. every required task is terminal and accepted;
3. no active task/lease/repair remains;
4. dependency graph is satisfied;
5. integration target is current and stable;
6. required test/build/lint/typecheck evidence is fresh;
7. required security/verification findings contain no blocker;
8. Final Audit is PASS and fresh for the current integration state;
9. required artifacts/reports exist.

Only then:

```text
BEGIN
  set mission.status = COMPLETE
  set completed_at
  persist completion summary reference
  append MissionCompleted
COMMIT
```

Only **after commit** may the notification subsystem announce completion.

---

## 0.43 Evidence References

Doc 05 owns evidence schema/content. Kernel stores references sufficient for state gating:

```text
EvidenceRef {
  evidence_ref
  mission_id
  task_id?
  evidence_type
  status
  freshness
  repository_revision_ref?
  verification_policy_ref?
  produced_at
}
```

Kernel does not reinterpret a test report or security scan itself. It evaluates the structured verification status supplied by the responsible subsystem and checks freshness/required presence.

---

## 0.44 Security and Trust Boundaries

All LLM output is untrusted proposal data until validated.

The Kernel must defend against:

- malformed task/plan IDs;
- forged `worker_id`;
- stale/forged lease IDs;
- replayed completion messages;
- payloads attempting direct state changes;
- prompt injection in repository content from Doc 02;
- provider output containing fake tool/state commands;
- a Worker claiming another task's evidence;
- duplicate/replayed IPC commands;
- oversized payload/resource abuse.

State-mutating APIs validate:

```text
actor identity
mission/task membership
current lease/fencing epoch where relevant
allowed state transition
payload schema/version
idempotency key
policy
```

Tool permissions remain Doc 04. The Kernel never treats a model-generated string like `"set task complete"` as an authoritative command.

---

## 0.45 Idempotency, Replay and Fencing

Exactly-once execution is not assumed.

Instead AgentCode uses:

```text
transactional claims
idempotency keys
operation IDs
fencing tokens
reconciliation
```

Examples:

- duplicate `start_mission` with same idempotency key returns original result;
- duplicate task-assignment request cannot produce a second active lease;
- duplicate event delivery to UI does not mutate Kernel state;
- a Worker with fencing epoch 7 cannot write after epoch 8 was issued;
- daemon restart may safely retry `publish plan` only through idempotency protection;
- non-idempotent external tool operations are reconciled through Doc 04 and are never blindly replayed.

---

## 0.46 Boot / Crash / Power-Loss Reconciliation

After unclean daemon shutdown, normal scheduling remains disabled until reconciliation completes.

Order:

```text
1. acquire daemon singleton lock
2. open SQLite
3. verify schema/migrations
4. run integrity checks required by policy
5. identify unfinished authoritative transactions/records
6. load nonterminal missions
7. inspect RUNNING/RECOVERING tasks
8. inspect active leases and fencing epochs
9. inspect Worker/session process reality
10. inspect pending Doc 04 tool/edit operations
11. inspect worktrees and checkpoint HEADs
12. inspect external repository changes
13. refresh/validate Doc 02 repository generations
14. invalidate stale verification evidence
15. reconcile resource locks
16. refresh route capability via Doc 01
17. produce RecoveryRecords
18. move missions/tasks to safe resumable states
19. enable scheduler
```

Example reconciliation decisions:

```text
DB says worker active; process absent; lease expired; worktree valid
→ revoke lease
→ task RECOVERING
→ create replacement Worker

DB says checkpoint commit X; worktree HEAD Y with unknown diff
→ do not overwrite
→ recovery investigation / preserve diff

DB says mission COMPLETE but completion event missing
→ integrity anomaly; do not send notification until reconciled

DB says task RUNNING; Doc 04 has incomplete ChangeSet
→ resolve ChangeSet before Worker replacement
```

---

## 0.47 Notifications

Notifications are downstream of committed Kernel state.

Default user-facing classes:

```text
MISSION_COMPLETE
NEEDS_USER / BLOCKED
CRITICAL_FAILURE
```

Ordinary events that should remain silent by default:

```text
provider reroute
worker session replacement
LSP restart
test failure during implementation
automatic retry
checkpoint
```

The system may display them in the timeline, but they should not interrupt the user.

---

## 0.48 Observability and Metrics

Structured logs/events use mission/task/worker/session/operation correlation IDs and redact secrets.

Kernel metrics:

```text
active_missions
ready_task_count
running_task_count
blocked_task_count
queue_wait_time
task_run_time
task_verification_time
task_integration_time
attempts_per_task
recovery_count
recovery_success_rate
worker_replacement_count
lease_expiry_count
heartbeat_lag
stall_incident_count
loop_incident_count
replan_count
dynamic_task_count
human_intervention_count
resource_admission_delay
checkpoint_age
integration_conflict_count
final_audit_rejection_count
false_completion_rejection_count
mission_duration
```

Cross-subsystem joined metrics:

```text
provider_switch_count            // Doc 01 events
tokens/cost_per_verified_task    // Doc 01 + Doc 05
context_pack_size                // Doc 02
tool failure/retry counts        // Doc 04
verification rejection rate      // Doc 05
```

Metrics are for diagnosis/tuning, not direct completion authority.

---

## 0.49 Benchmark Protocol

Core Kernel state-machine behavior must be testable deterministically without live LLM/provider dependencies.

### Deterministic fixtures

Create simulated:

- Mission/Requirement/Plan records;
- fake Workers;
- fake Model Broker responses;
- virtual clock;
- fault-injected Worker processes;
- fake worktree/repository adapters;
- fake verification engine;
- resource pressure simulator.

### Required benchmark groups

1. **Scheduler**
   - dependency ordering;
   - fairness;
   - critical-path preference;
   - conflict serialization;
   - resource admission.

2. **Lease / ownership**
   - two-Worker race;
   - expiry;
   - renewal;
   - fencing zombie rejection;
   - daemon restart.

3. **Recovery**
   - Worker kill;
   - session replacement;
   - provider failure;
   - context exhaustion;
   - worktree mismatch;
   - user concurrent edit.

4. **Planning**
   - cycle rejection;
   - new requirement;
   - replan;
   - task supersession;
   - evidence preservation.

5. **Completion**
   - false Worker done claim;
   - failed verifier;
   - stale evidence;
   - incomplete integration;
   - Final Audit failure;
   - successful completion transaction.

6. **Daemon**
   - UI disconnect/reconnect;
   - daemon crash;
   - system restart simulation;
   - duplicate commands;
   - multiple clients.

Model-dependent planner/worker quality benchmarks should use repeated trials. Deterministic control-plane acceptance tests should not depend on probabilistic model success.

---

## 0.50 Hardened V1 Scope Classification

### `REQUIRED_V1`

```text
background daemon
singleton lock + IPC
SQLite control state
mission/requirement/plan/task records
requirement versioning
task DAG
scheduler/work queue
Planner/Worker/Researcher/Verifier runtimes
logical Worker/session separation
leases + fencing
heartbeats
progress/stall/loop handling
task attempts
checkpoints
retry/recovery
provider/model replacement coordination
messages + event log
resource locks/governor
dynamic tasks + replanning
pause/resume/cancel
human escalation
integration coordination
verification coordination
mission Final Audit
deterministic completion gate
background completion notification interface
```

### `OPTIONAL_V1`

```text
local Sentinel for summaries
advanced mission templates
advanced speculative task scheduling
some richer blackboard conveniences
```

### `BENCHMARK_PENDING`

```text
heartbeat interval
lease TTL
stall/loop thresholds
scheduler score weights
default parallel worker count
resource pressure thresholds
checkpoint frequency
retry limits by task type
```

### `POST_V1`

```text
distributed/remote Kernel
remote Worker fleet
organization collaboration
learned scheduler
mission forking product UX
cross-machine lease consensus
autonomous multi-user arbitration
```

---

## 0.51 Hardened Acceptance-Test Catalog

The following test IDs are the minimum Doc 03 subsystem validation set. Doc 10 may reference these IDs later.

### Mission / requirement

| ID | Scenario | Expected |
|---|---|---|
| `KRN-MIS-001` | create mission | immutable original goal persisted |
| `KRN-MIS-002` | attempt to overwrite original goal | rejected |
| `KRN-REQ-001` | add explicit requirement | new requirement version published |
| `KRN-REQ-002` | contradictory new user instruction | old requirement superseded; new explicit requirement active |
| `KRN-REQ-003` | Planner tries to silently remove explicit requirement | plan rejected |
| `KRN-REQ-004` | requirement accepted risk without authorization | rejected |

### Planning / DAG

| ID | Scenario | Expected |
|---|---|---|
| `KRN-PLN-001` | valid task DAG | plan published |
| `KRN-PLN-002` | dependency cycle | plan rejected |
| `KRN-PLN-003` | missing dependency target | plan rejected |
| `KRN-PLN-004` | Planner process dies | replacement can reconstruct from durable state |
| `KRN-PLN-005` | replan supersedes task | old task preserved as SUPERSEDED |
| `KRN-PLN-006` | completed task survives replan | valid evidence retained |

### Scheduler / concurrency

| ID | Scenario | Expected |
|---|---|---|
| `KRN-SCH-001` | dependent task before predecessor | not scheduled |
| `KRN-SCH-002` | independent tasks | may run in parallel |
| `KRN-SCH-003` | overlapping lockfile tasks | serialized |
| `KRN-SCH-004` | low-priority task ages | eventually scheduled |
| `KRN-SCH-005` | critical-path task unlocks many descendants | receives preference |
| `KRN-SCH-006` | memory pressure | new heavy task admission delayed |
| `KRN-SCH-007` | duplicate queue entry | one execution only |

### Lease / Worker

| ID | Scenario | Expected |
|---|---|---|
| `KRN-LSE-001` | two Workers race for one task | exactly one active lease |
| `KRN-LSE-002` | heartbeat stops | lease expires/recovery begins |
| `KRN-LSE-003` | old Worker resumes after replacement | fenced write rejected |
| `KRN-LSE-004` | valid heartbeat | lease renews |
| `KRN-LSE-005` | stale heartbeat with old epoch | ignored/rejected |
| `KRN-WRK-001` | model/provider session replaced | task state preserved |
| `KRN-WRK-002` | Worker process killed | replacement resumes from durable state |

### Progress / livelock

| ID | Scenario | Expected |
|---|---|---|
| `KRN-PRG-001` | repeated grep/read loop | loop suspicion escalates |
| `KRN-PRG-002` | repeated same failed patch/test cycle | strategy change/replacement |
| `KRN-PRG-003` | long healthy build | not falsely marked stalled |
| `KRN-PRG-004` | many tool calls with no state advance | stall detected |

### Checkpoint / recovery

| ID | Scenario | Expected |
|---|---|---|
| `KRN-CHK-001` | checkpoint with clean worktree | consistent checkpoint |
| `KRN-CHK-002` | checkpoint during incomplete edit transaction | rejected/deferred |
| `KRN-REC-001` | provider 429 | reroute without task failure |
| `KRN-REC-002` | context exhaustion | compact/rebuild and continue |
| `KRN-REC-003` | tool crash | restart/recover without mission loss |
| `KRN-REC-004` | worktree missing | task RECOVERING; no blind overwrite |
| `KRN-REC-005` | user concurrent edit | stale assumptions invalidated |
| `KRN-REC-006` | daemon crash | boot reconciliation restores safe state |
| `KRN-REC-007` | simulated power loss | mission/task/Git state recoverable |
| `KRN-REC-008` | disk full | safe BLOCKED state; no silent corruption |
| `KRN-REC-009` | SQLite transaction error | transaction rollback; consistent state |
| `KRN-REC-010` | all providers unavailable | BLOCKED/human request per policy |

### Retry / attempt

| ID | Scenario | Expected |
|---|---|---|
| `KRN-RTY-001` | identical failed strategy repeated | bounded; requires strategy change |
| `KRN-RTY-002` | alternate context/model succeeds | new attempt recorded |
| `KRN-RTY-003` | retry budget exhausted | no infinite loop; block/fail/replan |

### Communication / idempotency

| ID | Scenario | Expected |
|---|---|---|
| `KRN-MSG-001` | Worker dies with unread task message | replacement can receive |
| `KRN-MSG-002` | duplicate dedupe-key message | one logical message |
| `KRN-IDM-001` | duplicate start-mission command | one mission start |
| `KRN-IDM-002` | replayed stale state mutation | rejected |
| `KRN-IDM-003` | malicious Worker sends forged task ID | rejected |
| `KRN-IDM-004` | malicious Worker uses stale lease epoch | rejected |

### Pause / resume / cancel

| ID | Scenario | Expected |
|---|---|---|
| `KRN-CTL-001` | pause during active Worker | checkpoint/safe stop; no new tasks |
| `KRN-CTL-002` | resume after external edit | reconcile before scheduling |
| `KRN-CTL-003` | cancel mission | workers stopped; changes preserved by default |
| `KRN-CTL-004` | duplicate cancel | idempotent |

### Replanning / dynamic discovery

| ID | Scenario | Expected |
|---|---|---|
| `KRN-RPL-001` | Worker discovers missing frontend consumer | dynamic task proposed/inserted |
| `KRN-RPL-002` | new security finding | plan/requirement reconciliation |
| `KRN-RPL-003` | task superseded | history preserved |

### Verification / integration / completion

| ID | Scenario | Expected |
|---|---|---|
| `KRN-VER-001` | Worker claims done without evidence | task not passed |
| `KRN-VER-002` | Verifier fails implementation | task enters REPAIR |
| `KRN-VER-003` | repair passes | task reverified |
| `KRN-INT-001` | passed task merges cleanly | INTEGRATED after checks |
| `KRN-INT-002` | merge conflict | integration repair path |
| `KRN-INT-003` | integration invalidates old evidence | evidence stale/reverification |
| `KRN-FIN-001` | Final Audit finds missing requirement | mission leaves FINAL_AUDIT for repair |
| `KRN-FIN-002` | stale test evidence | mission completion rejected |
| `KRN-FIN-003` | all gates satisfied | atomic COMPLETE transition |
| `KRN-FIN-004` | notification requested before commit | suppressed |
| `KRN-FIN-005` | completion commit succeeds | completion notification emitted |

### Daemon / IPC

| ID | Scenario | Expected |
|---|---|---|
| `KRN-DMN-001` | close desktop UI | mission continues |
| `KRN-DMN-002` | reopen UI | snapshot + event stream reconnect |
| `KRN-DMN-003` | start second daemon | connects/refuses second writer |
| `KRN-DMN-004` | stale instance lock | safely detected/recovered |
| `KRN-DMN-005` | slow event subscriber | Kernel not blocked |

### Resource / budget / human escalation

| ID | Scenario | Expected |
|---|---|---|
| `KRN-RES-001` | local-model slot occupied | second heavy local model delayed |
| `KRN-BUD-001` | paid budget exhausted | no silent paid call; BLOCKED/request |
| `KRN-HUM-001` | missing credential | actionable deduplicated human request |
| `KRN-HUM-002` | routine test failure | no human request |
| `KRN-HUM-003` | user answers request | state updated and mission resumes appropriately |

Passing these tests does not by itself prove model quality. It proves the autonomy control plane behaves correctly under the conditions that make AgentCode meaningfully autonomous.

---

---

# 1. Purpose

AgentCode is intended to support a simple user experience:

```text
choose repository
        ↓
describe goal
        ↓
start mission
        ↓
leave AgentCode running
        ↓
return when genuinely complete
```

The user should not need to:

- repeatedly approve harmless commands;
- restart failed agents;
- manually switch providers;
- tell an agent to continue;
- repeatedly remind the system of the original task;
- re-explain completed work;
- babysit long coding sessions;
- decide which worker should handle which task;
- manually recover from context exhaustion;
- manually restart work after a model quota failure.

The AgentCode Autonomy Kernel exists to make that possible.

Its core responsibility is:

> **Maintain durable control over a software-engineering mission independently of any individual LLM, provider, process, UI session or worker.**

---

# 2. Fundamental Principle

The Kernel is the source of truth for mission execution.

Not:

```text
Planner model
```

Not:

```text
Worker model
```

Not:

```text
Verifier model
```

Not:

```text
OmniRoute
```

Not:

```text
desktop UI
```

The Kernel owns:

```text
mission truth
task truth
dependency truth
worker ownership
execution state
retry state
verification state
completion state
```

LLMs are replaceable workers operating under Kernel control.

---

# 3. Kernel Authority

Only the Kernel may perform authoritative state transitions such as:

```text
TASK_READY

TASK_RUNNING

TASK_IMPLEMENTED

TASK_VERIFIED

TASK_FAILED

TASK_RETRYING

TASK_COMPLETE

MISSION_COMPLETE
```

A model response saying:

```text
"Everything is done."
```

must never directly produce:

```text
MISSION_COMPLETE
```

Instead:

```text
model claims done
      ↓
Kernel records claim
      ↓
verification begins
      ↓
acceptance gates evaluated
      ↓
Kernel decides state
```

---

# 4. Relationship to Docs 01 and 02

Doc 01 determines:

```text
WHO SHOULD PERFORM THE TASK?
```

Doc 02 determines:

```text
WHAT INFORMATION SHOULD THEY RECEIVE?
```

Doc 03 determines:

```text
WHAT TASK EXISTS?

WHEN SHOULD IT RUN?

WHAT DOES IT DEPEND ON?

WHAT HAPPENS IF IT FAILS?

WHEN IS IT ACTUALLY COMPLETE?
```

Combined:

```text
                       USER GOAL
                           │
                           ▼
                   AUTONOMY KERNEL
                           │
                    task specification
                           │
             ┌─────────────┴─────────────┐
             ▼                           ▼
       MODEL BROKER                 CONTEXT ENGINE
       Doc 01                       Doc 02
             │                           │
             └─────────────┬─────────────┘
                           ▼
                     AGENT RUNTIME
                           │
                           ▼
                        TOOLS
                           │
                           ▼
                       EVIDENCE
                           │
                           ▼
                       KERNEL
```

---

# 5. Primary V1 Components

The Autonomy subsystem consists of:

1. AgentCode Background Daemon
2. Kernel State Store
3. Mission Manager
4. Goal Interpreter
5. Requirement Manager
6. Requirement Matrix
7. Planning Engine
8. Task DAG Manager
9. Scheduler
10. Work Queue
11. Agent Runtime
12. Planner Runtime
13. Worker Runtime
14. Researcher Runtime
15. Verifier Runtime
16. Skill Assignment Layer
17. Worker Registry
18. Worker Lease Manager
19. Heartbeat Manager
20. Checkpoint Manager
21. Recovery Engine
22. Retry Controller
23. Failure Classifier
24. Provider/Model Replacement Coordinator
25. Task Evidence Store
26. Task Communication System
27. Event Bus
28. Append-Only Event Log
29. Concurrency Manager
30. Resource Governor
31. Worktree Coordination Interface
32. Human Escalation Manager
33. Mission Pause/Resume System
34. Mission Completion Gate
35. Mission Summary Generator
36. Notification Interface
37. Kernel Observability Layer

---

# 6. Background Daemon

AgentCode's mission runtime must not depend on the desktop window remaining open.

The architecture should therefore contain a persistent local daemon:

```text
AgentCode Desktop UI
        │
        ▼
AgentCode Daemon
        │
        ▼
Autonomy Kernel
```

If the user:

```text
closes window
minimizes app
switches desktop
locks screen
```

the mission continues where technically possible.

The UI is a client of the Kernel.

It is not the Kernel itself.

---

# 7. Daemon Responsibilities

The daemon owns or coordinates:

```text
Kernel process

mission state

task queues

worker processes

provider routing

worktree lifecycle

tool processes

background indexing

notifications

event streams

checkpoint persistence
```

The daemon should start only one authoritative Kernel instance per AgentCode state directory.

---

# 8. Daemon Crash Recovery

If AgentCode itself crashes:

```text
process dies
     ↓
user relaunches AgentCode
     ↓
daemon reads persistent state
     ↓
detect incomplete missions
     ↓
reconcile worker/process reality
     ↓
recover expired leases
     ↓
resume safe tasks
```

A mission must not disappear because:

```text
desktop app crashed.
```

---

# 9. Mission

A **Mission** is the highest-level executable unit in AgentCode.

Example:

```text
"Make this repository production-ready.
Fix all current issues, wire incomplete features,
run all tests, audit security and do not stop
until every acceptance criterion passes."
```

Conceptually:

```text
mission_id

repo_ids

original_goal

created_at

status

priority

risk_class

budget_policy

autonomy_policy

requirements_version

plan_version
```

---

# 10. Immutable Original Goal

The original user goal must be stored verbatim.

Example:

```text
original_goal
```

It must never be replaced by a planner-generated summary.

A planner may create:

```text
goal interpretation

plan

requirements

tasks
```

but the original request remains permanently available.

This prevents long-running missions from slowly drifting away from user intent.

---

# 11. Mission Goal Layers

AgentCode distinguishes:

## Layer 1 — Original Goal

Immutable user input.

## Layer 2 — Interpreted Goal

Structured Kernel interpretation.

## Layer 3 — Requirements

Concrete expected outcomes.

## Layer 4 — Tasks

Executable work units.

## Layer 5 — Evidence

Proof of completion.

The hierarchy is:

```text
ORIGINAL GOAL
      ↓
REQUIREMENTS
      ↓
TASKS
      ↓
IMPLEMENTATION
      ↓
EVIDENCE
```

---

# 12. Requirement Extraction

Before implementation, AgentCode should convert the goal into explicit requirements.

Example:

```text
R1 Authentication works correctly

R2 Password reset works

R3 Existing sessions invalidate correctly

R4 Build passes

R5 Unit tests pass

R6 No known high-severity vulnerabilities remain
```

Requirements should be:

```text
specific
traceable
verifiable
```

rather than vague summaries.

---

# 13. Requirement Types

Possible types:

```text
FUNCTIONAL

NON_FUNCTIONAL

QUALITY

PERFORMANCE

SECURITY

UX

COMPATIBILITY

INFRASTRUCTURE

DOCUMENTATION

USER_CONSTRAINT
```

---

# 14. Requirement Representation

Conceptually:

```text
requirement_id

mission_id

description

type

priority

source

verification_method

status

evidence_refs

blocking
```

---

# 15. Requirement Source

Each requirement must record its origin.

Examples:

```text
USER_EXPLICIT

USER_IMPLIED

ARCHITECTURE_DOC

PLANNER_DERIVED

SECURITY_FINDING

TEST_FAILURE

EXTERNAL_CONSTRAINT
```

A planner-derived requirement should never silently override an explicit user requirement.

---

# 16. Requirement Matrix

The Kernel maintains a live Requirement Matrix.

Example:

```text
R1 Login              IMPLEMENTED    VERIFIED
R2 Logout             IMPLEMENTED    VERIFIED
R3 Session expiry     IMPLEMENTED    UNVERIFIED
R4 Responsive UI      NOT_STARTED    UNVERIFIED
R5 Build passes       PASS           VERIFIED
```

Mission completion is impossible while mandatory requirements remain:

```text
NOT_STARTED

IN_PROGRESS

FAILED

UNVERIFIED

BLOCKED
```

---

# 17. Requirement Versioning

Requirements may evolve after:

```text
new user instruction

research discovery

security discovery

architecture decision

new dependency constraint
```

Changes must produce:

```text
requirements_version
```

Existing task plans must be reconciled against the new version.

---

# 18. Goal Interpretation

The initial goal interpreter should classify:

```text
scope

risk

likely task categories

repository coverage

verification needs

required skills

potential destructive operations

unknowns
```

The interpreter is advisory.

The Kernel stores the resulting structured mission.

---

# 19. Planning

The Planner converts mission requirements into a Task DAG.

The Planner should not directly perform implementation unless explicitly operating in Worker mode.

Responsibilities:

```text
decompose mission

identify prerequisites

identify parallelizable work

identify verification tasks

identify research needs

identify high-risk operations

estimate context needs

identify completion gates
```

---

# 20. Planner Is Replaceable

Planning state must not exist only inside a Planner conversation.

The Planner outputs structured state.

Example:

```text
plan_version

tasks

dependencies

acceptance criteria

risk

task types

suggested skills
```

If the Planner dies:

```text
new Planner
     ↓
reads mission state
     ↓
reads requirements
     ↓
reads current DAG
     ↓
continues/replans
```

---

# 21. Task DAG

Tasks form a directed acyclic graph where practical.

Example:

```text
                 T1 Repository Audit
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
      T2 Auth      T3 Payments    T4 UI
          │            │            │
          ▼            ▼            ▼
      T5 Tests      T6 Tests      T7 QA
          └────────────┼────────────┘
                       ▼
                  T8 Final Audit
```

Dependencies prevent invalid execution ordering.

---

# 22. Task Model

Conceptually:

```text
task_id

mission_id

title

description

type

status

priority

risk

dependencies

acceptance_criteria

required_skills

required_tools

required_context_profile

estimated_scope

assigned_worker

lease_id

attempt_count

max_attempts

checkpoint_id

evidence_refs

created_at

updated_at
```

---

# 23. Task Types

Possible types:

```text
PLAN

IMPLEMENT

RESEARCH

DEBUG

REFACTOR

TEST

VERIFY

SECURITY

DESIGN

MIGRATION

DOCUMENTATION

INTEGRATION

FINAL_AUDIT
```

Task type influences:

```text
role
model selection
context profile
tools
verification
```

---

# 24. Task State Machine

The canonical V1 state machine is defined normatively in **Section 0.10** of this hardened revision.

The persisted task lifecycle is:

```text
PLANNED
   ↓
READY
   ↓
RUNNING
   ↓
IMPLEMENTED
   ↓
VERIFYING
   ├──────────────→ REPAIR ──────────────┐
   │                                      │
   ▼                                      │
PASSED                                    │
   ↓                                      │
INTEGRATING                               │
   ├──────────────→ REPAIR ───────────────┘
   ▼
INTEGRATED
   ↓
COMPLETE
```

Cross-cutting nonterminal states:

```text
BLOCKED
RECOVERING
```

Terminal non-success states:

```text
CANCELLED
SUPERSEDED
FAILED
```

`FAILED` is reserved for a terminal task failure after recovery/replanning options are exhausted or the required task is impossible. An inference error, failed test during implementation, Worker crash, or failed verification round is **not** by itself `TASK_FAILED`.

`FINAL_AUDIT` is a **mission-level phase**, not a task lifecycle state. A task of type `FINAL_AUDIT` may execute work for that phase, but the Kernel's mission-level Final Audit and completion transaction remain authoritative.

---

# 25. Strict State Transitions

State transitions should be validated.

For example:

```text
READY
→ COMPLETE
```

must normally be illegal.

Likewise:

```text
RUNNING
→ VERIFIED
```

without implementation/evidence stages should be rejected.

---

# 26. Scheduler

The Scheduler selects READY tasks whose dependencies are satisfied.

Inputs include:

```text
priority

dependency state

risk

available workers

provider capacity

resource constraints

repository conflicts

worktree availability

paid budget

human approval constraints
```

---

# 27. Scheduling Principle

The Scheduler should maximize safe progress.

Not simply:

```text
run everything in parallel.
```

Concurrency should be used only when tasks can safely execute independently.

---

# 28. Parallelization Eligibility

Tasks are strong candidates for parallel execution when:

```text
different subsystems

separate files

separate worktrees

no shared migration

no shared generated artifact

no dependency ordering
```

Tasks should serialize when:

```text
one depends on another

both modify same critical files

shared database migration

shared lockfile update

high merge-conflict probability
```

---

# 29. Conflict Prediction

Before scheduling tasks concurrently, AgentCode should estimate potential conflicts using Doc 02 intelligence.

Signals:

```text
overlapping target files

shared symbols

same package

dependency distance

shared schema

shared config

shared generated files
```

High-conflict tasks should not be parallelized unnecessarily.

---

# 30. Concurrency Manager

AgentCode should support configurable concurrency.

Conceptually:

```text
max_parallel_workers

max_parallel_cloud_calls

max_parallel_local_models

max_parallel_worktrees

max_parallel_heavy_commands
```

Defaults should be conservative on the target Mac.

---

# 31. Concurrency on an 8 GB Mac

The system should avoid:

```text
4 local LLMs

3 language servers

2 builds

browser

Zoekt

multiple large test suites
```

all consuming memory simultaneously.

Resource coordination is mandatory.

---

# 32. Kernel Resource Governor

The Kernel coordinates with Doc 02's resource governor.

It tracks:

```text
RAM pressure

CPU pressure

disk pressure

network availability

local-model occupancy

browser processes

test/build processes
```

Tasks may remain READY until resources are available.

---

# 33. Logical Agent Roles

V1 defines four primary logical roles:

```text
Planner

Worker

Researcher

Verifier
```

These roles are sufficient for most workflows.

Specialists are introduced through skills rather than dozens of persistent personas.

---

# 34. Planner

Planner responsibilities:

```text
mission decomposition

task creation

dependency mapping

replanning

risk assessment

completion-gap analysis
```

Planner normally receives read-only repository tools.

---

# 35. Worker

Worker responsibilities:

```text
deep repository exploration

implementation

file modification

shell execution

debugging

tests

checkpoint creation

evidence production
```

A Worker must be powerful enough to complete normal tasks independently.

Subagents should not compensate for a weak Worker implementation.

---

# 36. Researcher

Researcher responsibilities:

```text
external documentation

API research

technical comparison

version investigation

implementation strategy research

evidence-backed recommendation
```

Research output must be persisted.

---

# 37. Verifier

Verifier responsibilities:

```text
challenge implementation

inspect actual diff

run tests

trace requirements

look for missing wiring

identify false success claims

check integration consequences
```

Verifier should generally be independent from the implementation model/provider where practical.

---

# 38. Specialist Skills

Examples:

```text
React

Rust

PostgreSQL

Security

Cloud Security

Playwright

Performance

Accessibility

Design

Supabase

AWS
```

A task may produce:

```text
Worker
+
React Skill
+
Playwright Skill
```

rather than spawning a permanent:

```text
React Agent.
```

---

# 39. Agent Runtime

The Agent Runtime is the execution harness used by logical roles.

It should provide each agent with:

```text
role instructions

task specification

context pack

tools

permissions

skills

event interface

checkpoint interface

Kernel messaging
```

---

# 40. Agent Session

An agent session is temporary.

Conceptually:

```text
agent_session_id

role

task_id

model

provider

context_pack_id

worker_id

started_at

last_activity

status
```

Agent sessions can die without losing mission state.

---

# 41. Worker Identity

Worker identity should be stable across model replacement when useful.

Example:

```text
worker_id = worker-T42
```

Its underlying session may change:

```text
session A
Step / NVIDIA
        ↓ failure
session B
Qwen / ModelScope
```

Task ownership can remain logically continuous.

---

# 42. Worker Registry

The Kernel tracks every active Worker.

Conceptually:

```text
worker_id

task_id

role

status

worktree

model

provider

lease

heartbeat

current_action

started_at
```

Possible states:

```text
STARTING

ACTIVE

WAITING_TOOL

WAITING_MODEL

CHECKPOINTING

VERIFYING

RECOVERING

STOPPED

FAILED
```

---

# 43. Worker Lease

Every active task assignment uses a lease.

Example:

```text
lease_id

task_id

worker_id

fencing_epoch

issued_at

last_renewed_at

expires_at

heartbeat_interval
```

The lease proves:

```text
Worker X currently owns Task Y.
```

---

# 44. Why Leases Matter

Without leases:

```text
worker dies
```

may leave:

```text
task permanently marked running.
```

With leases:

```text
worker heartbeat disappears
      ↓
lease expires
      ↓
Kernel detects abandoned task
      ↓
task enters RECOVERY
```

---

# 45. Heartbeats

Workers periodically report:

```text
alive

current operation

progress marker

last successful checkpoint

current model/provider

tool state
```

Heartbeats should be lightweight.

---

# 46. Activity vs Progress

Heartbeat activity must not be confused with actual progress.

A model repeatedly performing useless actions may remain alive while making no progress.

AgentCode therefore distinguishes:

```text
LIVENESS
```

from:

```text
PROGRESS.
```

---

# 47. Progress Markers

Possible progress signals:

```text
new files inspected

meaningful diff produced

test state changed

new evidence produced

new task subgoal completed

failure resolved

requirement status advanced
```

---

# 48. Stall Detection

A Worker may be considered stalled if:

```text
many repeated tool calls

no meaningful repository change

same error repeating

same model output pattern

no requirement advancement

excessive elapsed time
```

Kernel may then:

```text
request self-diagnosis

inject recovery instruction

switch model

invoke Planner

replace worker
```

---

# 49. Loop Detection

AgentCode should detect common autonomous loops such as:

```text
edit
test fails
undo
edit same thing
test fails
undo
```

or:

```text
grep X
read file
grep X
read same file
```

Repeated identical action fingerprints should increase loop suspicion.

---

# 50. Anti-Livelock

Munder Difflin's anti-livelock concepts are useful references.

AgentCode should implement:

```text
attempt counters

repeated-action detection

message hop limits

retry budgets

escalation rules
```

No task should consume unlimited inference because an agent refuses to converge.

---

# 51. Checkpoint

A checkpoint is a durable recovery point.

It may capture:

```text
task state

worktree commit or diff

context state

current errors

tests

important decisions

failed approaches

worker notes
```

---

# 52. Checkpoint Types

Possible types:

```text
AUTOMATIC

MILESTONE

PRE_RISKY_OPERATION

PRE_PROVIDER_SWITCH

PRE_COMPACTION

PRE_VERIFICATION

MANUAL
```

---

# 53. Automatic Checkpoints

Checkpoints should occur before:

```text
large refactor

dependency migration

database migration

risky bulk edit

model replacement

long-context compaction
```

and after:

```text
meaningful milestone

test suite passes

major task subgoal
```

---

# 54. Git-Based Evidence

Where possible, Worker changes should exist in Git-backed worktrees.

A checkpoint may therefore be:

```text
commit SHA
```

or:

```text
known diff against checkpoint commit.
```

Git provides excellent recovery evidence.

---

# 55. Checkpoint Is Not Completion

A checkpoint means:

```text
safe state exists.
```

It does not mean:

```text
task succeeded.
```

---

# 56. Recovery Engine

The Recovery Engine handles interrupted or failed execution.

Recovery inputs:

```text
task state

last checkpoint

current diff

failure classification

worker state

provider state

context snapshot

test state

attempt history
```

---

# 57. Recovery Workflow

Example:

```text
worker stream dies
      ↓
failure classified
      ↓
lease invalidated
      ↓
task → RECOVERY
      ↓
checkpoint loaded
      ↓
current repository reconciled
      ↓
new model selected
      ↓
new context pack built
      ↓
handoff packet created
      ↓
replacement Worker starts
```

---

# 58. Failure Classification

Failures must be classified.

Possible categories:

```text
MODEL_RATE_LIMIT

MODEL_TIMEOUT

MODEL_CONTEXT_LIMIT

MODEL_INVALID_RESPONSE

MODEL_PREMATURE_STOP

MODEL_LOOP

PROVIDER_OUTAGE

AUTH_FAILURE

TOOL_FAILURE

COMMAND_FAILURE

TEST_FAILURE

BUILD_FAILURE

MERGE_CONFLICT

WORKTREE_FAILURE

RESOURCE_EXHAUSTION

KERNEL_CRASH

USER_INTERRUPTION

UNKNOWN
```

Different failures require different recovery policies.

---

# 59. Model Failure vs Task Failure

Example:

```text
provider returns 429
```

does not imply:

```text
Task failed.
```

It means:

```text
execution attempt interrupted.
```

Kernel should replace the inference route and continue.

---

# 60. Test Failure vs Runtime Failure

A failing test may be expected during implementation.

Therefore:

```text
TEST_FAILED
```

is evidence.

Not necessarily:

```text
TASK_FAILED.
```

Task state depends on whether the Worker can continue making progress.

---

# 61. Retry Controller

Each task has retry policy.

Conceptually:

```text
attempt_count

same_model_attempts

same_strategy_attempts

cross_model_attempts

max_attempts

retry_backoff

escalation_threshold
```

---

# 62. Intelligent Retry

Bad:

```text
same model
same prompt
same failure
retry 10 times
```

Preferred:

```text
attempt 1 fails
      ↓
classify why
      ↓
change context / strategy / model / skill
      ↓
attempt 2
```

Retries should introduce new information or capability.

---

# 63. Retry Escalation Ladder

Example:

```text
Worker self-repair
        ↓
Worker with expanded context
        ↓
different model
        ↓
different provider
        ↓
Planner re-evaluation
        ↓
Researcher investigation
        ↓
independent debugging task
        ↓
human escalation
```

---

# 64. Failed Approaches

Every meaningful failed strategy should be recorded.

Example:

```text
attempt:
switch auth library

reason rejected:
breaks existing token format
```

Replacement agents should receive this information.

This prevents repeated rediscovery of failed approaches.

---

# 65. Agent Communication

Agents should communicate through durable structured messages.

Not transient chat only.

Conceptual message:

```text
message_id

from

to

task_id

type

payload

priority

created_at

read_at
```

---

# 66. Message Types

Examples:

```text
TASK_ASSIGNMENT

RESEARCH_RESULT

BLOCKER

QUESTION

EVIDENCE

VERIFICATION_FINDING

REPLAN_REQUEST

RECOVERY_NOTE

INTEGRATION_NOTICE
```

---

# 67. Mailbox Pattern

Each logical Worker may have:

```text
inbox

outbox
```

Messages persist in Kernel state.

This borrows useful actor/mailbox ideas from Munder Difflin without adopting its office metaphor.

---

# 68. No Agent-to-Agent Hidden Conversation

Agent communication should remain inspectable.

No important architectural decision should exist only in private multi-agent dialogue.

Durable messages should be:

```text
logged

traceable

linked to tasks
```

---

# 69. Shared Blackboard

A lightweight mission blackboard may contain:

```text
current blockers

high-priority findings

integration warnings

shared research results

cross-task constraints
```

It must not become an uncontrolled global prompt.

Agents retrieve relevant entries only.

---

# 70. Event Bus

Kernel subsystems communicate through typed events.

Examples:

```text
MissionCreated

PlanUpdated

RequirementAdded

TaskReady

TaskAssigned

WorkerStarted

WorkerHeartbeat

ToolStarted

ToolCompleted

CheckpointCreated

TaskImplemented

VerificationStarted

VerificationFailed

TaskCompleted

ProviderSwitched

WorkerRecovered

MissionCompleted
```

---

# 71. Append-Only Event Log

Important events should be persisted.

Conceptually:

```text
event_id

timestamp

mission_id

task_id

worker_id

event_type

payload

causation_id

correlation_id
```

This supports:

```text
debugging

recovery

auditing

timeline UI

metrics
```

---

# 72. Event Log vs Current State

The system maintains both:

```text
current relational state
```

and:

```text
historical event log.
```

Current state enables fast operation.

Event history explains how AgentCode reached that state.

---

# 73. Idempotency

Kernel operations must consider retry safety.

Example:

```text
task assignment request retried
```

must not accidentally create:

```text
two Workers
```

for one exclusive task.

Important operations should have:

```text
idempotency keys

operation IDs
```

where practical.

---

# 74. Non-Idempotent Tool Calls

Provider/model retry must never blindly replay potentially destructive operations.

Examples:

```text
database migration

publish package

delete file

deploy

send network mutation
```

Tool execution state must be checked before retrying.

---

# 75. Exactly-Once Is Not Assumed

Distributed/recoverable execution rarely guarantees perfect exactly-once semantics.

AgentCode should instead design important operations to be:

```text
detectable

idempotent where possible

reconcilable
```

---

# 76. Kernel State Store

V1 should use SQLite as the primary local transactional store.

Reasons:

```text
local

portable

simple

transactional

reliable

sufficient
```

---

# 77. Conceptual Core Tables

Possible tables:

```text
missions

mission_requirements

plans

tasks

task_dependencies

workers

leases

checkpoints

attempts

messages

events

evidence

human_requests

resource_locks

notifications
```

The exact physical SQL DDL is implementation-specific, but the authoritative logical control-plane schema, ownership and invariants are defined in **Section 0.31** of this hardened revision.

---

# 78. Transaction Boundaries

Critical transitions should occur transactionally.

Example:

```text
assign Task T42 to Worker W7
```

should atomically update:

```text
task ownership

worker state

lease

event
```

to avoid partial assignment.

---

# 79. Single Kernel Writer Principle

For core mission state, AgentCode should prefer:

```text
single authoritative Kernel writer
```

with controlled transactional access rather than allowing every Worker to mutate state directly.

Workers submit:

```text
requests

events

evidence
```

Kernel performs authoritative transitions.

---

# 80. Planner Replanning

Replanning may occur when:

```text
new requirement added

task fails repeatedly

research invalidates approach

security finding appears

major dependency conflict discovered

architecture assumption wrong
```

---

# 81. Replanning Must Preserve History

Planner must not simply replace the entire DAG invisibly.

Changes should record:

```text
old plan

new plan

reason

affected tasks

superseded tasks
```

---

# 82. Task Supersession

If a plan changes:

```text
Task T14
```

may become:

```text
SUPERSEDED
```

rather than deleted.

Historical execution remains auditable.

---

# 83. Dynamic Task Creation

Workers and Verifiers may discover new required work.

Example:

```text
Worker changes API
        ↓
discovers frontend client not covered
        ↓
proposes new task
```

Kernel validates and adds task to DAG.

Agents do not silently expand mission scope without record.

---

# 84. Research Tasks

If implementation depends on uncertain external information:

```text
Worker
   ↓
requests research
   ↓
Kernel creates Research task
   ↓
Researcher works
   ↓
result persisted
   ↓
Worker resumes
```

---

# 85. Research Evidence

Research output should contain:

```text
question

sources

findings

recommendation

rejected options

version/date assumptions

risks
```

This becomes durable mission evidence.

---

# 86. Verification Tasks

Verification should usually be represented explicitly.

Example:

```text
T42 Implement auth repair

depends →
T43 Verify auth repair
```

This prevents verification from being treated as an optional final sentence in a Worker response.

---

# 87. Verification Failure

If verification fails:

```text
T43
→ FAILED
```

the implementation task enters the canonical repair state:

```text
REPAIR
```

Kernel may:

```text
return to same Worker

assign different Worker

create focused repair task
```

---

# 88. Incremental Verification

After an implementation has previously passed verification, later small changes may receive incremental review.

Example:

```text
previously verified change set
        +
new diff
        +
dependency impact
        ↓
incremental verifier
```

A broader final verification still occurs before mission completion.

---

# 89. Integration

Parallel tasks must eventually be reconciled.

Conceptual flow:

```text
Task A passes
Task B passes
Task C passes
      ↓
integration stage
      ↓
merge/reconcile changes
      ↓
integration tests
      ↓
final verification
```

Doc 04 defines the Git/worktree mechanics.

The Kernel coordinates state.

---

# 90. Integration Conflicts

If parallel changes conflict:

```text
MERGE_CONFLICT
```

should become a first-class task/failure.

Kernel may create:

```text
Integration Repair Task
```

instead of blindly asking one Worker to overwrite the other's changes.

---

# 91. Resource Locks

Some tasks require exclusive access to resources.

Examples:

```text
package lockfile

database schema

shared migration directory

release configuration

deployment environment
```

Kernel can issue logical locks.

Conceptually:

```text
resource_id

owner_task

lease

mode
```

---

# 92. Human Escalation

AgentCode should minimize interruptions.

Human involvement is required only when necessary.

Examples:

```text
destructive production operation

missing credential/login

ambiguous irreversible choice

paid budget exceeded

legal/licensing concern

external service requires approval

all viable recovery strategies exhausted
```

---

# 93. Human Escalation Object

Conceptually:

```text
request_id

mission_id

task_id

reason

severity

question

options

recommended_choice

blocking
```

---

# 94. Goal Mode Autonomy

Default Goal Mode should allow safe routine actions autonomously.

The user should not need to approve:

```text
read file

search code

create worktree

edit task files

run tests

run lint

build local project

inspect Git

use approved provider
```

Approval policies are defined in detail in Doc 04.

---

# 95. Pause Mission

Users must be able to:

```text
Pause
```

A pause should:

```text
stop starting new tasks

checkpoint active tasks

allow safe command termination

persist state
```

---

# 96. Resume Mission

Resume should:

```text
reload state

reconcile worktrees

revalidate repository changes

refresh provider health

expire abandoned leases

continue READY work
```

---

# 97. Cancel Mission

Cancellation should be explicit.

It should:

```text
stop workers

checkpoint current state if possible

preserve changes unless user requests rollback

mark mission CANCELLED
```

Cancellation must not silently destroy useful code.

---

# 98. User Modification During Mission

Users may manually edit repository files during execution.

Kernel should not assume complete repository ownership.

Doc 02's index detects changes.

Kernel should:

```text
record external modification

invalidate affected assumptions

notify conflicting worker if necessary
```

---

# 99. Mission State Machine

Recommended:

```text
CREATED
   ↓
ANALYZING
   ↓
PLANNING
   ↓
EXECUTING
   ↓
VERIFYING
   ↓
FINAL_AUDIT
   ↓
COMPLETE
```

Additional states:

```text
PAUSED

BLOCKED

RECOVERING

CANCELLED

FAILED
```

---

# 100. Mission Failure

`FAILED` should be rare.

Most failures should instead create:

```text
RECOVERING
```

or:

```text
BLOCKED.
```

A mission becomes FAILED only when:

```text
recovery options exhausted

required condition impossible

user terminates as failure

critical unrecoverable corruption
```

---

# 101. Final Audit

Mission completion requires a final audit separate from normal task verification.

Final Audit receives:

```text
original goal

current requirement matrix

actual repository state

combined diff

tests

build status

security state where applicable

unresolved findings

known limitations
```

---

# 102. Final Verifier

The final verifier should ideally use:

```text
different model family

different provider
```

from the primary implementation path when practical.

Doc 01 controls routing.

---

# 103. Final Completion Gate

The Kernel may transition to:

```text
MISSION_COMPLETE
```

only if all applicable gates pass.

Minimum:

```text
required tasks terminal

dependencies satisfied

requirement matrix complete

blocking findings = 0

required tests pass

build/lint/typecheck gates pass

integration verified

final audit passes

required artifacts exist
```

---

# 104. Completion Is Evidence-Based

The completion gate checks:

```text
evidence
```

not model confidence.

Bad:

```text
Verifier:
"Looks good to me."
```

Preferred:

```text
Requirement R12

Implementation:
src/auth/session.ts

Test:
tests/auth/session.test.ts

Execution:
PASS

Verifier:
PASS
```

---

# 105. Mission Completion Summary

When complete AgentCode generates a compact report:

```text
Mission completed

Requirements:
14/14 verified

Tasks:
23 complete

Tests:
318 passed

Security:
0 blocking findings

Files changed:
17

Models used:
...

Recovery events:
2

Paid cost:
...

Duration:
...
```

Detailed evidence remains inspectable.

---

# 106. Notification Interface

On successful completion:

```text
system notification
+
subtle short completion sound
```

On human intervention:

```text
distinct subtle notification
```

On ordinary worker/provider recovery:

```text
no interruption by default.
```

Doc 06 defines user experience details.

---

# 107. Background Completion

The user should be able to leave the desktop UI closed while the daemon continues.

On completion:

```text
macOS notification

"AgentCode — Mission complete"
```

Clicking opens the mission summary.

---

# 108. Mission Budget

Each mission may contain:

```text
token budget

paid spend budget

maximum duration

maximum parallelism
```

Budget policies must not silently compromise correctness.

---

# 109. Free-First Execution

Doc 01's free-first model strategy applies.

The Kernel only needs to know:

```text
route available

route failed

cost policy
```

It does not directly manage provider details.

---

# 110. Paid Escalation

If free inference becomes unavailable:

```text
Kernel
   ↓
Model Broker
   ↓
paid route required?
```

Paid usage occurs according to mission/user policy.

Significant budget overrun should require escalation.

---

# 111. Context Compaction Integration

Doc 02 handles context compaction.

Kernel triggers it at lifecycle boundaries such as:

```text
agent replacement

task checkpoint

phase transition

provider switch

mission pause
```

The Kernel should never depend on raw transcript continuity.

---

# 112. Model Context Exhaustion

If a model reaches its context limit:

```text
CONTEXT_LIMIT
```

Kernel should:

```text
checkpoint task

invoke compaction

build new context pack

continue same model if suitable
```

or:

```text
switch model.
```

---

# 113. Provider Failure

Example:

```text
Worker using Step/NVIDIA
        ↓
NVIDIA outage
        ↓
Doc 01 reports route unavailable / alternate candidate
        ↓
Kernel/Runtime checkpoint only if session replacement would otherwise lose meaningful state
        ↓
new route selected
        ↓
replacement session continues
```

Mission state remains unchanged.

---

# 114. Worker Process Crash

If a worker runtime process crashes:

```text
heartbeat missing
      ↓
lease expires
      ↓
task RECOVERING
      ↓
replacement worker
```

---

# 115. Tool Process Crash

A tool failure should normally remain inside Worker attempt state.

Example:

```text
Playwright process crashed
```

does not necessarily invalidate:

```text
task implementation.
```

Worker or Kernel may restart the tool.

---

# 116. Kernel Crash

Kernel restart recovery should perform reconciliation.

Check:

```text
running worker processes

active worktrees

unfinished commands

database transactions

task leases

repository HEADs
```

Any uncertain active lease should eventually expire and recover.

---

# 117. Power Loss

AgentCode should be designed assuming sudden termination.

Important state should already exist in:

```text
SQLite

Git/worktree

event log

checkpoints
```

not only process memory.

---

# 118. Recovery Reconciliation

On restart:

```text
load missions
      ↓
inspect RUNNING tasks
      ↓
inspect workers
      ↓
inspect leases
      ↓
inspect worktrees
      ↓
inspect pending command state
      ↓
classify reality
```

Possible result:

```text
worker gone
worktree intact
checkpoint intact
→ resume
```

---

# 119. Poisoned State Prevention

If task state and repository reality disagree:

```text
do not blindly resume.
```

Kernel should run reconciliation before proceeding.

Example:

```text
database says Task T4 checkpoint = commit X

worktree HEAD = Y
```

The mismatch must be investigated.

---

# 120. Deterministic Core, LLM-Assisted Decisions

The Kernel itself should avoid requiring an LLM for:

```text
state transitions

leases

heartbeats

dependency checks

retry counts

budget checks

task readiness

completion gates
```

LLMs may assist with:

```text
planning

complexity assessment

recovery strategy

requirement interpretation

architecture reasoning
```

---

# 121. Kernel Should Function Without Local LLM

AgentCode's local Sentinel is useful.

It is not required for correctness.

If Ollama is unavailable:

```text
Kernel still runs.
```

---

# 122. Sentinel Integration

Llama 1B may assist with:

```text
log summaries

simple failure classification

recovery summaries

human-readable status
```

Kernel validates deterministic facts independently.

---

# 123. Router Advisor Integration

Qwen3 4B may assist the Model Broker when model-selection decisions are ambiguous.

This remains Doc 01 responsibility.

The Kernel requests:

```text
role/task requirements
```

and receives:

```text
selected execution route.
```

---

# 124. Agent Runtime Permissions

Each role receives a capability boundary.

Conceptually:

```text
Planner
read/query/plan

Researcher
read/research

Worker
read/write/shell/test

Verifier
read/test/inspect
```

Detailed permission architecture is Doc 04.

---

# 125. Planner Write Restriction

Planner should normally not mutate repository files.

This preserves separation between:

```text
decide what should happen
```

and:

```text
perform implementation.
```

---

# 126. Verifier Write Restriction

Verifier should initially be read/test-only.

If it detects a failure:

```text
report finding
```

Kernel creates repair work.

This reduces the risk of a verifier silently fixing the very issue it is supposed to independently detect.

---

# 127. Self-Repair Exception

A future optimized mode may permit a verifier to generate a suggested patch.

But:

```text
verification
```

and:

```text
patch acceptance
```

must remain distinct states.

---

# 128. Task Acceptance Criteria

Every implementation task should have explicit acceptance criteria.

Example:

```text
T42 Password reset

AC1 endpoint accepts valid token

AC2 expired token rejected

AC3 token invalid after use

AC4 password update persisted

AC5 tests pass
```

The Worker receives them.

Verifier independently checks them.

---

# 129. Hidden Completion Conditions Are Forbidden

If a Planner knows a task requires:

```text
backend
frontend
tests
```

all must be present in task/requirement state.

Critical expectations should not remain only in natural-language conversation history.

---

# 130. Task Evidence

Tasks produce evidence objects.

Possible categories:

```text
CODE_CHANGE

TEST_PASS

BUILD_PASS

SCREENSHOT

RUNTIME_TRACE

SECURITY_RESULT

RESEARCH_SOURCE

VERIFIER_REPORT

USER_APPROVAL
```

---

# 131. Evidence Representation

Conceptually:

```text
evidence_id

task_id

type

source

artifact_ref

summary

timestamp

freshness

verified
```

---

# 132. Evidence Freshness

Evidence can become stale.

Example:

```text
tests passed
```

then code changes afterward.

Relevant prior test evidence may become:

```text
STALE
```

and require rerun.

---

# 133. Dependency-Driven Evidence Invalidation

Doc 02 impact analysis may indicate:

```text
Task B modified code covered by Task A tests.
```

Kernel can invalidate:

```text
Task A verification evidence
```

when necessary.

---

# 134. Task Completion vs Integration Completion

A task may be locally complete but not yet integrated.

Example:

```text
Task A worktree
PASS
```

but:

```text
main integration
not performed.
```

Therefore:

```text
PASSED
```

and:

```text
INTEGRATED
```

remain separate.

---

# 135. Scheduler Fairness

Long-running low-priority tasks must not indefinitely starve other ready work.

Scheduler should consider:

```text
priority

age

blocking value

critical path
```

---

# 136. Critical Path Awareness

Tasks that unlock many downstream tasks should receive scheduling preference.

Example:

```text
shared schema migration
```

may unblock:

```text
backend
frontend
tests
```

---

# 137. Mission Critical Path

The Planner/Scheduler may estimate:

```text
critical path
```

to improve total mission completion time without simply maximizing concurrency.

---

# 138. Speculative Work

AgentCode should be cautious with speculative parallel implementation.

Bad:

```text
Worker B assumes Worker A's unverified API.
```

Preferred:

```text
wait for stable interface
```

or:

```text
explicit shared contract checkpoint.
```

---

# 139. Shared Contracts

Parallel tasks may depend on an approved contract artifact.

Examples:

```text
API schema

type definition

database migration

interface
```

Once stabilized:

```text
contract checkpoint
```

allows safe parallel work.

---

# 140. Agent Limits

No agent should have unlimited authority.

Every session has:

```text
task scope

tool scope

repository scope

time budget

attempt budget

cost policy
```

---

# 141. Scope Drift Detection

If Worker T42 begins modifying unrelated subsystems:

```text
Kernel should detect scope expansion.
```

Signals:

```text
files far outside impact graph

unrelated package

unexpected destructive command
```

Kernel may:

```text
allow justified expansion

request explanation

create separate task

block action
```

---

# 142. Scope Expansion

Legitimate implementation may reveal additional required changes.

Worker can submit:

```text
SCOPE_EXPANSION_REQUEST
```

containing:

```text
reason

affected files

requirement impact
```

Kernel updates task or creates new task.

---

# 143. Human Override

User may:

```text
change priority

pause task

cancel task

force provider

pin model

add requirement

remove requirement

approve risky action
```

Kernel records overrides as events.

---

# 144. New User Instruction During Mission

If user says:

```text
Also make the dashboard responsive.
```

Kernel should:

```text
record new instruction

update requirement matrix

increment requirements version

ask Planner to reconcile DAG
```

Existing work remains preserved.

---

# 145. Contradictory User Instruction

If later user instruction conflicts with earlier one:

```text
new explicit instruction
```

takes precedence.

The conflict and replacement should be recorded.

---

# 146. Mission Forking

Future support may allow:

```text
duplicate mission state
```

to test alternate architectures.

Not required for early V1.

Architecture should not preclude it.

---

# 147. Mission Templates

Future AgentCode may support reusable mission patterns:

```text
Production Readiness Audit

Security Audit

Dependency Upgrade

UI Redesign

Bug Fix

Repository Deep Analysis
```

They translate into requirement/task templates.

The Kernel remains generic.

---

# 148. Observability

The Kernel should expose:

```text
mission status

requirements progress

task count

ready tasks

running tasks

blocked tasks

worker count

provider switches

recoveries

retry counts

current work

test state

verification state

cost

duration
```

---

# 149. Minimal User Status

Doc 06's UI should reduce this to something like:

```text
AgentCode — Working

42% complete

Current:
Implementing session recovery

23 / 51 tasks complete

No action required
```

Detailed Kernel internals remain available when expanded.

---

# 150. Timeline View

Advanced inspection can show:

```text
00:41 Mission created

00:42 Repository indexed

00:43 Plan v1 created

00:44 T1 assigned

00:57 T1 verified

01:04 NVIDIA failed

01:04 Worker recovered on ModelScope

01:22 Integration tests passed
```

---

# 151. Metrics

Track:

```text
mission duration

task duration

time blocked

worker utilization

parallelism

recovery count

retries

model switches

provider switches

stalled workers

requirements completion rate

first-attempt task success

verification rejection rate

tokens per verified task

cost per verified task
```

---

# 152. Autonomous Runtime Benchmarks

AgentCode should be evaluated on missions requiring:

```text
multi-file changes

multi-hour execution

provider failure

context compaction

agent replacement

parallel work

integration

verification
```

Not only single prompt coding tasks.

---

# 153. Crash Recovery Acceptance Test

Start mission.

Kill AgentCode daemon.

Restart.

Expected:

```text
mission rediscovered

state recovered

worktree reconciled

expired worker detected

task resumed
```

No manual mission recreation.

---

# 154. Provider Failure Acceptance Test

During Worker execution:

```text
force provider failure
```

Expected:

```text
provider failure classified

checkpoint preserved

replacement route selected

new worker/session continues

task does not restart unnecessarily
```

---

# 155. False Completion Acceptance Test

Worker says:

```text
"Task finished successfully."
```

while required test fails.

Expected:

```text
Kernel rejects task completion.
```

---

# 156. Lease Expiration Acceptance Test

Kill Worker without cleanup.

Expected:

```text
heartbeat stops

lease expires

task → RECOVERING

replacement worker assigned
```

---

# 157. Loop Detection Acceptance Test

Create Worker behavior that repeats the same ineffective actions.

Expected:

```text
loop detected

attempt classified

recovery strategy changed
```

rather than infinite token usage.

---

# 158. Replanning Acceptance Test

During implementation reveal:

```text
original architecture assumption invalid.
```

Expected:

```text
Planner replans

existing completed evidence preserved

affected tasks superseded/updated

DAG remains coherent
```

---

# 159. Parallel Task Acceptance Test

Create two independent tasks.

Expected:

```text
separate workers

separate worktrees

parallel execution

independent verification
```

without cross-contamination.

---

# 160. Conflict Prevention Acceptance Test

Create two tasks likely to modify the same critical file.

Expected:

```text
Scheduler identifies conflict

tasks serialized
```

or coordinated explicitly.

---

# 161. Context Replacement Acceptance Test

Terminate Worker after substantial progress.

Replacement model receives Doc 02 handoff context.

Expected:

```text
continues from current state

does not redo entire repository exploration

does not repeat known failed approach
```

---

# 162. Requirement Drift Acceptance Test

Mission contains eight requirements.

After long planning/execution:

Expected:

```text
all eight remain represented

none silently disappear during compaction/replanning.
```

---

# 163. Restart Persistence Acceptance Test

Pause mission.

Quit application.

Restart machine/application.

Expected:

```text
mission available

task DAG intact

requirements intact

checkpoints intact

worktree state reconciled
```

---

# 164. Background Daemon Acceptance Test

Start mission.

Close main AgentCode window.

Expected:

```text
mission continues.
```

When complete:

```text
system notification appears.
```

---

# 165. V1 Completion Definition

The Autonomy Kernel & Agent Runtime subsystem is V1-complete only when:

```text
✓ background daemon exists

✓ UI can disconnect without killing mission

✓ missions persist

✓ original goal remains immutable

✓ structured requirements exist

✓ requirement matrix exists

✓ requirement versioning works

✓ Planner creates structured plan

✓ task DAG persists

✓ dependency scheduling works

✓ strict task state transitions exist

✓ Planner role works

✓ Worker role works

✓ Researcher role works

✓ Verifier role works

✓ specialist skills attach dynamically

✓ worker registry exists

✓ leases work

✓ heartbeats work

✓ lease expiration triggers recovery

✓ liveness and progress are distinguished

✓ stall detection exists

✓ loop detection exists

✓ anti-livelock safeguards exist

✓ checkpoints persist

✓ checkpoint restoration works

✓ failure classification exists

✓ provider failure does not fail mission

✓ model replacement works

✓ retry controller works

✓ retries change strategy when appropriate

✓ failed approaches persist

✓ structured agent messaging exists

✓ event bus exists

✓ append-only event history exists

✓ core state transitions are transactional

✓ task assignment is idempotent

✓ non-idempotent tool retry is protected

✓ concurrency manager works

✓ conflict-aware scheduling works

✓ resource governor works

✓ research task workflow works

✓ verification task workflow works

✓ verification failure creates repair path

✓ integration stage exists

✓ resource locks exist

✓ human escalation exists

✓ mission pause works

✓ mission resume works

✓ mission cancel works safely

✓ external repository changes are reconciled

✓ mission crash recovery works

✓ power/process-loss recovery is possible

✓ replanning preserves history

✓ dynamic task creation works

✓ scope drift detection exists

✓ user instructions can update running mission

✓ final audit exists

✓ deterministic completion gate exists

✓ false model completion cannot bypass gate

✓ mission completion report exists

✓ completion notification works

✓ system can run a substantial mission without continuous babysitting
```

---

# 166. Locked V1 Principles

The following are locked:

1. The Kernel is the authoritative mission controller.

2. LLMs are replaceable workers.

3. The UI is not the runtime.

4. Missions survive UI closure.

5. Missions survive model/provider replacement.

6. Original user goals remain immutable.

7. Requirements are explicit and traceable.

8. Completion is requirement-based.

9. Tasks form a persistent dependency graph.

10. Task state transitions are deterministic.

11. Planner state must be externalized.

12. Planner, Worker, Researcher and Verifier are the core roles.

13. Specialist expertise is normally supplied as skills.

14. A Worker must independently support full repository work.

15. Every active assignment uses a lease.

16. Workers emit heartbeats.

17. Liveness is different from progress.

18. Stalls and loops must be detectable.

19. No autonomous loop receives unlimited retries.

20. Checkpoints exist independently of model conversations.

21. Failure recovery uses structured state.

22. Provider failures do not equal task failures.

23. Context-limit failures do not equal task failures.

24. Retries should introduce new information or strategy.

25. Failed approaches should be remembered.

26. Agent communication is durable and inspectable.

27. Important mission decisions must not exist only in chat.

28. Kernel state transitions are transactional.

29. Important mutations should be idempotent or reconcilable.

30. Parallelism is conflict-aware.

31. More workers are not automatically better.

32. Resource use must respect target hardware.

33. Worktrees provide isolation but Kernel owns coordination.

34. Verification is explicit.

35. Verifiers are independent where practical.

36. Verified task completion and integration completion are separate.

37. Evidence can become stale after related changes.

38. New user requirements can modify an active mission.

39. Replanning preserves historical truth.

40. Human intervention should be rare and meaningful.

41. Routine safe operations should not require babysitting.

42. Mission completion requires deterministic gates.

43. No LLM may directly declare a mission complete.

44. Final audit compares actual repository state against the original mission.

45. AgentCode's flagship experience is:

```text
define goal
     ↓
start
     ↓
leave
     ↓
receive notification when genuinely finished
```

---

# 167. Final Architecture

```text
                              USER
                               │
                               ▼
                        MISSION CREATION
                               │
                               ▼
                      IMMUTABLE ORIGINAL GOAL
                               │
                               ▼
                       REQUIREMENT MANAGER
                               │
                               ▼
                       REQUIREMENT MATRIX
                               │
                               ▼
                            PLANNER
                               │
                               ▼
                           TASK DAG
                               │
                               ▼
                           SCHEDULER
                               │
                  ┌────────────┼────────────┐
                  ▼            ▼            ▼
               Worker      Researcher    Verifier
                  │            │            │
                  └────────────┼────────────┘
                               │
                           AGENT RUNTIME
                               │
             ┌─────────────────┼─────────────────┐
             ▼                 ▼                 ▼
        MODEL BROKER      CONTEXT ENGINE      TOOL ENGINE
           Doc 01            Doc 02             Doc 04
             │                 │                 │
             └─────────────────┼─────────────────┘
                               ▼
                          EXECUTION
                               │
                               ▼
                     CHECKPOINT + EVIDENCE
                               │
                               ▼
                           KERNEL
                               │
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
            RECOVERY       NEW TASK       VERIFY
                │                             │
                └──────────────┬──────────────┘
                               ▼
                         INTEGRATION
                               │
                               ▼
                         FINAL AUDIT
                               │
                               ▼
                      COMPLETION GATE
                         /          \
                       FAIL         PASS
                        │             │
                        ▼             ▼
                      REPAIR     MISSION COMPLETE
                                      │
                                      ▼
                              NOTIFICATION + REPORT
```

---

# 168. Final Statement

AgentCode must not be autonomous merely because an LLM is allowed to keep producing messages.

True autonomy requires durable control outside the model.

The AgentCode Kernel must know:

```text
what the user originally asked for

what requirements remain

what tasks exist

what each task depends on

which workers are alive

what work has actually changed

what has been tested

what failed

what has already been attempted

what evidence exists

what must happen next
```

If a Planner disappears:

```text
planning survives.
```

If a Worker crashes:

```text
task state survives.
```

If a provider runs out of quota:

```text
mission survives.
```

If context fills:

```text
knowledge survives.
```

If the desktop interface closes:

```text
execution survives.
```

If an LLM falsely says:

```text
"done"
```

the Kernel still asks:

```text
"Where is the evidence?"
```

The intended result is:

> **A durable autonomous execution kernel capable of taking a high-level software-engineering goal, converting it into traceable requirements and executable tasks, coordinating replaceable agents across hours of work, recovering from failures automatically, preserving progress across crashes and model changes, and refusing to declare success until the real repository satisfies deterministic completion conditions.**

This document is the **V1 source of truth for AgentCode's Autonomy Kernel and Agent Runtime subsystem.**

