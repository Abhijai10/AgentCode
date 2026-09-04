# AgentCode — Design Studio / Discuss Mode / Security Mode Detailed Audit

**Scope:** Full code-cycle audit of the three product modes against Docs 05/06/08 (PRD), Roadmap Doc 09, AGENTS.md §6, and the doc-contract implementation level actually present in the tree at HEAD `531a1b1`.
**Method:** Read the doc contracts first, then traced every production path: Desktop UI (React) → Tauri command (`apps/desktop/src-tauri/src/main.rs`) → daemon IPC (`crates/ac-daemon/src/ipc.rs` dispatch) → mode logic (`conversation.rs` / `design.rs` / `security.rs`) → SQLite persistence (`ac-db`) → tests. Cross-checked against the richer P20/P21 service layer in `crates/ac-agent` (discuss.rs / design.rs).
**Constraint honored:** Audit only — nothing implemented, no test edits, only this report file added.

---

## Executive Summary

All three modes **exist, persist, and survive restart** — the conversation/chat/product-activity cycle is real and well-tested (51 daemon IPC tests + 3 security E2Es + vision E2E, all green). The skeleton is honest: every button in the UI is wired to a real daemon command, every command persists real state, read-only policies are enforced, and there is no fake success anywhere.

But **none of the three modes is at the level the core docs actually specify**. The dominant pattern is the same in all three: the *persistence and lifecycle plumbing* was built to doc-grade, while the *product intelligence layer above it* was built thin — and, critically, the **fuller implementations that the docs describe already exist in `crates/ac-agent` (`DiscussMode`, `DesignStudio`) but are orphaned from the production path** (they are never called by the daemon; this is exactly the "uncalled implementation" AGENTS.md §6 forbids counting as complete).

| Mode | Doc-grade skeleton | Product-grade intelligence | Verdict |
|---|---|---|---|
| Discuss Mode | ✅ Complete | ❌ Surface-level (file NAMES, not contents) | **PARTIAL — will not work as intended** |
| Design Studio | ✅ Complete | ⚠️ Half (workflow real, QA heuristics disconnected from real browser output; mission promotion loses all design context) | **PARTIAL — core loop works, quality layer is scaffolding** |
| Security Mode | ✅ Complete | ✅ Mostly real (real scanners, real triage, real lifecycle) | **CLOSEST TO INTENT — needs depth polish, not redesign** |

The single most valuable fix (below) is **wiring the daemon's mode endpoints to the existing ac-agent services** — most of the "remaining" intelligence is already written, tested, and green in `ac-agent` tests, just never invoked in production.

---

## Part 1 — Discuss Mode Audit

### What the docs require (Doc 06 §20-25, PRD-DISC-001..007, scope matrix `REQUIRED_V1` P1)

Repository-aware engineering discussion: the model must answer "How does authentication currently work?" **grounded in the actual project** — via repository reading, deep search, architecture reasoning, code explanation. Read-only default. Model-broker routing with local fallback. **Turn Into Plan** (structured extraction: accepted design, constraints, rejected approaches, requirements, open questions). **Execute Plan → mission** without copy/paste. Accepted decisions persist into `DECISIONS.md` / `CONTEXT.md` / structured Kernel state.

### What is implemented (verified in code)

| Requirement | Status | Evidence |
|---|---|---|
| Durable conversations, messages, attachments, restart persistence | **VERIFIED_COMPLETE** | `conversation.rs` + 12 green IPC tests (`discuss_*`, three_chats, restart, project isolation) |
| Read-only default | **VERIFIED_COMPLETE** | Prompt-level + `DiscussMode::evaluate_read_only` enforces Deny on write capabilities |
| Model routing + local fallback | **VERIFIED_COMPLETE** | `daemon_provider_registry`, `RoutingProfile::LocalFirst`, ProviderModelRecord persisted per send (conversation.rs:444-465) |
| Mission promotion without copy/paste | **VERIFIED_COMPLETE** | `goal_submit` links conversation↔mission, mission_ref on message, `set_conversation_mission` |
| Attachments feed context (bounded, security-pipeline) | **VERIFIED_COMPLETE** | `load_goal_attachments` 4KB bound; oversize/cross-project tests green |
| **PRD-DISC-001: Repository-grounded answers** | **PARTIAL — the core gap** | `discuss_send` builds context from `bounded_project_listing` (top-level dir entries: **names + sizes only, no file contents**) + last-20 messages + project path hint (conversation.rs:386-400, 585-607) |
| **§23 Turn Into Plan (structured extraction)** | **MISSING in production** | UI has "Create Mission" which submits raw text; no plan document is created |
| **§25 Decisions → DECISIONS.md/CONTEXT.md** | **MISSING in production** | No decision persistence anywhere in daemon path |
| Web research fallback (§20) | **MISSING in production** | `DiscussMode::integrate_research` exists in ac-agent, uncalled |

### The critical orphaned implementation

`crates/ac-agent/src/discuss.rs` contains the **complete P20 service**: `start_session`, `record_message`, `answer_repository_question(&mut CodeIntelligenceService, …)` (real repo-context Q&A with source citations), `evaluate_read_only`, `integrate_research`, `decision_candidate` → `accept_decision` (persists to MemoryService), `promote_to_plan` (structured PlanDoc), `promote_to_mission` (Kernel mission, `transcript_replay_required=false`). It is tested green (`discuss_answers_from_repo_context_and_promotes_without_transcript_replay`, lib.rs:5350).

**But `grep -rln "DiscussMode" crates/` returns only ac-agent itself.** The daemon's `DiscussSend` never calls it. The production path answers "how does auth work?" with a model that sees a directory listing, not code. This fails PRD-DISC-001 as-written ("must be grounded in the actual project" — a filename list cannot explain how authentication works).

### Verdict

**PARTIAL — will NOT work exactly as intended.** The chat experience, durability, routing, and mission promotion are genuinely doc-grade. The defining capability — repository grounding — is surface-level, and the plan/decision layers the docs mandate are absent from the production path despite existing (orphaned) in ac-agent.

---

## Part 2 — Design Studio Audit

### What the docs require (Doc 06 §26-63, PRD-DES-001..016, H16)

Full pipeline: product understanding → Design Brief → Design Grammar → **real implementation** → live preview → browser inspection → **visual critic** → anti-slop review → responsive/accessibility/functional QA → repair loop → functional verification. Mission types DESIGN_ONLY / DESIGN_AND_IMPLEMENT / VISUAL_REPAIR / DESIGN_SYSTEM_EVOLUTION. Constraints (H17) durable and outranking critic. Reference images → principles not copies. DESIGN_STATE.md readable snapshot. Design memory across missions. Design skills loaded on demand. QA must use **deterministic browser checks + visual-model QA** (PRD-DES-011).

### What is implemented (verified in code cycle)

Real and doc-grade:
- **Product understanding** (`design_understand`): framework detection, routes/components/styles/tokens/assets/navigation scan (bounded 200 entries) → persisted `product_analysis` doc. ✅ (shallower than Doc 06 §30's "user workflow, brand, density" but functional)
- **Brief + Grammar + DESIGN_STATE**: persisted, versioned, restart-surviving docs (`design_brief`, `design_grammar`, `design_state`) with real E2E roundtrip test. ✅ as *storage*; content is template-derived (grammar is hardcoded strings with the product name interpolated — §32 "product-specific grammar" is aspirational)
- **Live preview lifecycle** (`design_preview_start/status/stop`): real dev-server spawn, stdout port detection, HTTP probe, honest `PREVIEW_COMMAND_MISSING`. ✅
- **Browser inspection** (`design_browser`): real Chrome CDP (session, navigate, DOM text/controls/a11y tree, console/network diagnostics, screenshot + evidence ref, viewport profiles). ✅ — this is real and modern (Chrome 151 flow hardened this session)
- **Reference-image analysis** (`design_analyze_reference`): REAL gemma3:4b vision, strict-JSON H28 extraction, copying boundary, persisted `reference_analysis`, honest failure classes. ✅ **This is the strongest implementation in the whole design layer** — E2E-proven with a real model.
- **Anti-slop critic** (`design_critique`): 5 real heuristics (gradient hero, card spam, AI copy, glass, placeholders), persisted with improvement_required flag; repair suggestions per rule. ✅ as a v1 heuristic layer
- Design conversation durability, project isolation, three-chat switching, restart. ✅
- **Implement via Mission** → real Kernel mission. ✅ mechanically

The gaps (each verified in code, not speculation):

1. **QA layer is disconnected from the real browser output.** `design_qa_responsive/accessibility/functional` are pure substring checks over a caller-supplied string. The UI feeds them `browser.visible_text` (DesignView.tsx:336-338) — **rendered page text can never contain `position: fixed` or `<button`**, so responsive QA on a real preview structurally always passes/vacuously fails. Doc 06 §49 (deterministic viewport/overflow/presence checks via the browser), §54 (real responsive checks), §55-56 (a11y via accessibility tree), PRD-DES-011/012/013 are **not met** in the live loop. The deterministic DOM test passes only because the harness feeds HTML markup directly.
2. **Visual critic on screenshots is absent.** §50-51 (local Gemma3 4B visual QA; critic ≠ implementer model) — exists ONLY as the reference-analysis path; the preview screenshot is never sent to a vision model for critique. `run_preview_iteration` in ac-agent (design.rs:487) implements this loop; uncalled.
3. **"Implement via Mission" strips all design context.** It submits the raw user text as the mission goal (DesignView.tsx:666). The Brief, Grammar, DESIGN_STATE, reference principles, and constraints the user curated are **not injected into the mission goal/context**. PRD-H16's "implement real code" informed by the design system and §46-47 component reuse are therefore not delivered — the mission implements the *user's sentence*, not the *design contract*. H16 mission types (DESIGN_ONLY etc.) do not exist anywhere in code.
4. **Design memory across missions (§91, PRD-DES-015) is missing.** DESIGN_STATE persists per-conversation (design_documents table keyed by conversation_id); a new DESIGN conversation starts blank. `generate_design_state` in ac-agent supports the cross-session pattern; uncalled.
5. **Multiple directions (§43) and variations**: absent (Doc 06 marks OPTIONAL_V1 — legitimately out of scope).
6. **Design skills (§95) / design context pack (§96)**: no design-mode skill loading or DesignContextPack build; the daemon prompt includes only the DESIGN_STATE block. P16 skill infra exists generally; not applied here.
7. **Click-to-source (§87-88) / Onlook**: `map_dom_to_source` exists in ac-agent (design.rs:594); uncalled. Marked OPTIONAL_V1/advanced — legitimately post-scope.
8. **Constraint persistence (H17)**: no durable constraint record; the critic cannot respect "preserve navigation, keep typography" because constraints never leave the chat text.

### Verdict

**PARTIAL — the core loop (understand → brief → grammar → preview → inspect → critique → repair → mission) is genuinely real and persistent, but the quality layer is scaffolding**: QA can't actually see the page it claims to check, the visual critic never sees the screenshot, and the mission that implements the design is blind to the design. Of the "Required V1 core" list in H16, the first six items are real; the last five (responsive QA, accessibility QA, functional preservation, meaningful Design State, functional verification) are placeholders in the live path.

---

## Part 3 — Security Mode Audit

### What the docs require (Doc 06 §79-83, PRD-SEC-001..018, Doc 05 §92-98)

Native five-option entry (Quick Audit / Full / Cloud / AI Security / Adversarial Validation); repo-derived threat context before scanning; SAST/secret/dependency/IaC scanning with redaction; attack-path reasoning; safe authorized active validation; controlled finding lifecycle (never auto-CONFIRMED); false-positive reduction; remediation → normal Kernel repair missions; regression retest; md/json/sarif reports; production safety.

### What is implemented (verified — strongest of the three modes)

- **Scope confirmation + authorization gates**: SecurityModeScope with target/scope_kind/auth_state/allowed hosts/ports/techniques; `security_set_scope` persists; UI shows the exact Doc 06 §80 confirmation pattern. ✅
- **Threat model first**: baseline orchestrator derives entry points/assets from real file content before findings; threat_model persisted into session. ✅ (PRD-SEC-002)
- **Real external scanners** (closed this session, `0b09640`): gitleaks/semgrep/osv/trivy/checkov run through the same governed Tool-Broker executor as `security.verify`, per-scanner honest unavailability, merged via `merge_reports`, coverage lists persisted and surfaced in status. **Real E2E proof**: installed gitleaks detects the seeded credential. ✅ (PRD-SEC-003/004/005/006 — semgrep rules path + osv/trivy DBs remain env-dependent, honestly reported)
- **Redaction**: secret values never persisted in fingerprints; `provider_account_ipc_responses_never_expose_raw_secret` test; evidence raw-content guard tests. ✅ (PRD-SEC-004)
- **AI security**: applicability detection before scanning; injection-attempt detection recorded as UNTRUSTED DATA findings (real self-security gate). ✅ (PRD-SEC-013)
- **False-positive reduction / lifecycle**: findings NEVER enter CONFIRMED from scanner output (New → … gates); test/fixture-only findings routed to NeedsManualReview by reachability; state machine enforced (`walk_finding_states`, `can_auto_repair`). ✅ (PRD-SEC-014/015)
- **Remediation → Kernel mission**: `security_remediate` builds a goal from the finding, requires CONFIRMED state + explicit approval, creates real mission, links finding→mission, transitions to Fixed. ✅ (PRD-SEC-016, §83)
- **Retest/regression**: `security_retest` rescans, closes clean FIXED findings, reopens regressions to CONFIRMED. ✅ (PRD-SEC-017)
- **Reports**: security-report.md/json/sarif with canonical 7-state FinalSecurityStatus including SCANNER_COVERAGE_INCOMPLETE honesty state. ✅ (PRD-SEC-018)
- **Restart persistence + project isolation**: finding-by-fingerprint stable identity across restarts; cross-project access policy-denied. ✅
- **Attack paths** (§82): persisted graph (entry_point → finding chain) and surfaced in UI. ✅ as functional visualization
- **Production safety** (PRD-SEC-012): production scopes block active actions with recorded stop_reasons; active testing requires explicit authorization state. ✅

Remaining gaps (minor vs the other two modes):

1. **The five entry options are UI-visual only.** Doc 06 §79 / PRD-SEC-001 wants Quick vs Full vs Cloud vs AI vs Adversarial audits to translate into **different scan policies**. The UI offers one "Run Audit" flow (scope form captures adversarial/production nuances); `security_audit` always runs the same baseline+managed+AI-probe composition. A "Quick Audit" and a "Full Security Audit" produce identical work. Audit-depth selection exists nowhere in the daemon contract.
2. **Cloud audit (PRD-SEC-008) depth**: Prowler-compatible posture analysis exists in ac-security (P18) but is not part of `security_audit`'s sweep; Cloud Lab scope_kind exists but no cloud adapter runs in the Security Mode path (needs explicit cloud scope + credentials — legitimately env-dependent, but the *wiring* is absent).
3. **Dependency-scan data**: osv/trivy adapters are honestly Unavailable without prepared DBs (`scanner_data_unavailable`) — correct honesty, but dependency security in Security Mode therefore depends on host preparation. `dependency_manifest` is hardcoded `None` in `scan_input` (security.rs:346) — the audit never passes a real lockfile to the baseline orchestrator even when one exists.
4. **Web DAST (PRD-SEC-007)**: ZAP adapter + `authorize_browser_target` are real, but Security Mode has no flow that launches a dev server and runs the DAST against it (the design preview's dev server would be the natural target). scope_kind=adversarial stores intent; no adapter executes within `security_audit`.
5. **SARIF report**: generated (report.sarif) but not exported to a user-visible path in the UI — the report panel shows md/json summaries; SARIF is for CI, acceptable.
6. **Finding detail panel** (§81: problem/evidence/attack path/fix/verification): UI expands findings with severity/state/category and remediation text; evidence references are IDs, not clickable/evidence-linked content. Functional but shallower than the doc's five-section expansion.

### Verdict

**CLOSEST TO INTENT — the lifecycle is genuinely doc-grade and honestly evidenced.** The remaining work is depth, not architecture: audit-depth selection (Quick/Full/Cloud/AI/Adversarial policies), real dependency-manifest passing, optional DAST-against-preview, and richer finding-detail evidence display. Nothing here is fake; several doc requirements are met only via honest-unavailable states (osv/trivy DBs, semgrep rules) which is the *correct* behavior but still means "Full Security Audit" ≠ what a user with a prepared environment would get elsewhere.

---

## Part 4 — Cross-cutting findings (all three modes)

1. **The orphaned ac-agent service layer is the project's biggest structural finding.** `DiscussMode` and `DesignStudio` (P20/P21 WP deliverables, tested green, including `answer_repository_question`, `promote_to_plan`, `accept_decision`, `run_preview_iteration`, `generate_design_state`, `map_dom_to_source`) have **zero production callers**. AGENTS.md §6: "Every accepted WP must answer: Who calls it?" — currently: nobody. Either wire them in (my recommendation, below) or explicitly re-scope them as library-level capability records. The phase records P20/P21 marked COMPLETE on the strength of these implementations while the daemon shipped a parallel thin path — the doc-gates passed on the wrong layer.
2. **`bounded_project_listing` is the single weakest line of code for doc-conformance.** All three modes share it (discuss/design/security chat prompts). It gives the model ~80 top-level names. Doc 06 §20-21 (repository-aware discussion), §30 (product understanding), and the security chat all depend on real file contents that never arrive. ac-context's ContextPackBuilder and ac-code-intel's ContextBuilder exist, are tested, and are not used here.
3. **Mission promotion loses context in ALL modes.** Discuss "Create Mission", Design "Implement via Mission", Security remediation all construct goal text from user/finding text only. The docs demand structured context (plan docs, design docs, remediation scope+constraints) flow into the mission. Security's goal construction is the best of the three (includes remediation text + affected files); Design's is the worst (raw sentence).
4. **Mode entry UX (Doc 06 §84)**: sidebar switches conversations by mode; creating a DESIGN conversation from the project happens via a mode-specific "New Chat". Functional, matches the doc's intent.
5. **Testing honesty is excellent throughout**: 51 IPC tests including restart/persistence/isolation per mode; real-scanner and real-vision E2Es are genuinely real. The weak spots (QA heuristics on visible_text) are honestly green only because the deterministic harness feeds them markup — i.e., tests pass on the inputs production never produces. This is the exact "tests that test only mocks" smell AGENTS.md §6 prohibits counting toward completeness.
6. **Persistence schema is doc-grade**: conversations/messages/attachments per mode, design_documents/versioned, security_mode_sessions/findings with lifecycle states, design_previews, attack paths, all restart-stable, all project-isolated.

---

## Part 5 — Concept-closeness & Rating

### How close is AgentCode to the concept?

The concept (Doc 06 §1-2): a calm, professional, local-first autonomous coding product where complexity is coordinated invisibly — mission runtime, model routing, worktrees, verification, security — behind OPEN PROJECT → DESCRIBE GOAL → START → LEAVE → REVIEW.

**Where the concept is genuinely realized (the hard parts!):**
- The **autonomous mission runtime is real and proven**: real daemon binary, real SQLite kernel state, worktree isolation, crash/disconnect recovery without replay, evidence-backed completion gates — the infrastructure that 90% of "AI app builders" fake, is honest here, at production quality.
- **Provider fabric with local-first routing, failover, cost policy** — real, tested, vision-capable.
- **Governed tool execution with honest sandbox evidence** — real, including honest degradation.
- **The UX shell matches the doc's mental model** — project list → goal composer → mission view with activity/changesets → calm panels. No office metaphor, progressive disclosure, honest failure states.
- **Security lifecycle honesty** — never-auto-confirm, reachability triage, honest scanner coverage — is philosophically exactly the doc's intent.

**Where the concept is not yet realized:**
- The three specialist modes promise product-defining intelligence (repo-grounded reasoning, design-quality loops, depth-varied security) and currently deliver **chat with thin context on durable rails**.
- "Repository-aware" currently means "filename-aware". The docs' core promise — the agent *knows your codebase* — lives in ac-code-intel/ac-context, is built and tested, and is **switched off** on every path a user actually touches.
- The design loop's promise "browser feedback → visual critique → repair" is real up to the screenshot, then goes dark: the screenshot is captured (with evidence!) and never looked at by the critic.
- Mode → mission promotion is where the product concept (modes as on-ramps to autonomy) currently leaks the most value.

### Project rating: **7/10 — strong foundation, incomplete product surface**

| Dimension | Score | Rationale |
|---|---|---|
| Architecture & honesty | 9.5 | Doc-locked, evidence-first, no fake success, honest degradation everywhere; exemplary discipline |
| Core autonomous runtime | 9 | Real kernel/worktree/recovery/routing, proven by real-model E2Es |
| Test & evidence culture | 9 | Restart/isolation/redaction/security-evidence tests are unusually thorough; the only smell is QA-on-markup |
| Discuss Mode vs docs | 4.5 | Durable chat yes; repo-grounding, plan promotion, decisions — missing from production path |
| Design Studio vs docs | 5.5 | Real pipeline skeleton + reference analysis; QA/visual-critic/mission-context are scaffolding |
| Security Mode vs docs | 8 | Lifecycle doc-grade; depth selection + dependency/DAST wiring remain |
| Desktop UX vs docs | 7 | Faithful shell, all panels wired; preview embed, findings detail, command palette remain |
| **Overall vs concept** | **7** | The invisible-complexity promise is delivered on the mission spine; the three modes are at ~50-60% of their doc contracts |

**One-line verdict:** The engine is a 9; the three mode bodies around it are a 5 — and unusually, most of what's missing is *already built one layer down* and simply not connected.

---

## Part 6 — Implementation Plan (ordered by value, dependency-aware)

### Phase A — Connect the existing intelligence (highest value, lowest risk; ~1-2 weeks)

The ac-agent services are written and tested; wiring them into the daemon path is mostly plumbing plus honest integration tests.

**A1. Discuss Mode: route `discuss_send` through `DiscussMode::answer_repository_question`.**
- Build `DiscussRepositoryContext` in the daemon: use `ac-code-intel`'s ContextBuilder (deterministic fallback: read up to N file contents ≤ size budget for user-referenced paths + top-K relevant files by recency/relevance from the structural index) + `MemoryService` for session memory.
- Keep the current prompt assembly as the no-index fallback (degraded but honest).
- Persist `sources` (the answer's citations) into message metadata; render them in DiscussView (PRD-DISC-001 done for real).
- Test: extend `discuss_send_creates_discuss_conversation_and_appends_messages` — assert the model prompt included file CONTENT for a known fixture file; E2E: ask "what does src/auth.rs contain?" → answer references the fixture's function name.

**A2. Discuss Mode: ship the missing §23/§25 features via the existing service.**
- New IPC command `DiscussPlanExtract` → `DiscussMode::promote_to_plan` (needs accepted decisions; allow explicit user-supplied plan bullets when no decisions recorded).
- Persist the plan as a `plan_document` (reuse design_documents table with doc_type="discuss_plan" or a conversation_doc table) + render a plan panel with [Execute Plan] → `promote_to_mission`.
- Decision capture: lightweight "Accept as decision" action on any assistant message → `decision_candidate`/`accept_decision` → MemoryService + append to `DECISIONS.md` in the project (Doc 06 §25).
- Tests: plan extraction roundtrip; decision persists across restart; DECISIONS.md contains the decision text.

**A3. Design QA: feed the QA functions real inputs and wire the visual critic.**
- `design_browser` already captures DOM + a11y tree + diagnostics + screenshot. Change QA to consume THAT:
  - responsive: run `design_browser` at the 3 viewport profiles and diff real metrics (horizontal overflow via DOM scrollWidth>clientWidth, nav collapse, touch-target sizes from the a11y tree) — deterministic checks Doc 06 §49/54 exactly;
  - accessibility: run real checks over `dom.controls` + accessibility_tree (labels, roles, contrast needs color info — start with labels/roles/keyboard-focusability; record contrast as NeedsManualReview without rendered styles);
  - functional: `diagnostics.console_errors`/`page_errors`/`network_failures` + presence of key controls — real signal the browser already returns.
- Visual critic: pass `screenshot_uri` to `request_model_with_image` (gemma3:4b; AGENTCODE_VISION_MODEL) with a critique prompt (Doc 06 §50) — distinct profile from the implementer (§51 independence); critique findings merge into the existing design_critique table with a `visual_critic` doc_type. Honest unavailability when no vision model (reuse the H28 failure classes).
- Tests: deterministic DOM fixture with real overflow/labels defects → QA finds them; vision-gated visual-critic E2E mirroring design_reference_vision_e2e.

**A4. Design "Implement via Mission": inject the design contract.**
- Build the mission goal as structured text: goal + Design Brief summary + Grammar principles + explicit_do_not_copy/adopted principles from reference analysis + H17 constraints (new lightweight constraint list captured in chat) + target files from product_analysis.
- Persist the linkage (mission_ref on a design mission message; DESIGN_STATE updated with mission id).
- This requires no new runtime capability — the mission goal/context is plain text; but curate it under a token budget using ac-context if the docs get large.
- Test: mission goal text contains brief/grammar content for a fixture conversation (assert via GetMission).

**A5. Delete or wire remaining orphaned pieces.**
- `DiscussMode::integrate_research` → optional research fallback in discuss (needs network policy decision).
- `DesignStudio::run_preview_iteration` + `generate_design_state` (cross-conversation memory §91) → A3/A4 sequence; `map_dom_to_source` stays post-V1 (explicit ADR note).
- Add a CI guard: `grep -rln "DiscussMode\|DesignStudio" crates/` outside ac-agent tests fails — prevents future orphan drift. (AGENTS.md §6 enforcement made mechanical.)

### Phase B — Security depth (mostly small, self-contained; ~1 week)

**B1. Audit-depth selection.** Add `audit_depth` param to `security_set_scope`/`security_audit`: `quick` (baseline only), `full` (baseline+managed+AI), `cloud` (full + Prowler-compatible posture when cloud scope/credentials declared — fail honestly otherwise), `ai` (AI-probe emphasis), `adversarial` (managed + authorized active adapters incl. ZAP against the design preview URL when the user authorizes it). UI: the five §79 buttons. This is the doc-required "translate options into Doc 05 policies" (PRD-SEC-001) and it's mostly policy-branching over existing orchestrator capabilities.
**B2. Pass a real dependency manifest.** Detect Cargo.toml/pnpm-lock/package.json in the project and pass `dependency_manifest: Some(...)` (security.rs:346) so baseline dependency analysis has input even when osv/trivy DBs are unprepared.
**B3. Finding-detail evidence links.** Surface evidence content (redacted) + regression refs in the detail panel; make evidence IDs resolvable via the existing observability evidence IPC.
**B4. DAST-against-preview option** (only if time allows): when a design preview is running + user authorizes, run the ZAP adapter against 127.0.0.1:port in `adversarial` depth.

### Phase C — Polishing to doc-level UX (parallel, ~1 week)

- C1. Live preview embed in DesignView (iframe/webview for ready_url + Open Fullscreen, Doc 06 §85-86) instead of URL text.
- C2. Screenshot thumbnail rendering from screenshot_uri in the browser panel.
- C3. Command palette + project search (Doc 06 §126-127) — currently absent; small React work.
- C4. Plan panel + decision affordance in DiscussView (from A2).
- C5. Findings detail evidence expansion (from B3).

### Sequencing & effort

```
Week 1: A1+A2 (discuss intelligence + plan/decisions)  — unblocks the biggest doc gap
Week 2: A3+A4 (design QA realism + mission context)    — makes the design loop real
Week 3: B1+B2 (+B3)                                   — security depth to doc level
Week 4: C-series + A5 cleanup + CI orphan guard       — UX polish + drift prevention
```

Each phase lands as focused commits with the same E2E discipline used this session (real fixtures, restart/persistence assertions, honest-unavailable paths kept honest). After Phase A+B, realistic re-grade: Discuss 4.5→8, Design 5.5→8, Security 8→9, overall 7→8.5.

### Explicitly out of scope / legitimately deferred

- Multiple design directions (§43) — OPTIONAL_V1 per Doc 06.
- Click-to-source direct manipulation (§87-88) — OPTIONAL_V1.
- Onlook/Dyad-style visual editing — reference-only in docs.
- Windows packaging, cloud-credentialed Prowler execution — environment-dependent; keep honest-unavailable.
- Skills-per-mode progressive loading (§95) — worth doing after A-series via the P16 registry.
