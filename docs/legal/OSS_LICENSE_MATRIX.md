# OSS License Matrix

Status: ACTIVE (Phase 0). Updated by P01-WP02 and every dependency admission.

Classification legend:

- `DIRECT_RUNTIME_DEPENDENCY` — linked/bundled into AgentCode artifacts.
- `BINARY_TOOL_INVOCATION` — external tool invoked out-of-process.
- `FORK` — modified copy of upstream source maintained by AgentCode.
- `SOURCE_ADAPTATION` — AgentCode-derived source materially based on donor code.
- `PATTERN_STUDY_ONLY` — behavior studied; no code copied.
- `NO_PLANNED_REUSE` — cataloged, not used.

License gate verdicts: PASS / REVIEW / BLOCKED / PENDING (per `DEPENDENCY_ADMISSION.md`).

## Phase 2 Build/Runtime Dependencies

| Dependency | Version | License | Classification | Verdict | License file inspected |
|------------|---------|---------|----------------|---------|------------------------|
| rusqlite (bundled SQLite) | 0.37.x (pinned in Cargo.lock) | MIT (rusqlite) / Public Domain (SQLite) | DIRECT_RUNTIME_DEPENDENCY | PASS | yes — crates.io source + sqlite3 public domain notice |
| tracing | 0.1.x | MIT/Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | PASS | yes |
| tracing-subscriber | 0.3.x | MIT/Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | PASS | yes |
| serde / serde_json / toml | 1.x | MIT/Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | PASS | yes |
| anyhow | 1.x | MIT/Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | PASS | yes |
| thiserror | 2.x | MIT/Apache-2.0 | DIRECT_RUNTIME_DEPENDENCY | PASS | yes |
| pino | 9.x | MIT | DIRECT_RUNTIME_DEPENDENCY (TS) | PASS | yes |
| react / react-dom | 18.x | MIT | DIRECT_RUNTIME_DEPENDENCY (desktop renderer) | PASS | yes |
| tauri / tauri-build (Tauri 2) | 2.x | MIT/Apache-2.0 (Tauri core) | DIRECT_RUNTIME_DEPENDENCY (desktop) | PASS | yes |
| vite / typescript / eslint / vitest / prettier | pinned | MIT | DEV | PASS | yes |

## Foundation Extraction Sources (reference library)

| Repository | License | Classification | Verdict | Notes |
|------------|---------|----------------|---------|-------|
| OmniRoute | (see P01-WP02 record) | PATTERN_STUDY_ONLY / SOURCE_ADAPTATION (Phase 4) | PASS/PENDING | recorded in P01-WP02 |
| codex | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| aider | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| openhands | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| opencode | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| mini-swe-agent | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| tree-sitter | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| rtk | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| letta-code | ... | PATTERN_STUDY_ONLY | PASS | P01-WP02 |
| ... (complete list in P01-WP02) | | | | |

## Known Exceptions

(none yet — P1-G7: every exception recorded here with justification and owner sign-off)

## Governing Rules

1. No code is copied without a `SOURCE_ADAPTATION`/`FORK` classification, license
   inspection, and attribution in `THIRD_PARTY_NOTICES.md`.
2. `PATTERN_STUDY_ONLY` implies the AgentCode implementation is original; no
   verbatim code blocks are taken from the donor.
3. This matrix is updated in the same commit as the dependency admission.