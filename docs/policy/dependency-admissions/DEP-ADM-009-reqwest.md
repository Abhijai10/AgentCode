# DEP-ADM-009 — reqwest @ 0.12

Need:
  The OmniRoute provider layer (ac-provider) performs real HTTP calls to
  configured model providers (Ollama local, OpenAI-compatible, Anthropic,
  Gemini). A blocking HTTP client with TLS, streaming reads, and header
  control is required.

Proposed dependency:
  reqwest 0.12 from crates.io (default-features off; features: blocking, json,
  rustls-tls), pinned by Cargo.lock.

Existing alternative check:
  The workspace had no HTTP client. Raw TcpStream HTTP (used only for the
  localhost Chrome DevTools endpoint inside ac-verification) is not viable for
  remote TLS provider endpoints and would need a TLS stack anyway.

License:
  MIT OR Apache-2.0. Local crate cache contains LICENSE-MIT and LICENSE-APACHE.

Security status:
  Client-only; no server surface. TLS via rustls (no OpenSSL linkage).
  Requests go only to user-configured provider endpoints; no telemetry.

Maintenance status:
  Maintained by seanmonstar; the standard Rust HTTP client.

Runtime/bundle cost:
  Moderate binary-size cost from rustls; required for any remote provider.

Install/postinstall behavior:
  No postinstall scripts. Standard Cargo registry crate.

Why existing components are insufficient:
  No workspace HTTP client existed.

Note (gap honesty): reqwest was already in use by `ac-provider` before this
record existed (implementation drift per AGENTS.md §7). This admission
retroactively documents the pre-existing use; no new crate is added by this
record. Provider traffic remains inside ac-provider (OmniRoute owns provider
calls); ac-daemon does NOT gain a reqwest dependency.

License gate result: PASS
Admission decision: ADMIT
Reviewer + date + evidence refs:
  G-batch closure audit, 2026-08-27. Evidence: local Cargo registry metadata
  reports `license = "MIT OR Apache-2.0"`; license files present in cache;
  crates/ac-provider/Cargo.toml line 10 pins the exact feature set.
