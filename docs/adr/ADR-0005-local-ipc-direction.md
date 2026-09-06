# ADR-0005 — Local IPC Direction

- **Status:** ACCEPTED (direction) with bounded-PENDING transport sub-item
- **Decision class:** CONSTRAINED_IMPLEMENTATION
- **Date:** 2026-08-20
- **Supersedes:** none
- **Superseded by:** none

## Problem

How do the desktop shell and any local client communicate with the daemon, and who may
own what state?

## Constraints

- Doc 07 §104 (Local RPC boundary): Unix socket, localhost HTTP, WebSocket, or Tauri
  IPC; "Do not tightly couple renderer state to Kernel internals."
- Doc 03: Kernel is the single authoritative writer of mission/task/completion truth;
  desktop UI must never mutate the mission DB directly.
- Doc 11 H17: desktop UI, provider gateway, scanner adapter, Worker must not write
  mission state directly.
- Doc 06 §4146: renderer restart, Tauri window recreation, laptop sleep, daemon
  restart, or temporary IPC loss must not leave the interface half-true.

## Considered Alternatives

| Alternative | Description | Why rejected |
|-------------|-------------|--------------|
| Tauri IPC (commands) for everything | Renderer→Rust commands | Couples renderer to the Rust shell; daemon is a separate process — Tauri IPC is process-local |
| Direct SQLite access from UI | Simplest read path | Forbidden by Doc 03/H17 |
| Local HTTP over Unix domain socket from daemon (chosen direction) | Daemon exposes a versioned JSON API; clients are HTTP clients | Matches Doc 07 §104 options; process boundary preserved; testable with curl |

## Decision (direction — ACCEPTED)

- The **daemon owns the control-plane API**. All mission/task state mutation flows
  through the Kernel in the daemon process.
- Desktop and other local clients speak a versioned JSON API to the daemon; they never
  open the control-plane SQLite database.
- Renderer state is derived/presentation state only (Doc 06 §4239 four-class state
  separation applies in Phase 6+).

## Bounded PENDING sub-item — exact transport

- **Open question:** Unix-domain-socket HTTP vs localhost TCP HTTP vs WebSocket for V1.
- **Evidence that resolves it:** Phase 3 "daemon IPC" prototype (Doc 11 §30 prototype
  queue item) plus macOS sandbox/notarization constraints for packaged apps (Doc 09
  H12, Doc 06 §138), and desktop connection-loss semantics (Doc 06 §4146).
- **Deadline:** must be ACCEPTED before Phase 3 daemon IPC work; the skeleton in
  Phase 2 keeps the API surface transport-agnostic (no socket code is written in
  Phase 2).

## Consequences

- Positive: ownership is unambiguous; UI cannot corrupt mission truth.
- Negative: every UI read goes through the API (extra hop); acceptable.
- Affected: Doc 03, Doc 07 §104, Doc 06; Phase 3 daemon work.

## Verification

- Phase 2: no IPC code exists yet (skeleton only); architecture lint rejects UI→DB
  imports; Phase 3 gates P3-G5..G10 prove the chosen transport.