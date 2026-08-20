# P01-WP07 Code Intelligence Implementation Packet

## Future Interfaces Required

### Code Intelligence Service

```text
CodeIntelligenceService
  indexRepository(scope: RepositoryScope, policy: IndexPolicy) -> IndexRunReceipt
  updateFiles(scope: RepositoryScope, changes: FileChangeSet) -> IndexRunReceipt
  querySymbols(query: SymbolQuery) -> SymbolQueryResult
  searchText(query: TextSearchQuery) -> TextSearchResult
  searchStructure(query: StructuralQuery) -> StructuralQueryResult
  rankContextCandidates(request: ContextCandidateRequest) -> ContextCandidateResult
  explainArtifact(artifactId: CodeIntelArtifactId) -> EvidenceReference
```

### Core Data Shapes

- `RepositoryScope`: repository id, worktree id, root path, current commit SHA, ignore policy, trust profile.
- `SourceFileIdentity`: canonical relative path, file kind, content hash, size, language, encoding, symlink status.
- `ParserIdentity`: parser engine, grammar id/version/hash, query id/version/hash.
- `SymbolOccurrence`: file identity, symbol id, name, kind, role, range, line, extraction method, confidence.
- `CodeIndexArtifact`: artifact id, scope, source file identities, parser/indexer identities, created time, stale marker, degraded marker, evidence refs.
- `ContextCandidate`: source span/tree/search hit, score components, freshness, confidence, why selected.

## Ownership Boundaries

| Component | Owns | Must not own |
|---|---|---|
| Kernel | task state, mission state, completion truth, accepted evidence references | derived index internals |
| Code Intelligence | rebuildable index artifacts, symbol/search query results, ranking explanations | task truth, memory truth, file edits |
| Context Engine | context-pack assembly and token budgeting | repository mutation, durable task truth |
| Tool Broker / Sandbox | process execution for ripgrep, ast-grep, external indexers | interpretation of results as truth |
| Git/Worktree service | commit/worktree evidence, changed files | code intelligence scoring |

## Data Ownership

- Git commit and working tree content are source evidence.
- Code index artifacts are derived, rebuildable data.
- ASTs, tags, SCIP-like documents, and search hits require provenance.
- Rendered snippets are views over evidence, not authoritative records.

## State Ownership

- Durable Kernel state records which code intelligence artifacts were used as evidence.
- Code Intelligence caches may be discarded and rebuilt.
- Per-file index freshness is keyed by content hash plus parser/query/indexer identity.
- Staleness is explicit when worktree commit or file hash changes after artifact generation.

## Incremental Update Requirements

- Detect file changes from Git/worktree change set and file watcher events.
- Re-index only affected files for lexical/AST artifacts.
- Invalidate dependent symbol graph/ranking artifacts when definitions/references change.
- Use tombstone records for deleted/moved files until dependent indexes compact.
- Record partial/degraded index runs rather than pretending freshness.

## Security Constraints

- No direct writes to repository files.
- External commands run only through Tool Broker with sandbox policy, timeout, cancellation, stdout/stderr capture, and provenance.
- Do not follow symlinks by default.
- Project-provided ast-grep rules, indexer configs, and generated files are untrusted.
- Disable preprocessors or interpreters unless explicitly policy-approved.
- Search/index artifacts must avoid storing secrets beyond evidence retention policy.

## Failure and Degraded Modes

- Parser unavailable: return lexical-only degraded result.
- Grammar/query failure: mark affected files degraded.
- Cache corruption: rebuild cache, emit event, preserve failure evidence.
- Search command timeout/cancel: return partial results only if provenance marks partial.
- Large repository limits: return bounded inventory with omitted count and recommendation for background index.

## Verification Hooks for Future Phases

- Fixture with at least 10k files: modify 3 files and prove only affected file artifacts rebuild.
- Stale-artifact test: query after file hash changes must report stale or force refresh.
- Symlink test: external symlink is excluded by default.
- Degraded parser test: missing grammar falls back without crashing.
- Ranking explanation test: each selected context candidate exposes score factors and evidence source.

## Unresolved Design Questions

- Which languages are REQUIRED_V1 for tree-sitter extraction?
- Should AgentCode store SCIP-compatible protobuf directly or an internal normalized schema with optional SCIP export?
- How much symbol graph history should persist across commits?
- What is the first release threshold for introducing Zoekt-like sharded search?
- Which consumers can request background indexing versus synchronous bounded search?
