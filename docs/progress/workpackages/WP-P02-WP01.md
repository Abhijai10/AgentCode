# WP-P02-WP01 — Workspace and Foundation Layer

- **Phase:** P02
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 61705201e6bc513c2c8ba9495a9941617dc7af8c
- **Accepted commit:** completion package commit for this record
- **Owner modules:** Cargo.toml, Cargo.lock, Makefile, crates/
- **Architecture refs:** ADR-0001; ADR-0002; ADR-0003; ADR-0004; ADR-0009; ADR-0010; P01 WP03-WP17 implementation packets
- **Acceptance gates:** P2-G1..P2-G8

## Objective

Complete the Phase 2 production implementation substrate with a Rust workspace, shared primitives, configuration/logging foundations, Kernel authority foundation, append-only Evidence Store foundation, ChangeSet lifecycle foundation, runtime execution skeleton, controlled subsystem boundaries, migration harness, CI validation, fixtures, and daemon/desktop placeholders.

## Implemented In This Batch

- Root Cargo workspace and Makefile command surface.
- `ac-common`: stable ids, timestamp primitive, typed error taxonomy.
- `ac-config`: dependency-free typed layer builder with secret references.
- `ac-logging`: structured log records and exact-value redaction.
- `ac-evidence`: append-only in-memory evidence records with provenance.
- `ac-changeset`: proposed change lifecycle with approval-before-apply constraint.
- `ac-kernel`: Kernel lifecycle, mission state machine, event log, policy boundary.
- `ac-runtime`: Worker and AgentSession skeleton with event loop and cancellation.
- `ac-daemon`: minimal daemon placeholder that starts Kernel foundation.
- `ac-provider`: provider/model registry, normalized request, stream events, failure classes, route attempts.
- `ac-tool`: Tool Broker, typed tool definitions, capability evaluation, evidence-backed results.
- `ac-git`: repository/worktree/checkpoint records with branch validation.
- `ac-code-intel`: repository indexing, symbol extraction, text search, degraded symlink handling.
- `ac-context`: context packs, authority classes, memory facts, supersession.
- `ac-security`: capability policy, extension registry, finding store.
- `ac-sandbox`: execution/filesystem policy evaluation and sandbox plan creation.
- `ac-verification`: browser/validation/security-scan evidence interfaces.
- `ac-migrations`: deterministic migration ledger plus V1 kernel schema SQL.
- `ac-desktop-placeholder`: minimal desktop launch placeholder.
- CI workflow, integration-test structure, and fixture repository.

## Boundaries Preserved

- Kernel owns mission decisions and state transitions.
- Runtime sessions emit temporary events and do not own global state.
- Evidence records are append-only.
- ChangeSets model intent and cannot be applied until validated and Kernel-approved.
- Provider, tool, browser, scanner, git, code intelligence, context, memory, sandbox, and security foundations expose controlled interfaces only; no external provider, shell, browser, scanner, or unrestricted filesystem implementation was added.

## Validation

- `cargo fmt --all -- --check` PASS.
- `cargo check --workspace --all-targets` PASS.
- `cargo test --workspace --quiet` PASS.
- `cargo clippy --workspace --all-targets --quiet -- -D warnings` PASS.
- `cargo build -p ac-daemon --quiet` PASS.
- `cargo build -p ac-desktop-placeholder --quiet` PASS.
- `cargo run -p ac-daemon --quiet` PASS (`ac-daemon Info`).
- `cargo run -p ac-desktop-placeholder --quiet` PASS.

## Gate Results

- P2-G1 PASS: workspace builds.
- P2-G2 PASS: unit test command works.
- P2-G3 PASS: integration test structure exists.
- P2-G4 PASS: CI workflow runs basic validation commands.
- P2-G5 PASS: migration harness applies V1 schema migration once.
- P2-G6 PASS: structured logger exists with redaction.
- P2-G7 PASS: fixture repository exists.
- P2-G8 PASS: daemon and desktop placeholders build and launch.
