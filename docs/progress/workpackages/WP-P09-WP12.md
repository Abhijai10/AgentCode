# WP-P09-WP12 — Workspace Detection

- **Phase:** P09
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 551e5be
- **Accepted commit:** 712d57b
- **Owner modules:** `crates/ac-code-intel`, `crates/ac-db`
- **Architecture refs:** Doc 09 Phase 9.9; Doc 10 P9-G8; Doc 11 P09-WP12
- **Acceptance gates:** P9-G8 PASS.

## Evidence

Implemented workspace/package boundary detection for npm package roots, pnpm, Turborepo, Nx, Cargo, Go workspaces and Python package roots, persisted in `workspace_boundaries`.

Validation: `phase9_semantics_survive_lsp_degradation`; `semantic_graph_evidence_survives_reopen`; full workspace check/test/clippy.

## Handoff

Boundaries are explicit graph context and can guide future LSP process scoping/resource policy.
