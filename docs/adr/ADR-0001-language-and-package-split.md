# ADR-0001 — Language and Package Split

- **Status:** ACCEPTED
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

Which implementation languages/package ecosystems compose AgentCode V1, and where are
the boundaries?

## Constraints

- Doc 07 §102–103: "Recommended Core Boundary" — DESKTOP UI Tauri + React/TypeScript;
  AUTONOMY KERNEL Rust or another reliable native service; PROVIDER FABRIC custom
  OmniRoute fork / TypeScript-compatible layer; specialized security tools external
  binaries; optional Python security/AI tools managed adapters.
- Doc 07 §106: Kernel, Model Broker, OmniRoute interface, Code Intelligence, Context
  Engine, Tool Broker, Edit Engine, Git/worktrees, Verification, Skill loader, Hook
  engine, Desktop UI are core.
- Doc 03: Kernel is the single authoritative writer; SQLite V1 control store.
- Doc 09 H12: build/repository boundary direction; architecture linting must prevent
  UI→DB, provider→Kernel-state, security→scheduler inversions.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| All-TypeScript | Simplest for UI + OmniRoute reuse | Conflicts with "Kernel as reliable native service"; SQLite/process control weaker; Doc 02/03 backend = Rust direction |
| All-Rust | Kernel + desktop in Rust | Conflicts with "custom OmniRoute fork / TypeScript-compatible layer" and Tauri + React/TypeScript preferred direction (Doc 06 §8030) |
| Hybrid (chosen) | Rust core + TS provider fabric + Tauri/React desktop + external binaries | Matches Doc 07 §103 exactly |

## Evidence

- Doc 07 §102–103, §104 (Local RPC boundary).
- Doc 06 §8030: "Tauri 2 + React/TypeScript/Vite is the preferred V1 desktop direction".
- Doc 02 §115 example: backend language = Rust (LONG_LIVED knowledge class).
- Extraction: P01-WP04 (runtime) — donor runtimes (codex Rust core, opencode TS loop)
  confirm per-language strengths; P01-WP03 (provider fabric) confirms TS provider
  ecosystem.

## Decision

- **Kernel and core services** (kernel, runtime, tools, edit, git, verification,
  security policy, code intelligence core, context core, model broker core): **Rust**
  (Cargo workspace, crates under `crates/`).
- **Provider fabric / OmniRoute-compatible layer**: **TypeScript** (Node, pnpm
  workspace, under `services/provider_gateway`).
- **Desktop shell**: **Tauri 2 + React/TypeScript/Vite** (under `apps/desktop`).
- **External binaries** (scanners, ctags, etc.): invoked per ADR-0008, never a
  language dependency of core.
- No Python in the V1 core path; optional Python adapters are out-of-process tools per
  ADR-0008.

## Consequences

- Positive: each layer uses its strongest ecosystem; OmniRoute TS reuse is direct;
  Kernel is a native service with real process/SQLite control.
- Negative: two package ecosystems to maintain; IPC boundary required (ADR-0005).
- Affected interfaces/documents: Doc 07 §102–103, Doc 09 H12, all Phase 2+ work.
- Rollback: none needed; this is the baseline decision for the workspace.

## Verification

- Phase 2 build commands build both ecosystems from clean checkout (P2-G1).
- Architecture-boundary linting enforces cross-language ownership rules (P02-WP01).