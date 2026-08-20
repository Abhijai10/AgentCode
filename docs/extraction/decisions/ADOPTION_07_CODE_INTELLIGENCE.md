# WP07 Code Intelligence Adoption Decision

## Decision Summary

AgentCode will ADAPT a layered, read-only Code Intelligence fabric:

1. repository inventory,
2. lexical search,
3. structural AST/query extraction,
4. symbol graph/index artifacts,
5. ranked context-pack candidates.

All outputs are derived evidence. Kernel remains owner of task truth; Git/files remain source evidence; Context Engine owns prompt assembly.

## Mechanism Decisions

| Mechanism | Donor evidence | Classification | AgentCode decision |
|---|---|---:|---|
| Tree-sitter parser lifecycle | tree-sitter `parser.c`, `parser.h`, `query.c` at `f235c2f1c399` | TAKE | Use parser/old-tree/included-range/query model for syntax artifacts. Persist parser/grammar/query versions and source hashes. |
| Aider tag tuple | Aider `aider/repomap.py` at `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` | TAKE | Use `Tag`-like definition/reference records as compact symbol evidence. |
| Aider repo-map rendering | Aider `RepoMap.get_ranked_tags_map_uncached` | ADAPT | Reuse budget-fitting idea for Context Engine candidates, but store structured evidence before rendering. |
| Aider PageRank symbol/file ranking | Aider `RepoMap.get_ranked_tags` | ADAPT | Use as one scoring feature, combined with task scope, recency, lexical hits, and Kernel-provided goals. |
| Aider mtime DiskCache | Aider `RepoMap.get_tags` | REJECT | Replace with content hash plus commit/worktree/parser/query identity. mtime is not sufficient evidence. |
| Aider cache failure fallback | Aider `get_tags` SQLite fallback | ADAPT | Keep degraded-mode behavior but persist degraded evidence and observability event. |
| Cline host-index then ripgrep fallback | Cline `executeHostIndexForFiles`, `executeRipgrepForFiles` at `8a038022a439f401d78764a059e1561578848e81` | ADAPT | Use query source fallback with explicit `source` and degradation evidence. |
| Cline active-file boost | Cline `getActiveFiles`, `searchWorkspaceFiles` | TAKE | Include runtime-visible files as ranking feature only. |
| Cline symlink-following ripgrep | Cline `rg --files --follow --hidden` | REJECT | Follow symlinks only when policy and sandbox approve; default must not cross repository boundaries silently. |
| OpenHands bounded workspace listing | OpenHands `use-workspace-files.ts` at `b25f9b3969f924f37440fee908ff35309ec6eea2` | ADAPT | Use bounded inventory for UI/initial context, but not as complete index. |
| OpenHands local/cloud transport split | OpenHands `useLocalWorkspaceFiles`, `useCloudWorkspaceFiles` | TAKE | Preserve backend-specific file inventory transports behind one interface. |
| ast-grep structural rule engine | ast-grep `scan.rs` at `0eb08389b6c4` | WRAP | Expose as controlled structural query tool with JSON/SARIF output evidence and rule hash. |
| ast-grep project config as policy | ast-grep `ScanWithConfig::try_new` | REJECT | Project-provided rules are untrusted input unless admitted by AgentCode policy. |
| ripgrep lexical search | ripgrep `search.rs` at `3fce3b5bb023` | WRAP | Use as deterministic lexical evidence source with command args, stats, and output hashes. |
| ripgrep preprocessors/compressed search | ripgrep `SearchWorkerBuilder` config | REJECT | Disable for untrusted repos by default because preprocessors can execute or expand risky content. |
| SCIP index schema | SCIP `scip.proto`, `symbol.go` at `8b8c4fc0dea6` | ADAPT | Use SCIP-like document/symbol/occurrence schema and path constraints for portable symbol artifacts. |
| SCIP external symbols | SCIP `Index.external_symbols` | STUDY | Useful for dependencies, but V1 should focus on repository-local symbols unless a trusted indexer exists. |
| Zoekt option hash | Zoekt `HashOptions`, `GetHash` at `dcd8c9ca9b84` | TAKE | Hash index settings to decide rebuild compatibility. |
| Zoekt delta/tombstone shards | Zoekt `IsDelta`, `MarkFileAsChangedOrRemoved`, `SetTombstone` | ADAPT | Use delta invalidation/tombstone concepts for derived index artifacts. |
| Zoekt full search service | Zoekt builder/gitindex | WRAP | Potential large-repo acceleration only; not required for initial Code Intelligence implementation. |
| Ctags-like fallback | Zoekt ctags options; Aider Pygments fallback | IGNORE for V1 | Keep under study; tree-sitter and lexical search are enough for first implementation slice. |

## Required AgentCode Posture

- Code Intelligence is an evidence provider.
- It has no write path to the repository.
- It cannot mark task progress complete.
- It cannot silently refresh task assumptions without Kernel provenance.
- It may expose stale/degraded results, but consumers must see freshness and confidence.

## Rejected Donor Couplings

- Prompt-rendered repo maps as stored truth.
- mtime-only invalidation.
- UI file lists as complete repository indexes.
- Unbounded symlink traversal.
- Project-supplied structural rules as trusted policy.
- Search index hits as proof of absence.
