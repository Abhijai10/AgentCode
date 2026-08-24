# WP-P09-WP01 — LSP Protocol Abstraction

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`, `crates/ac-migrations`
- **Architecture refs:** Doc 09 Phase 9.1-9.4; Doc 10 P9-G1; Doc 11 P09-WP01; P01 WP07 packet
- **Acceptance gates:** P9-G1 PASS via `CodeIntelligenceService::index_repository` creating typed `LspServerSession` records for supported languages.

## Evidence

Implemented `LspServerKind`, `LspSessionState`, `LspServerSession`, and `ensure_lsp_session` as the Phase 9 protocol surface. Sessions are derived during repository indexing and exposed as typed state, with durable row export for `lsp_server_sessions`.

Validation: `cargo fmt --all`; `cargo check --workspace --all-targets`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; JSON validation excluding macOS `._*`; `git diff --check`.

## Handoff

Downstream packages may rely on LSP-shaped session state being produced from the real code-intel indexing path. External language-server binaries remain an optional later enhancement; absence is explicit degraded/unavailable state, not silent success.
