# Third-Party Notices

AgentCode is distributed under the license chosen by its owners (to be recorded in
the repository root before any release). It includes or is built with third-party
material listed below, in compliance with each upstream license.

Generated/maintained by the license-review workflow (`docs/legal/LICENSE_REVIEW.md`)
and `docs/legal/third_party_manifest.json` (machine-readable companion).

## Runtime Dependencies (Rust)

| Package | License | Copyright notice / attribution |
|---------|---------|--------------------------------|
| rusqlite | MIT | Copyright (c) 2014-2025 The rusqlite developers; SQLite is public domain |
| tracing | MIT OR Apache-2.0 | Copyright (c) 2019 Tokio Contributors |
| tracing-subscriber | MIT OR Apache-2.0 | Copyright (c) 2019 Tokio Contributors |
| serde | MIT OR Apache-2.0 | Copyright (c) 2006-2025 David Tolnay et al. |
| serde_json | MIT OR Apache-2.0 | Copyright (c) 2016-2025 David Tolnay et al. |
| toml | MIT OR Apache-2.0 | Copyright (c) 2014-2025 Alex Crichton et al. |
| anyhow | MIT OR Apache-2.0 | Copyright (c) 2018-2025 David Tolnay |
| thiserror | MIT OR Apache-2.0 | Copyright (c) 2016-2025 David Tolnay |

## Runtime Dependencies (TypeScript / Desktop)

| Package | License | Copyright notice / attribution |
|---------|---------|--------------------------------|
| pino | MIT | Copyright (c) 2016-2025 pino contributors |
| react | MIT | Copyright (c) Meta Platforms, Inc. and affiliates |
| react-dom | MIT | Copyright (c) Meta Platforms, Inc. and affiliates |
| tauri | MIT OR Apache-2.0 | Copyright (c) 2019-2025 Tauri Contributors |

## Development / Build Tools

| Package | License | Notes |
|---------|---------|-------|
| vite | MIT | build tool |
| typescript | Apache-2.0 | compiler |
| eslint | MIT | lint |
| vitest | MIT | test runner |
| prettier | MIT | formatter |
| @vitejs/plugin-react | MIT | build plugin |
| @types/react etc. | MIT | type defs |

## Reference-Research Attribution

The following repositories were studied as research input during Phase 1 extraction
(pattern study). No code was copied; see `docs/extraction/` records for the pinned
SHAs and exact source paths reviewed:

- OmniRoute, codex, aider, OpenHands, opencode, mini-SWE-agent, SWE-ReX,
  tree-sitter, ast-grep, ripgrep, Letta Code, RTK, Gemini CLI, cline, goose,
  LangGraph, Microsoft Agent Framework, Playwright, OpenHack, and others recorded in
  `docs/reference/AGENTCODE_REFERENCE_CATALOG.md`.

Attribution obligations for any `SOURCE_ADAPTATION`/`FORK` material (none admitted in
Phase 2) will be added here before the material is committed, per Doc 07 §93.

---
*This file is generated/maintained by the license-review workflow; the authoritative
machine-readable record is `third_party_manifest.json`.*