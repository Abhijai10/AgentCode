# WP-P06-WP01 — Agent Session

- **Phase:** P06
- **Status:** IMPLEMENTING
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 593e649935a1391df26c3d74cc17bb3f48f2b615
- **Accepted commit:** not accepted yet
- **Owner modules:** `crates/ac-agent`, `crates/ac-runtime`, `crates/ac-tool`
- **Architecture refs:** P01 WP03-WP10 implementation packets; P02 completion package
- **Acceptance gates:** P6 agent-loop gates not fully evaluated in this batch

## Implemented In This Batch

- Added `ac-agent` with goal representation, deterministic planner, ordered plan steps, stopping condition, checkpoints, resume guard, cancellation, retry, verification, and ChangeSet preparation.
- Connected planner/runtime flow through Provider Fabric, Context Engine, Tool Broker, Evidence Store, Verification Engine, ChangeSet, and Kernel approval.
- Added provider trait and scripted provider adapter for local tests without hardcoding provider logic into runtime.
- Added Tool Broker local workspace tools for file read, file write, directory list, search, and sandbox-planned command execution.
- Added tests for planner ordering, simple coding task to ChangeSet, tool failure recovery, cancellation, and checkpoint resume.

## Boundaries Preserved

- Planner creates plans only and does not execute tools.
- Agent logic invokes tools through Tool Broker.
- File writes happen inside tool executors and are represented as ChangeSets for Kernel approval.
- Evidence is recorded for tool results and verification.
- Context packs label authority class and do not treat retrieval as truth.

## Validation

- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.

## Remaining Before Acceptance

- Full production provider adapters.
- Real git worktree creation/isolation against repositories.
- Full build/test/lint command profiles per project.
- Durable persisted AgentSession resume path through daemon.
- Completion request protocol and full benchmark gate.
