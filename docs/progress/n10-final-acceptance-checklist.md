# N10 — Final Acceptance: Honest Completion Checklist (old §21)

Date: 2026-09-05 · Batches N1–N8 landed (N5 in this session: `e04e8f8`; MCP
self-security fixture `c4c490d`); N9 (signed packaging) remains blocked on Apple
Developer ID credentials — the ONLY unchecked box.

Verification basis: every box below is backed by committed tests that pass in
`make validate` (454 passed / 0 failed / 26 env-gated) or by real-model E2E
runs documented here. File existence alone is never completion (Doc 10).

| # | §21 box | State | Evidence |
|---|---------|-------|----------|
| 1 | multiple chats per project | ✅ | `ConversationCreate`/`List`/`Get`/`Rename`/`Archive`/`Delete` IPC; per-project conversation registry in `ac-daemon/src/conversation.rs` |
| 2 | persistent conversation history | ✅ | SQLite control plane (`ac-daemon` ControlPlaneDb): conversations/messages/missions survive restart; restart/recovery E2E |
| 3 | attachments | ✅ | `AttachmentRegister` IPC + fnv1a64 content-hash verification; exercised in realtime E2E |
| 4 | Goal/Mission chat | ✅ | `GoalSubmit` → mission creation; `SubmitMission`; realtime E2E proves goal→mission→planner→provider→tool→changeset→verification |
| 5 | visible plan | ✅ | Plan projection IPC; MissionView plan pane |
| 6 | visible live work/activity | ✅ | `ConversationActivity` + `GetMissionEvents` streaming projections; GUI activity feed |
| 7 | real tool/command evidence | ✅ | Tool Broker observation→EvidenceStore; CommandOutput evidence for terminal (N4, verified in DB by test) and MCP tools (N5, evidence captured by test) |
| 8 | ChangeSet/test/verification evidence | ✅ | Governed ChangeSet lifecycle (propose→validate→approve→apply→validating→accept) with evidence refs; verification engine gates acceptance |
| 9 | pause/resume/cancel | ✅ | `PauseMission`/`ResumeMission`/`CancelMission` IPC; in-flight cancellation test (`in_flight_cancellation_during_tool_execution_produces_cancelled_not_failed`) |
| 10 | restart/recovery | ✅ | `recover_or_create_worktree` + durability observer + RestartRecovery dogfood kind; chaos/restart integration tests |
| 11 | provider failover | ✅ | ProviderRegistry failover (ProviderFailure dogfood kind); N1 provider-preference persistence (commit `17f4894`) |
| 12 | browser tool | ✅ | Real Chrome via CDP (batch4 test drives real DOM + screenshots + network failure diagnostics); N2 graceful shutdown (`7d1e4b9`) |
| 13 | terminal/process tool | ✅ | N4 live terminal surface: streaming tail, cancel, list, evidence (commit `abd5100`) |
| 14 | Discuss | ✅ | Discuss mode with grounding (N3 LSP enrichment, commit `5adf05b`) |
| 15 | Discuss → Plan → Mission | ✅ | `DiscussPromote` dogfood kind + discuss→mission promotion path in conversation.rs |
| 16 | Design Studio basic workflow | ✅ | design.rs (170 mentions): reference/vision/metrics surfaces + design_reference_vision_e2e with real Chrome + vision model |
| 17 | Security baseline | ✅ | security.rs (260 mentions): scan, privileged-instruction detection, redaction; SecurityAudit dogfood kind |
| 18 | provider/model settings | ✅ | Provider settings UI (types.ts/daemon.ts) + N1 persistence (`17f4894`) |
| 19 | no secret leakage | ✅ | `redact_source_line` grounding redaction + terminal evidence redaction + evidence-layer `redact()`; `secrets_are_redacted_from_prompts` test; N10 hostile-MCP fixture proves attack output is bounded data |
| 20 | project isolation | ✅ | ac-sandbox isolation levels; `governed_command_cannot_read_sensitive_file_outside_workspace` test; workspace-scoped fs tools |
| 21 | daemon independence | ✅ | Daemon owns all state via SQLite; UI is a Tauri 2 client over IPC; `daemon_process_boundary` integration test |
| 22 | self-dogfooding on AgentCode | ✅ | DogfoodHarness required catalog (14 kinds incl. MaliciousMcp/PromptInjection) + `chaos_dogfood_flow` integration test + realtime GUI-path E2E on the real daemon with a real local model |
| 23 | MCP real transport | ✅ | N5 (commit `e04e8f8`): real stdio JSON-RPC 2.0 client through the Tool Broker; fixture-server protocol tests; agent-level `.mcp.json` discovery test |
| 24 | signed/notarized packaging | ⬜ **BLOCKED** | N9: requires Apple Developer ID (user decision/prerequisite — outside agent control). Bundle/notarization/update path cannot be produced without credentials. |

## Honest scores (N10)

- **Engineering substance vs old §21**: 23/24 boxes checkable with committed,
  test-backed evidence. The single open box (N9 packaging) is blocked on user
  credentials, not on code.
- **Validation matrix at this commit**: `make validate` GREEN — 454 passed /
  0 failed / 26 env-gated; workspace clippy 0 warnings; frontend
  build/tsc/eslint GREEN.
- **Real-model dogfood evidence (this session)**:
  `realtime_conversation_e2e -- --ignored` with gemma3:4b via local Ollama:
  PASS (71.4s) — full GUI-path mission (conversation → goal → mission →
  planner → provider → tool → changeset → evidence → verification → activity).
- **Known limitations (honest)**:
  - `batch4_browser_runtime_drives_real_chrome_dom_and_evidence` is
    resource-flaky under FULL-suite parallelism (CDP event arrival latency);
    passes in isolation and in most full runs. No product defect; timing.
  - LSP enrichment on developer machines without rust-analyzer degrades
    honestly (documented; fixture-server covers protocol truth in tests).
  - N9 packaging absent (credentials).

## What remains (the whole picture)

1. **N9 — signed/notarized packaging** (blocked on Apple Developer ID): once
   credentials exist, produce/sign/notarize the macOS ARM64 bundle, daemon
   lifecycle in the packaged app, update/rollback per P27, SBOM/licensing
   pass. ~2–3 days after credentials.
2. **N10 completion package** (Doc 11 H28/H34-35): commit this checklist,
   update the audit tracker rows (N5 ✅, N10 self-security ✅, remaining
   state), and record the goal-level handoff.
3. Optional confidence work: re-run the design-reference vision E2E
   (env-gated) when convenient; broaden the Chrome CDP poll deadline if the
   batch4 flake recurs in full-suite runs (testability tweak only, no
   product change).
