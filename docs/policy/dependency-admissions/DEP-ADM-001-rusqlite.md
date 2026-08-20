# DEP-ADM-001 — rusqlite @ 0.32.x

Need:
  SQLite-backed Kernel/control-plane persistence for Phase 3 durable state and ADR-0004.

Proposed dependency:
  `rusqlite`, version `0.32.1`, crates.io, pinned by `Cargo.lock`, feature `bundled`.

Existing alternative check:
  The Phase 2 migration ledger validates SQL but does not execute SQLite storage. A direct SQLite driver is required for durable Kernel ownership.

License:
  Declared crate license: MIT. `libsqlite3-sys 0.30.1` binding license: MIT. Bundled SQLite: Public Domain. Classification: permissive/pass per `docs/legal/OSS_LICENSE_MATRIX.md`. License files inspected from the local Cargo registry after lockfile resolution.

Security status:
  No network/postinstall script model; compiled Rust crate plus bundled SQLite C source via Cargo build. Advisory scanning is deferred to CI security tooling.

Maintenance state:
  Established Rust SQLite binding selected by ADR-0004.

Runtime/bundle cost:
  Adds SQLite driver and bundled SQLite build artifacts; acceptable for deterministic local control-plane storage.

Install/postinstall behavior:
  Cargo build compiles bundled SQLite; no global install, no runtime download.

Why existing components are insufficient:
  AgentCode needs actual SQLite execution, transactions, pragmas, and query APIs; a text migration ledger cannot satisfy durable Kernel persistence.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  AgentCode implementation run, 2026-08-21; ADR-0004; `Cargo.lock`; `crates/ac-db`.
