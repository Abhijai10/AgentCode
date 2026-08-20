# WP-P02-WP01 — Workspace and Foundation Layer

- **Phase:** P02
- **Status:** IMPLEMENTING
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 61705201e6bc513c2c8ba9495a9941617dc7af8c
- **Accepted commit:** not accepted yet
- **Owner modules:** Cargo.toml, Cargo.lock, Makefile, crates/
- **Architecture refs:** ADR-0001; ADR-0002; ADR-0003; ADR-0009; ADR-0010; P01 WP04/WP05/WP08/WP09/WP10 packets
- **Acceptance gates:** P2-G1, P2-G2, P2-G6 partial foundation coverage

## Objective

Start the Phase 2 production implementation substrate with a Rust workspace, shared primitives, configuration/logging foundations, Kernel authority foundation, append-only Evidence Store foundation, ChangeSet lifecycle foundation, Runtime skeleton, and daemon placeholder.

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

## Boundaries Preserved

- Kernel owns mission decisions and state transitions.
- Runtime sessions emit temporary events and do not own global state.
- Evidence records are append-only.
- ChangeSets model intent and cannot be applied until validated and Kernel-approved.
- No provider, tool, browser, scanner, or full agent reasoning implementation was added.

## Validation

Recorded in final response for this batch. This WP remains IMPLEMENTING until full P02 acceptance gates are completed.
