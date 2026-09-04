# DEP-ADM-008 — base64 @ 0.23.1

Need:
  Reference-image analysis (Design Studio, core Doc 06 H28) sends PNG bytes to
  vision-capable local models via the provider layer, which requires standard
  base64 encoding of image payloads. Also used by the verification crate for
  screenshot evidence digests.

Proposed dependency:
  base64 0.23.1 from crates.io, pinned by Cargo.lock.

Existing alternative check:
  The workspace had no base64 implementation. Hand-rolling MIME-safe base64 for
  multimodal provider payloads would be error-prone security-sensitive code for
  zero benefit.

License:
  MIT OR Apache-2.0. Local crate cache contains LICENSE-MIT and LICENSE-APACHE.

Security status:
  Pure encoding, no network or postinstall behavior. Only encodes local image
  bytes bound for a user-configured local model endpoint. Final OSV scan covers
  the locked crate graph.

Maintenance state:
  Maintained by the base64-rs organization; the standard Rust base64 crate.

Runtime/bundle cost:
  Tiny pure-Rust dependency.

Install/postinstall behavior:
  No postinstall scripts. Standard Cargo registry crate.

Why existing components are insufficient:
  No prior workspace crate provides RFC 4648 base64 encoding.

Note (gap honesty): base64 was already in use by `ac-verification` before this
record existed; this admission retroactively documents that pre-existing use
(implementation-drift repair per AGENTS.md §7) and extends it to `ac-daemon`
for the H28 reference-image flow.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  G-batch closure audit, 2026-08-27. Evidence: local Cargo registry metadata
  reports `license = "MIT OR Apache-2.0"`; license files present in cache.
