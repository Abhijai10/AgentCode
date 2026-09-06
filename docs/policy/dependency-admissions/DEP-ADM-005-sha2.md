# DEP-ADM-005 — sha2 @ 0.10.9

Need:
  FastEmbed model artifact integrity requires a real SHA-256 digest for the local AgentCode model manifest.

Proposed dependency:
  sha2 0.10.9 from crates.io, pinned by Cargo.lock.

Existing alternative check:
  The workspace had no direct SHA-256 implementation. Existing ad hoc hashes are non-cryptographic and unsuitable for artifact integrity.

License:
  MIT OR Apache-2.0. Local crate cache contains LICENSE-MIT and LICENSE-APACHE.

Security status:
  Used only for local digest calculation. No network or postinstall behavior. Final OSV scan covers the locked crate graph.

Maintenance state:
  Maintained by RustCrypto. Small, widely used Rust hashing crate.

Runtime/bundle cost:
  Small pure-Rust dependency; used only during model manifest verification.

Install/postinstall behavior:
  No postinstall scripts. Standard Cargo registry crate.

Why existing components are insufficient:
  FNV/local hashes in the repo are intended for stable identifiers, not integrity checks.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  Codex implementation pass, 2026-08-26. Evidence: local Cargo registry metadata reports `license = "MIT OR Apache-2.0"` and license files are present.
