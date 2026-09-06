# WP08 Context and Memory Adoption Decision

## Decision Summary

AgentCode will ADAPT a typed Context and Memory architecture with four separated layers:

1. Kernel durable state,
2. raw evidence store,
3. structured project memory/facts/decisions,
4. runtime context packs and retrieval accelerators.

Transcript, embeddings, graph-derived facts, and summaries are never authoritative.

## Mechanism Decisions

| Mechanism | Donor evidence | Classification | AgentCode decision |
|---|---|---:|---|
| Context graph working buffer | Gemini `contextManager.ts` at `571851b1077a51cef757146ce13f9da887326bec` | ADAPT | Use pristine graph plus working render buffer to separate committed state from pending context assembly. |
| Stable render cache | Gemini `lastRenderCache` | TAKE | Cache rendered context by stable node/header hashes. |
| Management trigger evaluation | Gemini `evaluateTriggers` | ADAPT | Use token/freshness/degradation triggers to request summarization or compaction, not to mutate Kernel truth. |
| Stale processor target drop | Gemini context processor handling | TAKE | Drop or mark results when source nodes changed before processor completion. |
| Chat history as durable source | Gemini `renderHistory` input model | REJECT | AgentCode durable source is Kernel/evidence/memory stores, not transcript. |
| File compression state | Gemini `contextCompressionService.ts` | TAKE | Use content-hash keyed compression records for file/tool evidence views. |
| Model-decided compression level | Gemini `FileLevel` routing | ADAPT | LLM can suggest context level; policy and evidence freshness decide final inclusion. |
| Rolling summary with `abstractsIds` | Gemini `rollingSummaryProcessor.ts` | TAKE | Every summary must name exact source evidence/node ids. |
| Silent summary failure fallback | Gemini failure logging | REJECT | AgentCode must surface degraded compression/summarization evidence. |
| Hierarchical memory files | Gemini `memoryContextManager.ts` | ADAPT | Support global/project/subdirectory scoped memory with trust and source provenance. |
| Concatenated memory instructions | Gemini memory render | REJECT | Concatenation without typed authority class invites prompt authority confusion. |
| Tool output masking | Gemini `toolMaskingProcessor.ts` | TAKE | Preserve raw output and pass masked pointer into context. |
| Temp-file raw output storage | Gemini tool-output path | ADAPT | Store raw output in AgentCode Evidence Store with hashes, retention, and redaction policy. |
| Node truncation with `replacesId` | Gemini `nodeTruncationProcessor.ts` | TAKE | Derived context nodes must identify original evidence they replace. |
| Letta default memory blocks | Letta Code `memory.ts` at `5786193dd10d` | IGNORE | Persona/human defaults are not needed for AgentCode project memory V1. |
| Read-only memory labels | Letta Code `READ_ONLY_BLOCK_LABELS` | ADAPT | Use immutable categories for architecture decisions and accepted facts. |
| Scoped memory filesystem | Letta Code `memory-filesystem.ts` | ADAPT | Use explicit Kernel/project/agent scope; env vars cannot own scope. |
| Memory enablement gate | Letta Code `memory-runtime.ts` | TAKE | Memory access must be capability-gated and observable. |
| MemFS tag at creation | Letta Code `stampMemfsTagOnCreateBody` | ADAPT | Record memory capability intent at session/project initialization for recovery. |
| Git-backed memory commits | Letta Code `memory-git.ts` | ADAPT | Git can store evidence/history of memory changes; Kernel accepts/supersedes facts. |
| Git auth redaction/noninteractive env | Letta Code `redactGitAuthError`, `buildNonInteractiveGitEnv` | TAKE | Apply to every git-backed memory sync operation. |
| Git memory as truth | Letta Code memory repo model | REJECT | Memory git repo is evidence/history, not authoritative Kernel state. |
| Memory writer confinement | Letta Code `memory-confinement.ts` | TAKE | Memory mutation tools must fail closed under filesystem/process policy. |
| Transcript FTS/vector/hybrid search | Letta Code `message-search.ts` | ADAPT | Use retrieval as accelerator over raw evidence with authority labels. |
| Temporal episode graph | Graphiti `graphiti.py`, `nodes.py` at `10374d6044f9` | ADAPT | Model project observations/facts as episodes/triples with source links, group ids, validity windows. |
| Graph reranking | Graphiti `search.py` | STUDY/ADAPT | Use MMR/RRF/cross-encoder later for retrieval ranking; V1 can begin with simpler deterministic ranking. |
| Graph facts as authority | Graphiti extracted entities/edges | REJECT | Facts require Kernel acceptance, provenance, confidence, and freshness. |
| RTK deterministic filters | RTK `toml_filter.rs` at `ba7a9ce0d92a46f2458b82b1fcdd000f887f651a` | TAKE | Adopt deterministic output compression pipeline for tool evidence views. |
| RTK trust-gated filters | RTK filter registry | TAKE | Project/user filters require trust/admission; otherwise built-in/passthrough only. |
| RTK `never_worse` guard | RTK `summary.rs` | TAKE | Compression must not obscure more than it saves; raw evidence remains available. |
| RTK filters as sandbox | RTK docs | REJECT | Filters reduce context size; they do not enforce security. |

## Required AgentCode Posture

- Kernel durable state is the only authority for missions/tasks/completion.
- Raw evidence store owns immutable command/file/tool outputs.
- Structured memory owns accepted facts, decisions, handoffs, conflicts, supersession, confidence, and freshness metadata.
- Retrieval memory owns indexes/embeddings/graphs as rebuildable accelerators.
- Runtime context owns prompt-ready packs with source links and expiry.

## Rejected Donor Couplings

- Transcript-as-state.
- Generated summaries without provenance.
- Embedding/vector hit as fact.
- Memory files that silently override architecture documents.
- Git memory repos as hidden agent memory.
- Compression that discards raw evidence.
