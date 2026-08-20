# WP-P01-WP05 — Tools and Editing Extraction Campaign

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** e365e2eff81010663622340bb45672d2ae7036d2
- **Accepted commit:** 7ee6c2fc6939ad4b8a88200ab08d79fa02a309e2
- **Owner modules:** docs/extraction/, docs/extraction/packets/, docs/extraction/decisions/
- **Architecture refs:** DOC-04; DOC-07 §124-125; DOC-10 P1-G4; DOC-11 P01-WP05
- **Acceptance gates:** P1-G4 extraction evidence

## Objective

Create implementation-grade tools/editing extraction artifacts covering tool
contracts, lifecycle, permissions, approval boundaries, command execution, edit
strategies, failure recovery, and validation hooks.

## Outputs

- `docs/extraction/05-tools-editing/EXTRACTION_05_TOOLS_EDITING.md`
- `docs/extraction/decisions/ADOPTION_05_TOOLS_EDITING.md`
- `docs/extraction/packets/P01_WP05_TOOLS_EDITING_IMPLEMENTATION_PACKET.md`

## Donors Inspected

Codex, Aider, Gemini CLI, Cline, OpenHands, OpenCode, mini-SWE-agent, and
software-agent-sdk at pinned catalog SHAs.

## Evidence

Extraction cites exact source paths, important symbols, control flow, lifecycle,
failure behavior, tests inspected, and AgentCode adoption decisions. No production
implementation code was added.

## Handoff

Phase 5 Native Tool Runtime Foundation and Phase 13 Advanced Edit Engine consume the
implementation packet. Phase 1 remains IN_PROGRESS; Phase 2 remains NOT_STARTED.
