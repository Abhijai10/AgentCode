# P01-WP08 Context and Memory Implementation Packet

## Future Interfaces Required

### Context Engine

```text
ContextEngine
  buildContextPack(request: ContextPackRequest) -> ContextPack
  renderContext(packId: ContextPackId, target: ModelTarget) -> RenderedContext
  compressEvidence(request: CompressionRequest) -> CompressionReceipt
  explainContextItem(itemId: ContextItemId) -> EvidenceReference
```

### Memory Service

```text
MemoryService
  recordFact(input: FactInput) -> MemoryFactId
  supersedeFact(factId: MemoryFactId, replacement: FactInput, reason: SupersessionReason) -> MemoryFactId
  recordDecision(input: DecisionInput) -> MemoryDecisionId
  searchMemory(query: MemoryQuery) -> MemorySearchResult
  expireMemory(scope: MemoryScope, policy: ExpiryPolicy) -> ExpiryReceipt
  exportHandoff(scope: KernelScope) -> HandoffContext
```

### Evidence Store Integration

```text
EvidenceStore
  putRawEvidence(input: RawEvidenceInput) -> EvidenceId
  getEvidenceSlice(evidenceId: EvidenceId, range: EvidenceRange) -> EvidenceSlice
  createDerivedView(input: DerivedEvidenceInput) -> DerivedEvidenceId
```

## Core Data Shapes

- `AuthorityClass`: `KERNEL_STATE`, `RAW_EVIDENCE`, `ACCEPTED_MEMORY`, `DERIVED_SUMMARY`, `RETRIEVAL_ACCELERATOR`, `RUNTIME_CONTEXT`.
- `MemoryFact`: statement, scope, source evidence ids, confidence, freshness, valid_from, valid_until, supersedes, superseded_by, conflict_set.
- `MemoryDecision`: decision, rationale, authority refs, supersession policy.
- `ContextNode`: source id, authority class, token estimate, freshness, protected flag, replaces/abstracts ids.
- `ContextPack`: ordered nodes, budget, model target, scoring trace, omitted evidence summary.
- `CompressionRecord`: source evidence hash, compressor id/version, decision, summary id, abstracted ids, created time.
- `RetrievalRecord`: backend, query, mode, result ids, score trace, freshness, degraded marker.

## Ownership Boundaries

| Component | Owns | Must not own |
|---|---|---|
| Kernel | durable mission/task/completion truth and accepted handoff references | prompt summaries, embeddings |
| Evidence Store | immutable raw outputs, source slices, command/tool/file evidence | semantic truth |
| Memory Service | accepted project facts/decisions/handoff records with provenance/freshness | repository mutation, task completion |
| Retrieval Indexes | embeddings, FTS, graph indexes, caches | authority decisions |
| Context Engine | prompt assembly, compression, ranking, render cache | durable task truth |
| AgentSession/Worker | temporary working context and pending request preview | durable state |

## Data Ownership

- Raw command output remains in Evidence Store even when masked/truncated in prompts.
- Generated summaries and `CONTEXT.md` are derived views over explicit evidence.
- Memory facts require source evidence and can conflict; conflict is stored, not overwritten.
- Retrieval indexes can be deleted and rebuilt without losing truth.
- Handoff context is generated from Kernel state, accepted memory, and evidence references, not conversation transcript.

## State Ownership

- Kernel writes durable state.
- Memory Service writes typed memory records under Kernel-authorized scope.
- Context Engine writes derived context/compression receipts.
- Worker/AgentSession may hold temporary selected context but cannot become source of truth.
- File-backed or git-backed memory is synchronization/evidence storage, not authority.

## Context Assembly Requirements

- Select from Kernel state, current code evidence, accepted memory, recent raw evidence, and retrieval candidates.
- Label every context item with authority class and source reference.
- Protect system/core instruction nodes from summarization.
- Summaries must include exact `abstractsIds`; truncations must include `replacesId`.
- Stale or degraded items may appear only with visible freshness/degradation metadata.
- Model choice/token budget affects rendering, not source truth.

## Memory Requirements

- Facts and decisions need provenance, confidence, freshness, scope, and supersession links.
- Forgetting/expiry must preserve audit trail unless policy requires deletion.
- Conflicts must be represented explicitly.
- Retrieval memory may include embeddings and graph edges, but cannot answer without source references.
- Replacement Worker must resume from Kernel, Memory Service, and Evidence Store.

## Security Constraints

- Memory writers execute through sandbox/tool boundaries and fail closed.
- Project memory files are untrusted until admitted by policy.
- Do not load subdirectory memory from untrusted repo paths as instructions with authority.
- Tool-output masking must respect secrets/redaction policy.
- Git-backed memory sync uses noninteractive credentials and redacted errors.
- Compression filters are not a sandbox.

## Failure and Degraded Modes

- Compression model failure: keep raw evidence, mark compression degraded.
- Retrieval backend unavailable: return explicit degraded/no-op result.
- Memory conflict: store both facts with conflict set; do not auto-resolve.
- Stale memory: return only with stale marker or require refresh.
- Missing sandbox for memory writer: fail closed.
- Lost AgentSession: rebuild from durable Kernel/evidence/memory state.

## Verification Hooks for Future Phases

- Session replacement fixture: new Worker resumes task without prior transcript.
- Stale fact fixture: high-confidence fact becomes stale after source evidence changes.
- Summary provenance fixture: each summary resolves to original evidence ids.
- Conflict fixture: two contradictory facts are stored and surfaced, not overwritten.
- Masking fixture: full tool output remains retrievable while prompt sees pointer.
- Trust fixture: project memory file in untrusted repo is classified as untrusted context.

## Unresolved Design Questions

- What is the minimum V1 schema for accepted memory facts versus free-form notes?
- Which memory scopes are required: user, project, repository, branch, task, Worker?
- How should `CONTEXT.md` be generated and invalidated without becoming authority?
- Which retrieval modes are REQUIRED_V1: FTS only, embeddings, graph, or hybrid?
- What retention policy applies to raw evidence containing secrets?
