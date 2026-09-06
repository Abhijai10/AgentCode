# DEP-ADM-007 — url @ 2.5.8

Need:
  Security Mode network-scope enforcement (G5-18/G5-52).  When a safe
  validation targets a network destination, the daemon must parse the target
  URL and compare host + default port against the explicitly approved
  Tool Broker allowlist (`scope.network_allowed`).  An authorized target A
  must never silently become arbitrary scanning.

Proposed dependency:
  url 2.5.8, crates.io (registry), pinned in Cargo.lock
  (checksum ff67a8a4397373c3ef660812acab3268222035010ab8680ec4215f38ba3d0eed).
  ac-daemon uses only `Url::parse` / `host_str` /
  `port_or_known_default`.  No `url` features are enabled.

Existing alternative check:
  The workspace had no URL parser.  Hand-rolled parsing would duplicate
  WHATWG URL semantics and is exactly the kind of security-boundary code
  that must not be reinvented (the parser decides what "outside the
  allowlist" means).  `url` was already in the resolved dependency graph
  as a transitive dependency of existing workspace crates.

License:
  MIT OR Apache-2.0 (dual, per LICENSE-APACHE and LICENSE-MIT in the local
  crate cache at
  ~/.cargo/registry/src/*/url-2.5.8/).  Permissive — license gate PASS
  with attribution.

Security status:
  Well-audited parser used across the Rust ecosystem; no open advisories
  known at admission time.  Used for policy decisions on input the daemon
  already controls; no network calls are made by the crate itself.

Maintenance state:
  Maintained by the Rust-in-Web community; steady release cadence; part of
  the already-locked workspace graph.

Runtime/bundle cost:
  No new packages added to the graph — url was already compiled as a
  transitive dependency; only the direct-dependency edge in ac-daemon is
  new.

Install/postinstall behavior:
  None.  Standard Cargo registry crate; no build scripts, no network
  access at build time.

Why existing components are insufficient:
  No in-tree component parses URLs.  Reimplementing URL parsing for a
  security boundary would be higher-risk than the dependency.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  G5 Security Mode gap-closure pass, 2026-09-04.  Evidence: Cargo.lock
  entry above; local crate LICENSE-APACHE/LICENSE-MIT inspected;
  `SECURITY-NETWORK_TARGET_BLOCKED` E2E in
  tests/integration/security_mode_flow.rs proves the allowlist decision
  path executes against a real parsed URL.
