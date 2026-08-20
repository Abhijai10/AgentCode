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

- Added `ac-agent` with goal representation, provider-informed planner, ordered plan steps, stopping condition, checkpoints, resume guard, cancellation, retry, verification, and ChangeSet preparation.
- Connected planner/runtime flow through Provider Fabric, Context Engine, Tool Broker, Evidence Store, Verification Engine, ChangeSet, and Kernel approval.
- Added provider trait and scripted provider adapter for local tests without hardcoding provider logic into runtime.
- Added Tool Broker local workspace tools for file read, file write, directory list, search, and sandbox-planned command execution.
- Added tests for planner ordering, simple coding task to ChangeSet, tool failure recovery, cancellation, and checkpoint resume.
- Added explicit planner objective, expected outcomes, and stopping condition fields.
- Reordered the plan lifecycle so ChangeSet intent is prepared and Kernel-approved before tool execution.
- Added ContextBuilder integration that assembles Kernel goal state, raw evidence, accepted memory, and ranked retrieval candidates with authority labels.
- Added isolated workspace agent construction that creates a task workspace, registers workspace tools only against that root, and keeps source files unchanged.
- Added deterministic verification repair loop with repair evidence before retry.
- Added autonomous demo test that fixes a fixture inside an isolated workspace and proves the source repository remains unchanged.
- Reworked planning flow to collect workspace context, stream provider reasoning through the ProviderRegistry adapter lifecycle, then build an execution graph with assumptions, required files, dependencies, expected verification, and stopping condition.
- Connected verification to actual Tool Broker development commands (`dev.test`/`repo.diff`) instead of recording validation success directly.
- Added repair retry behavior that records failed verification, asks the provider path for repair context, reapplies the proposed change through Tool Broker, and reruns verification.

## Boundaries Preserved

- Planner creates plans only and does not execute tools.
- Agent logic invokes tools through Tool Broker.
- File writes happen inside tool executors and are represented as ChangeSets for Kernel approval.
- Evidence is recorded for tool results and verification.
- Context packs label authority class and do not treat retrieval as truth.
- Git/worktree state is attached as task evidence and does not become hidden agent memory.
- Verification failure produces evidence and retry behavior; it is not silently converted to success.
- Provider output informs plans but does not execute tools or own policy.
- Verification tools run through Tool Broker and Sandbox boundaries.

## Validation

- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.

## Remaining Before Acceptance

- Full production provider adapters.
- Provider output parsing is line-oriented; structured plan schemas are still future work.
- Full build/test/lint command profile discovery per project.
- Durable persisted AgentSession resume path through daemon.
- Completion request protocol and full benchmark gate.
- Rich multi-file repair planning beyond deterministic fixture repair.
