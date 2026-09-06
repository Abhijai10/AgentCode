# Post Phase 17 Architecture Hardening

Status: COMPLETE

Starting commit: `8309678`

Purpose: maintenance cleanup before Phase 18. This is not a numbered feature phase and does not reopen Phase 1 through Phase 17 completion status.

Problems fixed:
- Removed generated local hygiene artifacts: `.freebuff/` and macOS `._*` metadata files under source/test directories.
- Preserved useful extraction documentation under `docs/extraction/` as project documentation.
- Reduced maintenance risk from large single-file Rust crates.
- Added a first cross-crate integration test foundation.
- Added a formal daemon lifecycle runtime contract without adding a daemon loop, OS service integration, IPC expansion, or persistence changes.
- Added provider structured-output schema and validation preparation while preserving legacy scripted-provider compatibility.

Modules extracted:
- `crates/ac-db/src/lib.rs` now delegates to `models.rs`, `migrations.rs`, `agent.rs`, `context.rs`, `evidence.rs`, `security.rs`, `git.rs`, `verification.rs`, and `tests.rs`.
- `crates/ac-security/src/lib.rs` now delegates to `capability.rs`, `extensions.rs`, `hooks.rs`, `mcp.rs`, `scanner.rs`, `findings.rs`, and `tests.rs`.
- `crates/ac-runtime/src/lib.rs` now delegates to `tasks.rs`, `workers.rs`, `scheduler.rs`, `recovery.rs`, `autonomy.rs`, `session.rs`, and `tests.rs`.

Integration tests added:
- `tests/integration/autonomous_flow.rs` covers Kernel mission through autonomous agent execution, provider/context/tool/sandbox/change-set/verification/evidence/completion.
- `tests/integration/security_boundary.rs` covers workspace escape denial, denied Tool Broker capability, and unsafe MCP invocation rejection.
- `tests/integration/recovery_flow.rs` covers interrupted daemon session reload and recovery.
- `tests/benchmark/README.md` and `tests/chaos/README.md` establish lightweight future test areas without adding runtime cost.

Daemon contract added:
- `DaemonLifecycleRuntime` defines `start`, `stop`, `restart`, `recover`, `heartbeat`, and `shutdown`.
- `DaemonService` implements the contract using the existing lifecycle state and persistence surface.

Provider schema preparation:
- Added `StructuredAgentResponse`, `ProviderResponseFormat`, and `ValidatedProviderResponse`.
- Added strict structured response validation and explicit legacy fallback parsing for current line-oriented/scripted output.
- Invalid structured provider output is rejected.

Validation results:
- `cargo fmt --all`
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`

Migration impact: none. No database schema or migration version changed in this hardening sprint.

Limitations:
- `ac-db` uses included source fragments to preserve private field access and public API behavior during decomposition.
- Provider structured parsing intentionally supports the narrow future schema shape without adding a JSON dependency.
