# OmniRoute Base Record

Status: AgentCode-controlled in-repository implementation.
Recorded: 2026-08-21.

## Source Position

- Upstream repository: not vendored into this tree.
- Upstream base SHA: not applicable for this implementation run.
- AgentCode fork SHA: commit containing this file.
- Patch list: `crates/ac-provider/src/lib.rs` implements the Phase 4 OmniRoute-equivalent routing layer directly in the existing provider fabric.

## Policy

No donor source code was copied into AgentCode for Phase 4. The controlled route fabric is maintained as native AgentCode production code and remains behind the replaceable provider interfaces.

## Update Strategy

Future comparison against external OmniRoute releases requires a separate license/provenance review before importing code or behavior beyond the accepted Phase 1 extraction decisions.
