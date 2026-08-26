# DEP-ADM-006 — tokio @ 1.53.1

Need:
  Production Unix IPC must authenticate connecting peer credentials with a safe
  macOS-capable API while preserving `#![forbid(unsafe_code)]`.

Proposed dependency:
  tokio 1.53.1 from crates.io, already pinned transitively in Cargo.lock.
  ac-daemon uses only the `net` and `rt` features for `UnixStream::peer_cred`.

Existing alternative check:
  `rustix` is already locked but its safe `socket_peercred` API is Linux-only in
  the installed crate source. `libc::getpeereid` would require unsafe FFI inside
  AgentCode, which is forbidden.

License:
  MIT. Local crate cache contains LICENSE.

Security status:
  Used only to obtain effective peer credentials for a cloned Unix stream before
  IPC dispatch. No async runtime is introduced for request servicing.

Maintenance state:
  Widely maintained Rust async/networking project, already present in the
  resolved dependency graph.

Runtime/bundle cost:
  No new package download; direct daemon use of an existing locked dependency.

Install/postinstall behavior:
  No postinstall scripts. Standard Cargo registry crate.

Why existing components are insufficient:
  The available safe APIs in the existing graph either do not support macOS peer
  credentials or would require unsafe code in AgentCode.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  Codex implementation pass, 2026-08-26. Evidence: local Cargo registry metadata
  and LICENSE for tokio-1.53.1; focused IPC tests prove same-UID accept and
  wrong-UID rejection before dispatch.
