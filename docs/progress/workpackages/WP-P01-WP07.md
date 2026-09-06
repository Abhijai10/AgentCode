# WP-P01-WP07 — Code Intelligence Extraction Campaign

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** f18fcaf8bbb04470850f3ae2b5177dea09d1ed6a
- **Accepted commit:** c2de2290a4dea6176d68064906470210f522d59e
- **Owner modules:** docs/extraction/, docs/extraction/packets/, docs/extraction/decisions/
- **Architecture refs:** DOC-04; DOC-07 H43; DOC-10 P1-G4/P1-G8; DOC-11 P01-WP07
- **Acceptance gates:** P1-G4 extraction evidence; P1-G8 future code-intelligence evidence requirements

## Objective

Create implementation-grade code intelligence extraction artifacts covering repository indexing, AST parsing, symbol discovery, dependency/symbol graphs, code search, incremental updates, file inventory, and retrieval ranking.

## Outputs

- `docs/extraction/07-code-intelligence/EXTRACTION_07_CODE_INTELLIGENCE.md`
- `docs/extraction/decisions/ADOPTION_07_CODE_INTELLIGENCE.md`
- `docs/extraction/packets/P01_WP07_CODE_INTELLIGENCE_IMPLEMENTATION_PACKET.md`

## Donors Inspected

Aider, Cline, OpenHands, ast-grep, ripgrep, tree-sitter, SCIP, and Zoekt at pinned catalog/repository SHAs.

## Evidence

Extraction cites exact source paths, important symbols, control flow, lifecycle, failure behavior, tests inspected where used, limitations, and AgentCode adoption decisions. No production implementation code was added.

## Handoff

Future Code Intelligence implementation phases consume the implementation packet. Phase 1 remains IN_PROGRESS; Phase 2 remains NOT_STARTED.
