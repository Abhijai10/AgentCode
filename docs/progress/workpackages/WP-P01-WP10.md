# WP-P01-WP10 — Security / Isolation Extraction Campaign

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 3f8c6916f83e124d88dbbb59106ee662e6624925
- **Accepted commit:** e059aedcb66a32874845766e849ea5783324c10b
- **Owner modules:** docs/extraction/, docs/extraction/packets/, docs/extraction/decisions/
- **Architecture refs:** DOC-02; DOC-04; DOC-07 H46 plus security isolation sections; DOC-10 P1-G4; DOC-11 P01-WP10
- **Acceptance gates:** P1-G4 extraction evidence

## Objective

Create implementation-grade security/isolation extraction artifacts for treating tools, plugins, repositories, generated code, hooks, MCP-style extensions, and security-tool output as untrusted capability-requesting surfaces.

## Outputs

- `docs/extraction/10-security-isolation/EXTRACTION_10_SECURITY_ISOLATION.md`
- `docs/extraction/decisions/ADOPTION_10_SECURITY_ISOLATION.md`
- `docs/extraction/packets/P01_WP10_SECURITY_ISOLATION_IMPLEMENTATION_PACKET.md`

## Donors Inspected

Codex, Gemini CLI, Cline, OpenHands, and Munder Difflin at pinned catalog SHAs.

## Evidence

Extraction cites exact source paths, important symbols, control flow, lifecycle, failure behavior, tests inspected, and AgentCode adoption decisions. No production implementation code was added.

## Handoff

Future skills/hooks/MCP, security, Tool Broker, and isolation phases consume the implementation packet. Phase 1 remains IN_PROGRESS; Phase 2 remains NOT_STARTED.
