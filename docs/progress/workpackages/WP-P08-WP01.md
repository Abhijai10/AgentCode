# WP-P08-WP01 — Context Retrieval Integration

- **Phase:** P08
- **Status:** IMPLEMENTING
- **Risk:** MEDIUM | **Release scope:** REQUIRED_V1
- **Base commit:** 62439da380875cc06664b26f5cef9dcff17822c8
- **Accepted commit:** not accepted yet
- **Owner modules:** `crates/ac-agent`, `crates/ac-code-intel`, `crates/ac-context`
- **Architecture refs:** P01 WP07/WP08 implementation packets
- **Acceptance gates:** context retrieval partial; full P08 gates not complete

## Implemented In This Batch

- Connected ContextBuilder to CodeIntelligenceService for isolated worktree indexing.
- Added source-file collection for Rust, Markdown, TOML, and text files while skipping `.git` and `target`.
- Added goal-term search over indexed snippets and injected results as `RetrievalAccelerator` context.
- Preserved authority labels: Kernel goal state is protected; retrieval snippets are evidence/context only.
- Added targeted context retrieval test proving relevant file snippets are returned.

## Remaining Before Acceptance

- Incremental file watching.
- Dependency graph ranking.
- Persistent index storage and freshness checks.
