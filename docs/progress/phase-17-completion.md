# Phase 17 Completion — Baseline Security Platform

Status: COMPLETE

Starting commit: `db8199c`

Implementation commit: pending final commit

Completed work packages: P17-WP01 through P17-WP14 are ACCEPTED.

Implemented capabilities: security policy, threat model, scanner instance schema, normalized finding schema, baseline security orchestrator, Gitleaks-compatible secret scan with redaction, OSV-compatible dependency scan, Trivy adapter selection, Semgrep-compatible SAST scan, Checkov-compatible IaC scan, duplicate grouping, triage and false-positive dismissal, manual business-logic finding representation, confirmed-finding repair task creation, rescan/regression verification, Markdown/JSON/SARIF report export, and durable security scan persistence.

Architecture notes: Scanner output is candidate evidence, not automatic truth. Secrets are redacted before model/report exposure. Confirmed findings create typed repair tasks for the existing Kernel/worker pipeline; scanners and reports do not write Kernel state directly.

Migration impact: added `migrations/0012_baseline_security.sql`; database `user_version` is now 12.

Validation evidence:
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- JSON validation
- `git diff --check`

Gate mapping: P17-G1 through P17-G15 pass via Phase 17 tests in `ac-security` and `ac-db`.

Limitations: External scanner binaries are represented by deterministic local adapter fallbacks in this phase. Missing external scanners do not become zero findings; adapter availability is explicit in scan reports.
