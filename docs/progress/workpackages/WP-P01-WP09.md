# WP-P01-WP09 — Sandbox and Permissions Extraction Campaign

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** 3f8c6916f83e124d88dbbb59106ee662e6624925
- **Accepted commit:** e059aedcb66a32874845766e849ea5783324c10b
- **Owner modules:** docs/extraction/, docs/extraction/packets/, docs/extraction/decisions/
- **Architecture refs:** DOC-04; DOC-07 H46; DOC-10 P1-G4; DOC-11 P01-WP09
- **Acceptance gates:** P1-G4 extraction evidence

## Objective

Create implementation-grade sandbox and permissions extraction artifacts covering policy representation, command interception, cwd/workspace boundaries, filesystem and network scope, environment filtering, approval, sandbox launch, failure, and process cleanup.

## Outputs

- `docs/extraction/09-sandbox-permissions/EXTRACTION_09_SANDBOX_PERMISSIONS.md`
- `docs/extraction/decisions/ADOPTION_09_SANDBOX_PERMISSIONS.md`
- `docs/extraction/packets/P01_WP09_SANDBOX_PERMISSIONS_IMPLEMENTATION_PACKET.md`

## Donors Inspected

Codex, Gemini CLI, OpenHands, SWE-ReX, Cline, and OpenCode at pinned catalog SHAs.

## Evidence

Extraction cites exact source paths, important symbols, control flow, lifecycle, failure behavior, tests inspected, and AgentCode adoption decisions. No production implementation code was added.

## Handoff

Future sandbox, Tool Broker, Process Manager, and permission-policy implementation phases consume the implementation packet. Phase 1 remains IN_PROGRESS; Phase 2 remains NOT_STARTED.
