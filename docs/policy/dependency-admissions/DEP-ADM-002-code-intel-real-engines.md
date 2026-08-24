# DEP-ADM-002 — Code Intelligence Real Parser/LSP Dependencies

Need:
  Batch 3 real code intelligence requires syntax-tree parsing for supported languages and JSON-RPC/LSP payload handling. Required gates: Tree-sitter parser extraction, structural search/rewrite, LSP WorkspaceEdit normalization, and real language-server lifecycle.

Proposed dependencies:
  tree-sitter 0.26.13, tree-sitter-rust 0.24.2, tree-sitter-python 0.25.0, tree-sitter-javascript 0.25.0, tree-sitter-typescript 0.23.2, tree-sitter-go 0.25.0, lsp-types 0.97.0, serde 1.0.229, serde_json 1.0.151 from crates.io, pinned in Cargo.lock.

Existing alternative check:
  The existing ac-code-intel parser was line/string heuristic code and could not provide real syntax-tree provenance. The existing ac-changeset AstGrep/LspRename paths used literal replacement/identifier scanning and could not satisfy Batch 3.

License:
  tree-sitter and grammar crates: MIT. lsp-types: MIT. serde and serde_json: MIT OR Apache-2.0. Local license files inspected under ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/*/LICENSE*.

Security status:
  crates.io registry resolution completed through Cargo. No install scripts or network-at-runtime behavior are introduced by these crates. Advisory/CVE scan not run in this batch; final release remains subject to the repo dependency-check workflow.

Maintenance state:
  Tree-sitter and language grammars are actively maintained parser ecosystem crates. serde/serde_json are core Rust ecosystem crates. lsp-types is maintained for Language Server Protocol data structures.

Runtime/bundle cost:
  Adds parser grammars and JSON/LSP type dependencies to ac-code-intel. Runtime CPU cost is bounded to indexed files and incremental hash checks; no daemon or server is bundled.

Install/postinstall behavior:
  Cargo builds native grammar crates. No postinstall scripts, global installs, or downloaded language servers.

Why existing components are insufficient:
  Existing components were audited as fake/heuristic and mislabeled as AST/LSP. They cannot produce real syntax-tree nodes or parse LSP WorkspaceEdit payloads.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  Codex, 2026-08-25. Evidence: Cargo.lock entries; focused tests cargo test -p ac-code-intel and cargo test -p ac-changeset.
