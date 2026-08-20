# WP08 Context and Memory Extraction

## Scope

Phase 1 WP08 extracts context assembly and memory mechanisms only. AgentCode adaptation target: strict separation between Kernel durable state, runtime context, retrieval memory, generated summaries, and raw evidence. Transcript, embeddings, and retrieved context are never authority.

## Donors Inspected

| Donor | Pinned commit | Files inspected |
|---|---:|---|
| Gemini CLI | `571851b1077a51cef757146ce13f9da887326bec` | `packages/core/src/context/contextManager.ts`; `contextCompressionService.ts`; `processors/rollingSummaryProcessor.ts`; `memoryContextManager.ts`; `processors/toolMaskingProcessor.ts`; `processors/nodeTruncationProcessor.ts`; `contextManager.incremental.test.ts` |
| Letta Code | `5786193dd10d` | `src/agent/memory.ts`; `memory-filesystem.ts`; `memory-filesystem.test.ts`; `memory-git.ts`; `memory-runtime.ts`; `memory-confinement.ts`; `backend/message-search.ts` |
| Graphiti | `10374d6044f9` | `graphiti_core/graphiti.py`; `graphiti_core/search/search.py`; `graphiti_core/nodes.py` |
| RTK | `ba7a9ce0d92a46f2458b82b1fcdd000f887f651a` | `src/core/filter.rs`; `src/core/toml_filter.rs`; `src/cmds/system/summary.rs`; `src/filters/README.md` |

## Mechanisms Discovered

### Gemini CLI Context Graph Render

Source: `packages/core/src/context/contextManager.ts`

Important symbols:
- `ContextManager`
- `ContextWorkingBufferImpl`
- `renderHistory`
- `evaluateTriggers`
- `performHotStartCalibration`
- `getProtectedNodeIds`
- `lastRenderCache`
- `evaluatedNodeIds`

Control flow:
1. `renderHistory` syncs durable chat history into a pristine graph.
2. It updates a working buffer and records newly added node ids.
3. It creates preview nodes for the pending request.
4. It performs one-time hot-start calibration.
5. It waits for pipelines, evaluates triggers, and runs management processors such as GC/distillation/normalization.
6. It renders the graph back into history/API-history views.
7. It syncs management changes from working buffer back to master state.
8. It validates graph invariants, hardens rendered history with sentinels, splits committed versus pending history, and caches by stable node/header hash.
9. Processor results targeting nodes no longer present are dropped.

Lifecycle behavior:
- Context rendering is derived from graph state.
- Working buffer separates speculative render work from committed state.
- Render cache avoids recomputing unchanged history.

Failure behavior:
- Stale processor targets are discarded.
- Invariant validation prevents silently emitting malformed context.

Tests inspected:
- `contextManager.incremental.test.ts`: verifies normalized-token-budget overflow emits `NormalizeNeeded` while avoiding unrelated consolidation.

Limitations:
- Donor durable input is chat-history oriented; AgentCode must derive context from Kernel state, raw evidence, and typed memory rather than transcript truth.

AgentCode extraction decision: ADAPT. Use graph/working-buffer/render-cache mechanics with AgentCode authority boundaries.

### Gemini File Context Compression

Source: `packages/core/src/context/contextCompressionService.ts`

Important symbols:
- `ContextCompressionService`
- `FileRecord`
- `FileLevel`
- `compressHistory`
- `hashStringSlice`
- `batchQueryModel`

Control flow:
1. Loads `compression_state.json` from project temp storage.
2. Protects recently read files from compression.
3. Finds older file read outputs in history.
4. Hashes file output slices.
5. Reuses cached compression decision/summary when content hash matches.
6. Batches model routing decisions: `FULL`, `PARTIAL`, `SUMMARY`, `EXCLUDED`.
7. Saves updated compression state once.
8. Applies decisions to history without additional model calls.

Lifecycle behavior:
- Compression state is per project temp root.
- Cached summaries are content-hash keyed.

Failure behavior:
- Missing cache entries trigger new model decisions.
- Invalid/stale cache is bypassed by hash mismatch.

Tests inspected:
- No dedicated compression tests read in this run.

Limitations:
- Model-selected compression levels must remain advisory.
- Summary cannot outrank raw file evidence.

AgentCode extraction decision: TAKE/ADAPT. Use hash-keyed compression records and batched routing, but persist provenance and authority class.

### Gemini Rolling Summary Processor

Source: `packages/core/src/context/processors/rollingSummaryProcessor.ts`

Important symbols:
- `createRollingSummaryProcessor`
- `RollingSummaryProcessorOptions`
- `RollingSummary`
- `abstractsIds`

Control flow:
1. Selects candidate graph nodes by strategy: incremental, free N tokens, or max.
2. Skips protected/system prompt nodes.
3. Calls compressor model to summarize target nodes.
4. Creates a `ROLLING_SUMMARY` node with `abstractsIds`.
5. Replaces consumed nodes with the summary node.
6. On failure, logs and returns targets unchanged.

Lifecycle behavior:
- Summary nodes keep provenance of abstracted node ids.
- Summaries are derived context nodes.

Failure behavior:
- Failure preserves original nodes and avoids corrupting context.

Tests inspected:
- No rolling summary tests read in this run.

Limitations:
- Silent logging is insufficient for AgentCode; failures need explicit evidence/degraded markers.

AgentCode extraction decision: ADAPT. Preserve provenance with `abstractsIds`; expose failed compression.

### Gemini Hierarchical Memory Files

Source: `packages/core/src/context/memoryContextManager.ts`

Important symbols:
- `MemoryContextManager`
- `refresh`
- `discoverMemoryPaths`
- `loadMemoryContents`
- `categorizeMemoryContents`
- `discoverContext`
- `loadedPaths`
- `loadedFileIdentities`

Control flow:
1. Discovers memory paths from global, extension, project, and user-project scopes.
2. Loads project/environment memory only for trusted folders.
3. Deduplicates loaded files by identity.
4. Reads memory files and categorizes by source.
5. Concatenates global, extension, project, and user memory with MCP instructions.
6. JIT-discovers subdirectory memory for accessed paths.
7. Tracks loaded identities and emits memory change events.

Lifecycle behavior:
- Memory can refresh and discover context lazily.
- Loaded file identities prevent duplicate inclusion.

Failure behavior:
- Untrusted folders skip project/environment memory.
- Missing files are naturally absent from loaded context.

Tests inspected:
- No memory manager tests read in this run.

Limitations:
- Concatenated instruction text is vulnerable to authority confusion unless classified.

AgentCode extraction decision: ADAPT. Use hierarchical discovery and JIT locality with explicit scope/trust/provenance.

### Gemini Tool Output Masking and Node Truncation

Sources: `processors/toolMaskingProcessor.ts`; `processors/nodeTruncationProcessor.ts`

Important symbols:
- `createToolMaskingProcessor`
- `UNMASKABLE_TOOLS`
- `handleMasking`
- `createNodeTruncationProcessor`
- `truncateProportionally`
- `replacesId`

Control flow:
1. Tool masking scans `TOOL_EXECUTION` nodes for string fields above token threshold.
2. It writes full output to project temp `tool-outputs/session-<id>/...hash.txt`.
3. It replaces oversized text with a pointer containing size/line metadata.
4. Control tools in `UNMASKABLE_TOOLS` bypass masking.
5. Node truncation finds oversized text nodes, truncates proportionally, inserts omitted marker, and emits node with `replacesId`.

Lifecycle behavior:
- Full output remains retrievable outside prompt.
- Masked/truncated nodes preserve relationship to original evidence.

Failure behavior:
- Truncation only replaces when tokens are actually saved.

Tests inspected:
- No masking/truncation tests read in this run.

Limitations:
- Project temp files are not enough for AgentCode durable evidence.
- Mask pointers must include evidence-store ids and redaction policy.

AgentCode extraction decision: TAKE/ADAPT. Use masking, hashes, and replacement provenance; move raw storage to Evidence Store.

### Letta Code Memory Blocks

Source: `src/agent/memory.ts`

Important symbols:
- `MEMORY_BLOCK_LABELS`
- `READ_ONLY_BLOCK_LABELS`
- `parseMdxFrontmatter`
- `getDefaultMemoryBlocks`
- `cachedMemoryBlocks`

Control flow:
1. Loads embedded MDX prompt files as default memory blocks.
2. Parses simple frontmatter.
3. Labels blocks such as persona/human and marks configured labels read-only.
4. Caches default block construction.

Lifecycle behavior:
- Blocks are reusable initialization memory.
- Read-only labels constrain mutation.

Failure behavior:
- Malformed/missing frontmatter falls back to parsed body/default metadata.

Tests inspected:
- No memory block tests read in this run.

Limitations:
- Persona/human memory does not map directly to AgentCode project truth.

AgentCode extraction decision: IGNORE for authority; ADAPT labels/read-only idea for memory categories.

### Letta Code Memory Filesystem

Sources: `src/agent/memory-filesystem.ts`; `memory-filesystem.test.ts`; `memory-runtime.ts`

Important symbols:
- `MEMORY_FS_ROOT`
- `getMemoryFilesystemRoot`
- `getScopedMemoryFilesystemRoot`
- `resolveScopedMemoryDir`
- `ensureMemoryFilesystemDirs`
- `stampMemfsTagOnCreateBody`
- `prepareRawCreateAgentBodyForMemfs`
- `hydrateMemfsSettingFromAgent`
- `isMemfsEnabledOnServer`
- `ensureLocalMemfsCheckout`
- `isActiveMemfsEnabled`
- `getActiveMemoryDirectory`
- `isLocalMemfsActive`

Control flow:
1. Memory root defaults to `~/.letta/agents/<agentId>/memory`.
2. Scoped memory root is resolved from explicit agent id, runtime context, env variables, or backend settings.
3. Agent creation is stamped with a Git-memory-enabled tag so remote sync can be completed later.
4. Runtime gates memory directory access on local backend or per-agent memory setting.
5. Local checkout is initialized or remote memory repo is cloned when enabled.

Lifecycle behavior:
- Memory filesystem can be local or remote-backed.
- Enablement is a runtime/backend capability.

Failure behavior:
- Missing scope returns no active memory directory.
- Tests verify endpoint validation rejects unsafe/non-canonical sync endpoints.

Tests inspected:
- `memory-filesystem.test.ts`: tests tag stamping, path labels, endpoint validation, and memory tree rendering.

Limitations:
- Env-variable scope cannot be AgentCode authority.
- File memory is evidence/knowledge input, not Kernel truth.

AgentCode extraction decision: ADAPT. Use scoped memory roots, enablement gates, and initialization tags under Kernel-owned scope.

### Letta Code Git-Backed Memory

Source: `src/agent/memory-git.ts`

Important symbols:
- `MemoryCommitAuthor`
- `CommitMemoryWriteParams`
- `CommitMemoryWriteResult`
- `MemoryWriteSyncMode`
- `normalizeCredentialBaseUrl`
- `formatGitCredentialHelperPath`
- `redactGitAuthInText`
- `redactGitAuthError`
- `isMemfsRemoteUrlForAgent`
- `isRepairableMemfsRemoteUrl`
- `getGitRemoteUrl`
- `buildNonInteractiveGitEnv`
- `runGit`
- `isRetryableGitTransientError`
- `isMissingCwdGitError`
- `runGitWithRetry`
- `commitMemoryWrite`
- `initializeLocalMemoryRepo`
- `pullMemory`
- `pushMemory`

Control flow:
1. Builds memory git remote URL `/v1/git/<agentId>/state.git`.
2. Builds noninteractive git env with prompts and credential UI disabled.
3. Runs git with auth/proxy/no-signing args and timeout/maxBuffer.
4. Redacts auth headers, credentials, and `sk-let-*` tokens from logs/errors.
5. Retries transient HTTP/network failures.
6. Initializes local repo, validates relative paths, writes memory files, and commits initial/changed memory.
7. Pull uses `--ff-only`, reset-to-remote repair, then rebase fallback.
8. Push establishes upstream.

Lifecycle behavior:
- Memory changes become commits.
- Local and remote sync modes differ but use common git execution wrapper.

Failure behavior:
- No pathspec/no change returns non-committed result.
- Missing cwd can be repaired.
- Auth and network errors are redacted before surfacing.

Tests inspected:
- Related memory filesystem tests only; no `memory-git` test read in this run.

Limitations:
- Git-backed memory must not become hidden agent memory or task truth.

AgentCode extraction decision: ADAPT. Use Git evidence and redaction patterns; Kernel owns memory fact acceptance.

### Letta Code Memory Confinement

Source: `src/memory-confinement.ts`

Important symbols:
- `createMemoryConfinementLauncher`

Control flow:
1. Creates a launcher that applies a fail-closed filesystem policy.
2. Allows broad host reads, writes to harness state/own memory, and blocks other agents' memory.
3. Throws when no supported kernel sandbox exists.

Lifecycle behavior:
- Memory writer execution is launched under confinement.

Failure behavior:
- Missing sandbox support fails closed.

Tests inspected:
- No confinement tests read in this run.

Limitations:
- AgentCode cannot copy donor permissions; it must apply stricter policy from WP09/WP10.

AgentCode extraction decision: TAKE principle, ADAPT policy. Memory writers need explicit confinement.

### Letta Code Message Search

Source: `src/backend/message-search.ts`

Important symbols:
- `searchMessagesForBackend`
- `warmMessageSearchCacheForBackend`
- `SearchMode`

Control flow:
1. Local backend uses local transcript full-text search.
2. Remote backend calls API search with `vector`, `fts`, or `hybrid` mode.
3. Local warm-cache call returns explicit no-op status.

Lifecycle behavior:
- Search mode depends on backend capability.
- Cache warming is capability-aware.

Failure behavior:
- Unsupported local warm returns no-op instead of pretending remote behavior.

Tests inspected:
- No message-search tests read in this run.

Limitations:
- Transcript/message search is retrieval only.

AgentCode extraction decision: ADAPT. Use retrieval modes and explicit no-op statuses; never transcript as authority.

### Graphiti Temporal Knowledge Graph

Sources: `graphiti_core/graphiti.py`; `graphiti_core/search/search.py`; `graphiti_core/nodes.py`

Important symbols:
- `Graphiti`
- `AddEpisodeResults`
- `AddBulkEpisodeResults`
- `AddTripletResults`
- `GraphDriver`
- `LLMClient`
- `EmbedderClient`
- `CrossEncoderClient`
- `store_raw_episode_content`
- `search`
- `SearchConfig`
- `SearchResults`
- `edge_search`
- `node_search`
- `episode_search`
- `community_search`
- `maximal_marginal_relevance`
- `rrf`
- `EpisodeType`
- `Node`
- `EntityNode`
- `EpisodicNode`

Control flow:
1. `Graphiti` constructs graph, LLM, embedder, and cross-encoder clients with namespaces and optional raw episode storage.
2. Episodes/triples/entities are added into a graph with group partitioning.
3. `search` validates group ids and returns empty results for empty query.
4. It computes query embedding only when cosine/MMR configuration requires it.
5. It concurrently runs edge, node, episode, and community searches.
6. It traces each phase and result count.
7. It combines fulltext, similarity, BFS, MMR, RRF, and cross-encoder rerankers.

Lifecycle behavior:
- Knowledge graph separates raw episodes from extracted nodes/edges.
- Group ids partition memory/search domains.
- Search is retrieval over graph evidence.

Failure behavior:
- Empty query short-circuits.
- Invalid group ids fail validation.

Tests inspected:
- No Graphiti tests inspected in this run.

Limitations:
- Extracted graph facts are probabilistic/derived and can conflict with source evidence.

AgentCode extraction decision: STUDY/ADAPT. Use temporal episodes, groups, and reranking concepts with strict provenance/conflict handling.

### RTK Output Compression

Sources: `src/core/filter.rs`; `src/core/toml_filter.rs`; `src/cmds/system/summary.rs`; `src/filters/README.md`

Important symbols:
- `FilterLevel`
- `Language`
- `FilterStrategy`
- `NoFilter`
- `MinimalFilter`
- `AggressiveFilter`
- `TomlFilterRegistry`
- `TomlFilterDef`
- `CompiledFilter`
- `summarize_output`
- `detect_output_type`
- `never_worse`

Control flow:
1. Filter registry resolves command filters in priority order: project, user global, built-in, passthrough.
2. Trust gates project/user filters; untrusted filters are skipped.
3. Compiled TOML pipeline can strip ANSI, replace patterns, match output, strip/keep lines, truncate lines, keep head/tail, cap max lines, and supply fallback on empty.
4. Summary command detects output type and summarizes test/build/log/list/json/generic output.
5. `never_worse` prevents filtered output from exceeding raw output usefulness/size.

Lifecycle behavior:
- Filters are deterministic and configurable.
- Inline filter tests document expected behavior.

Failure behavior:
- Untrusted filters do not run.
- Passthrough remains available.

Tests inspected:
- `src/filters/README.md` documents inline filter tests; no Rust test file read in this run.

Limitations:
- RTK filtering is not a security boundary.
- Compression can hide detail unless raw evidence remains available.

AgentCode extraction decision: TAKE/ADAPT. Use deterministic compression with raw-output evidence pointers, trust gating, and never-worse guard.

## Architecture Patterns

- Context is rendered, not authoritative.
- Raw evidence, generated summaries, retrieved memories, and Kernel facts need separate types.
- Compression requires content hashes and provenance links.
- Memory updates require scoped ownership, conflict/supersession history, confidence, and freshness.
- Retrieval ranking may use fulltext, vectors, graph search, MMR, RRF, and cross-encoder scoring, but returned context remains advisory.

## AgentCode Limitations to Preserve

- Transcript must never be source of truth.
- Embeddings and graph facts must never outrank raw evidence or Kernel state.
- `CONTEXT.md` or generated summaries are derived views.
- High-confidence stale memory is stale.
- Replacement Worker/AgentSession must resume from Kernel/evidence/memory stores, not prior conversation.
