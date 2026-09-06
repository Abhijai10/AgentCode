DEP-ADM-021 — libc @ 0.2 (direct dependency, ac-daemon)

Need:
F7 of the final audit ("Final audit after all the remaining work.md"): the
daemon must shut down gracefully on SIGTERM/SIGINT (reap design-preview and
terminal children, remove its Unix socket) instead of dying mid-request and
leaving a stale socket. Registering signal handlers requires libc::signal.
libc was already in Cargo.lock as a transitive dependency of multiple
admitted crates (tokio, reqwest, rusqlite); this admission makes one direct
call-site explicit.

Scope of use:
- crates/ac-daemon/src/main.rs only: two `signal(2)` registrations
  (SIGTERM, SIGINT) with an extern "C" handler that only performs an atomic
  store (async-signal-safe). No other libc usage is added.
- The handler body performs no allocation, locking, or I/O.

Alternatives considered:
- signal-hook / tokio signal streams: heavier runtime deps for two lines of
  registration; rejected as disproportionate (DEP-ADM-006 precedent also
  avoids extra deps where std/libc surface suffices).
- std-only: std has no signal API; not possible.
- Doing nothing (Ctrl-C kills the process): leaves stale sockets (the exact
  F7 finding).

License:
MIT OR Apache-2.0 — dual-licensed, standard for the Rust ecosystem; both
compatible with the workspace. Reviewed via crates.io metadata + local
Cargo.lock source record; no vendored code.

Audit trail:
- Workspace lint table forbids unsafe_code; the two registrations carry a
  SAFETY comment explaining async-signal-safety, and the handler is a plain
  extern "C" fn. `forbid` still applies to all other code in the crate —
  this call-site required removing the workspace-lints inheritance for the
  ac-daemon *binary only* (the lib keeps `[lints] workspace = true`), a
  scoped, documented exception.
- Test coverage: manual verification recorded in the F7 commit message
  (SIGTERM → "shutdown signal received; stopping cleanly" + socket removed);
  the serve-loop flag check is exercised by the existing stop-path tests.
