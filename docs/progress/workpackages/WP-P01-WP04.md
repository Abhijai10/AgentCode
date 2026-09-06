# WP-P01-WP04 — Agent Runtime Extraction Campaign

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** e365e2eff81010663622340bb45672d2ae7036d2
- **Accepted commit:** 7ee6c2fc6939ad4b8a88200ab08d79fa02a309e2
- **Owner modules:** docs/extraction/, docs/extraction/packets/, docs/extraction/decisions/
- **Architecture refs:** DOC-03; DOC-07 §122-123; DOC-10 P1-G4/P1-G8; DOC-11 P01-WP04
- **Acceptance gates:** P1-G4, P1-G8 extraction evidence

## Objective

Create implementation-grade agent runtime extraction artifacts covering runtime
entrypoints, session lifecycle, model/tool loop, continuation, event streams,
cancellation, retries, checkpointing, resume, state ownership, and worker/session
boundaries.

## Outputs

- `docs/extraction/04-agent-runtime/EXTRACTION_04_AGENT_RUNTIME.md`
- `docs/extraction/decisions/ADOPTION_04_AGENT_RUNTIME.md`
- `docs/extraction/packets/P01_WP04_AGENT_RUNTIME_IMPLEMENTATION_PACKET.md`

## Donors Inspected

Codex, OpenHands, mini-SWE-agent, software-agent-sdk, Gemini CLI at pinned catalog
SHAs.

## Evidence

Extraction separates durable Kernel state from temporary Worker/AgentSession state
and explicitly rejects transcript-as-source-of-truth. No production implementation
code was added.

## Handoff

Phase 6 Basic Worker Agent Loop and Phase 12 Full Autonomy Kernel & Multi-Agent
Runtime consume the implementation packet. Phase 1 remains IN_PROGRESS; Phase 2
remains NOT_STARTED.
