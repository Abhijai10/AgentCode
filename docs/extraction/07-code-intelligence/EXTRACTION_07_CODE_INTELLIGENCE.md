# WP07 Code Intelligence Extraction

## Scope

Phase 1 WP07 extracts code intelligence mechanisms only. AgentCode adaptation target: a read-only Code Intelligence service that supplies evidence and context to Kernel/Context Engine, never task truth and never direct edits.

## Donors Inspected

| Donor | Pinned commit | Files inspected |
|---|---:|---|
| Aider | `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` | `aider/repomap.py`; `tests/basic/test_repomap.py` |
| Cline | `8a038022a439f401d78764a059e1561578848e81` | `apps/vscode/src/services/search/file-search.ts` |
| OpenHands | `b25f9b3969f924f37440fee908ff35309ec6eea2` | `src/hooks/query/use-workspace-files.ts` |
| ast-grep | `0eb08389b6c4` | `crates/cli/src/scan.rs` |
| ripgrep | `3fce3b5bb023` | `crates/core/search.rs` |
| tree-sitter | `f235c2f1c399` | `lib/src/parser.c`; `lib/src/parser.h`; `lib/src/query.c` |
| SCIP | `8b8c4fc0dea6` | `scip.proto`; `bindings/go/scip/symbol.go` |
| Zoekt | `dcd8c9ca9b84` | `index/builder.go`; `gitindex/index.go`; `index/tombstones.go` |

## Mechanisms Discovered

### Aider Repo Map

Source: `aider/repomap.py`

Important symbols:
- `Tag`: derived symbol evidence tuple with `rel_fname`, `fname`, `line`, `name`, `kind`.
- `RepoMap.__init__`: owns `TAGS_CACHE_DIR`, `CACHE_VERSION`, `tree_cache`, `tree_context_cache`, `map_cache`, token budgets.
- `RepoMap.get_repo_map`: entry point used by chat orchestration; accepts chat files, other files, mentioned filenames, mentioned identifiers, force-refresh.
- `RepoMap.get_tags`: cache lookup and invalidation.
- `RepoMap.get_tags_raw`: parser/query extraction.
- `RepoMap._run_captures`: tree-sitter query capture execution.
- `RepoMap.get_ranked_tags`: graph construction and ranking.
- `RepoMap.get_ranked_tags_map_uncached`: context-budget rendering.

Control flow:
1. `get_repo_map` receives current visible files plus candidate repo files.
2. It adjusts `max_map_tokens` upward when no chat files are present and a larger model context is available.
3. It calls `get_ranked_tags_map`, which caches by chat files, candidate files, mentions, and token budget unless `force_refresh`.
4. `get_tags` compares file mtime with cached metadata in DiskCache `.aider.tags.cache.v{CACHE_VERSION}`.
5. Cache hit returns tags; SQLite cache failure recreates cache or falls back to an in-memory dict.
6. Cache miss calls `get_tags_raw`, which maps filename to language, loads the tree-sitter language and tags query, parses bytes, runs captures, and emits definition/reference tags.
7. If tree-sitter yields definitions without references, Pygments name tokens backfill references.
8. `get_ranked_tags` builds `defines`, `references`, and `definitions`, then a `networkx.MultiDiGraph`.
9. Personalization boosts chat files, mentioned filenames, mentioned identifiers, and path components; ranking downranks private names and symbols with many definers.
10. PageRank produces file/symbol ranking; ranked tags are rendered into line-focused trees.
11. `get_ranked_tags_map_uncached` prepends important files and binary-searches tag count to fit token budget.

Lifecycle behavior:
- Index entries are lazy and file-scoped.
- Cache identity is mtime plus cache version, not commit/content hash.
- Rendered context is rebuilt when file set, mention set, force-refresh, or budget changes.

Failure behavior:
- `RecursionError` disables repo-map generation for that call and reports warning.
- Tree-sitter parse/query failures fall back per file.
- DiskCache errors recreate or demote cache to dict.
- Missing language/query returns no tags.

Tests inspected:
- `tests/basic/test_repomap.py`
- `test_get_repo_map`: confirms mapped files appear.
- `test_repo_map_refresh_files`: cache changes when file list changes.
- `test_repo_map_refresh_auto`: slow map refresh is skipped unless forced.
- `test_get_repo_map_with_identifiers`: identifiers influence map output.

Limitations:
- mtime cache is insufficient for AgentCode evidence because Git commit, worktree, grammar, query version, and content hash must be recorded.
- Ranking is useful but heuristic; it cannot become authority.
- Rendered repo map is optimized for prompt context, not durable index storage.

AgentCode extraction decision: ADAPT. Use tag extraction, graph ranking, and token-budget rendering as derived Code Intelligence artifacts with provenance.

### Cline Workspace Search

Source: `apps/vscode/src/services/search/file-search.ts`

Important symbols:
- `FileSearchSource`
- `RipgrepError`
- `executeRipgrepForFiles`
- `resolveRipgrepPath`
- `findSystemRipgrep`
- `getActiveFiles`
- `executeHostIndexForFiles`
- `searchWorkspaceFiles`
- `OrderbyMatchScore`
- `searchWorkspaceFilesMultiroot`

Control flow:
1. `searchWorkspaceFiles` collects active editor files for boost/dedupe.
2. It tries host-native index search when available.
3. If host index is missing or fails, it falls back to `executeRipgrepForFiles`.
4. Ripgrep runs `rg --files --follow --hidden` with ignored heavy directories.
5. Output is streamed line-by-line; hitting result limit kills the process intentionally.
6. Results are fuzzy-ranked through `fzf`, with filename weighted more than path.
7. Returned items include a `source` marker indicating host index or ripgrep.
8. `searchWorkspaceFilesMultiroot` searches roots independently and swallows per-root errors unless all roots fail.

Lifecycle behavior:
- File search is an ephemeral query, not a persistent repository index.
- Active files affect ranking at query time.
- Host index is preferred when available; ripgrep is fallback.

Failure behavior:
- Host index unimplemented/failure returns `null` and allows fallback.
- Ripgrep non-zero is tolerated when caused by limit-triggered kill and partial results exist.
- Multi-root failures degrade unless every root fails.

Tests inspected:
- No dedicated test file inspected for Cline file search in this run.

Limitations:
- `--follow` can cross symlink boundaries and must be policy-controlled in AgentCode.
- Search evidence is result set plus command metadata, not complete code understanding.

AgentCode extraction decision: WRAP/ADAPT. Wrap process execution through Tool Broker; adapt source/fallback reporting and active-file ranking.

### OpenHands Workspace File Listing

Source: `src/hooks/query/use-workspace-files.ts`

Important symbols:
- `MAX_FILES`
- `EXCLUDED_DIRS`
- `buildListCommand`
- `normalizePath`
- `useLocalWorkspaceFiles`
- `useCloudWorkspaceFiles`
- `useWorkspaceFiles`

Control flow:
1. `useWorkspaceFiles` reads active backend from the backend-registry external store.
2. Local backend calls `AgentServerRuntimeService.executeCommand` with a bounded `find` command in the conversation working directory.
3. Cloud backend calls `listCloudConversationFiles` with the conversation id and absolute workspace path.
4. Both paths normalize leading `./`, dedupe, slice to `MAX_FILES`, and disable retries.

Lifecycle behavior:
- UI file list is query-cached for 30 seconds with 5 minute garbage collection.
- Local and cloud paths deliberately use backend-specific transports.

Failure behavior:
- Local non-zero command exit throws.
- Query metadata suppresses toast noise.
- Query is disabled until runtime/conversation/workspace data are ready.

Tests inspected:
- No OpenHands workspace-file tests inspected in this run.

Limitations:
- `find` listing is bounded and UI-oriented, not a durable index.
- It proves useful transport separation, not semantic intelligence.

AgentCode extraction decision: ADAPT. Keep bounded, backend-aware listing as repository inventory evidence only.

### ast-grep Structural Scan

Source: `crates/cli/src/scan.rs`

Important symbols:
- `ScanArg`
- `run_with_config`
- `run_scan`
- `ScanWithConfig`
- `ScanStdin`
- `RuleCollection`
- `RuleConfig`
- `ScanTrace`
- `MaxItemCounter`

Control flow:
1. CLI parses scan args for inline rule, config rule, output format, stdin, paths, and max result limit.
2. `run_scan` chooses stdin scanning or project/path scanning.
3. `ScanWithConfig::try_new` loads project config, overwrites rules when CLI rule is supplied, canonicalizes project dir, and builds a max-results counter.
4. Workers scan files, emit rule matches through printer formats including JSON/SARIF/GitHub/cloud, and emit scan traces.
5. Diagnostic errors surface when rule violations/errors exist.

Lifecycle behavior:
- Rules/configuration own the structural query contract.
- Project scan has bounded output controls and trace hooks.

Failure behavior:
- Invalid config/rule returns diagnostics before scan.
- Worker error count causes non-success scan result.

Tests inspected:
- No ast-grep tests inspected in this run.

Limitations:
- ast-grep rules are external policy/query input; results are only as reliable as rule provenance.
- AgentCode must sandbox invocation and persist rule version/hash.

AgentCode extraction decision: WRAP. Use as optional structural query engine with JSON/SARIF evidence.

### ripgrep Lexical Search

Source: `crates/core/search.rs`

Important symbols:
- `SearchWorkerBuilder`
- `Config`
- `SearchResult`
- `PatternMatcher`
- `Printer`

Control flow:
1. `SearchWorkerBuilder` configures matcher, printer, binary handling, compressed search, preprocessing, and threading support.
2. Search workers consume file paths and produce `SearchResult`.
3. `SearchResult` exposes whether matches occurred and optional stats.
4. Printers can emit standard, summary, or structured JSON output.

Lifecycle behavior:
- Search is stateless per invocation but can supply deterministic evidence.
- Binary detection differs for implicit discovered files versus explicit files.

Failure behavior:
- Binary/preprocessor/compressed-file handling is controlled by config.
- Search errors are surfaced in result/printer path.

Tests inspected:
- No ripgrep tests inspected in this run.

Limitations:
- Lexical search is not semantic code understanding.
- Preprocessors are unsafe for untrusted repos unless sandboxed and policy-approved.

AgentCode extraction decision: WRAP. Use lexical search as a controlled evidence source.

### tree-sitter Parser Lifecycle

Sources: `lib/src/parser.c`; `lib/src/parser.h`; `lib/src/query.c`

Important symbols:
- `ts_parser_parse`
- `ts_parser_parse_with_options`
- `ts_parser_parse_string`
- `ts_parser_set_included_ranges`
- `ts_parser_included_ranges`
- `ts_parser_set_logger`
- `ts_parser_logger`
- `ts_range_array_get_changed_ranges`

Control flow:
1. Parser receives source input and optional old tree.
2. Incremental parse reuses old tree and computes changed ranges.
3. Included ranges limit parse scope.
4. Query engine runs separately over parsed syntax trees.
5. Logger hooks expose parser tracing.

Lifecycle behavior:
- Parser object owns current language/configuration.
- Old tree enables incremental updates, but cache identity must include source hash, grammar version, and query version.

Failure behavior:
- Parse can produce error nodes rather than fail hard.
- Invalid included ranges/query configuration must be treated as degraded extraction.

Tests inspected:
- No tree-sitter tests inspected in this run.

Limitations:
- Parser correctness depends on grammar and query quality.
- Syntax tree is structural evidence, not task truth.

AgentCode extraction decision: TAKE/ADAPT. Use parser lifecycle and incremental ranges; adapt cache/evidence identity.

### SCIP Symbol Index Contract

Sources: `scip.proto`; `bindings/go/scip/symbol.go`

Important symbols:
- `Index`
- `Metadata`
- `ToolInfo`
- `Document`
- `Occurrence`
- `SymbolInformation`
- `PositionEncoding`
- `IsGlobalSymbol`
- `IsLocalSymbol`
- `ParseSymbol`
- `ValidateSymbolUTF8`

Control flow:
1. Indexer emits an `Index` rooted at a single `Metadata.project_root`.
2. `metadata` appears first for streaming consumption.
3. Each `Document` uses a canonical relative path, language string, occurrences, and defined symbol metadata.
4. External symbols can be provided separately.
5. Symbol strings are parsed/validated with global/local distinctions.

Lifecycle behavior:
- Index payload can be streamed by field to reduce memory.
- Tool name/version/arguments are part of index metadata.

Failure behavior:
- Invalid symbol strings fail parser/validator.
- Document path contract rejects absolute paths, symlinks, non-canonical paths, and non-regular files.

Tests inspected:
- No SCIP tests inspected in this run.

Limitations:
- SCIP is an exchange format; precision depends on the producing indexer.
- AgentCode should not require all languages to have SCIP support in V1.

AgentCode extraction decision: ADAPT. Use SCIP-like schema concepts for future portable symbol artifacts.

### Zoekt Indexing and Delta Builds

Sources: `index/builder.go`; `gitindex/index.go`; `index/tombstones.go`

Important symbols:
- `Options`
- `Branch`
- `HashOptions`
- `GetHash`
- `Builder`
- `AddFile`
- `Add`
- `MarkFileAsChangedOrRemoved`
- `Finish`
- `IsDelta`
- `changedOrRemovedFiles`
- `ShardMax`
- `FindAllShards`
- `Tombstone`
- `prepareDeltaBuild`
- `prepareNormalBuild`
- `createDocument`
- `indexCatfileBlobs`
- `SetTombstone`
- `UnsetTombstone`

Control flow:
1. `Options` defines index directory, shard sizing, repo metadata, branch list, subrepos, ctags options, size limits, and delta mode.
2. `HashOptions/GetHash` hashes settings that invalidate index compatibility.
3. Git indexing prepares normal or delta build depending on prior shards and changed file set.
4. Builder adds files into shards.
5. Changed/removed files are marked for delta tombstoning.
6. `Finish` writes new shards, tombstones stale docs in old shards, and merges/deletes/preserves old shards according to delta settings.

Lifecycle behavior:
- Shards are durable derived indexes.
- Delta updates avoid rebuilding unaffected docs.
- Tombstones preserve old shard files while invalidating stale documents.

Failure behavior:
- Delta falls back when changed file threshold or prior state makes delta unsuitable.
- Finish path must complete copy/tombstone/delete consistently.

Tests inspected:
- No Zoekt tests inspected in this run.

Limitations:
- Zoekt is text-search infrastructure, not semantic code truth.
- Shard/index state must be rebuilt from Git/file evidence, never trusted as sole source.

AgentCode extraction decision: STUDY/WRAP. Use cache identity, delta, and tombstone concepts; wrap only if large-repo search acceleration is required.

## Architecture Patterns

- Separate inventory, lexical search, syntax tags, symbol index, and rendered context. They have different freshness and precision.
- Every artifact needs provenance: repo root, worktree id, commit SHA, file path, content hash, language, parser/indexer version, query/rule version, generated time, and failure/degraded marker.
- Incremental updates should be content-hash and dependency-aware, not mtime-only.
- Ranking can combine lexical hits, symbol graph centrality, active files, mentions, recency, and Kernel task scope; ranking is advisory.
- Code Intelligence outputs must be consumed through Context Engine or Kernel evidence references, never direct mutation paths.

## AgentCode Limitations to Preserve

- Code Intelligence must not write files or patches.
- Code Intelligence must not own task state, completion state, or memory truth.
- Derived indexes are disposable and rebuildable.
- Search/index tools execute only through controlled Tool Broker/sandbox boundaries.
- Stale high-confidence symbols remain stale.
