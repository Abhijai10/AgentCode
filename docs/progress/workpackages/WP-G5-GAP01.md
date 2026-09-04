# WP-G5-GAP01 - Security Mode gap closure: scanner-coverage truth, credential lifecycle, quality metrics, canonical final status

Status: ACCEPTED
Base commit: 94b8ca1 (feat(security): implement Security Mode — first-class audit pipeline, scope enforcement, safe validation, remediation, regression, report)
Phase: G5 (Security Mode batch, post-P22 desktop product track)
Risk: HIGH per playbook (security boundary + schema change)

## Objective

Close the genuine gaps left in the G5 Security Mode batch after the primary
commit: (1) network-scope escape enforcement at the Tool Broker boundary
(G5-18), (2) credential-exposure lifecycle for secret findings with
human-approval-gated rotation (G5-26), (3) security quality metrics derived
from persisted state (G5-35), (4) self-security scanning of
repository-controlled text (G5-20/G5-37), (5) canonical final-status matrix
that never collapses to PASS/FAIL (G5-52), (6) multi-seed synthetic canaries
(G5-43), (7) regression-freshness re-verification (G5-24).

## Implementation Summary

- `ac_security::scan_repository_instruction_attempts` fences
  repository-controlled text: privileged-instruction attempts are detected
  and recorded as bounded evidence (max 200 chars of the offending line) and
  become findings; the payload never gains instruction privilege.
- `SecretExposureLifecycle` models the post-confirmation credential steps
  (remove → assess history/distribution → rotate/revoke → verify
  replacement → rescan); `requires_human_approval` is true for assessment
  and rotation — the daemon never rotates autonomously.
- `FinalSecurityStatus::derive` is the single canonical matrix (7 states,
  precedence: retest-failed > confirmed findings > manual review > scope
  blocking > incomplete validation > scanner coverage). Both the live
  status snapshot and the persisted report derive through it, so they can
  never disagree.
- `regression_state_after_retest` re-verifies every regression obligation
  against the CURRENT commit: reappeared finding ⇒ Broken
  (REGRESSION_DETECTED + reopen), old-commit pin ⇒ Stale.
- Network scope enforcement in `SecurityValidate`: the validation target is
  parsed with `url` and host:port must be inside the explicit allowlist
  before any active validation proceeds; violations persist a Blocked
  validation outcome and return `SECURITY-NETWORK_TARGET_BLOCKED`.
- Multi-seed canaries: when the caller pins no marker, validation tries
  every seeded synthetic canary so any planted marker proves the read;
  the response reports only the marker that was actually proved.
- Fake-data repair (this WP's core review finding): the earlier draft fed
  `scanners_unavailable = 0` and empty scanner lists into the final-status
  matrix and report. Migration 0028 adds `scanners_unavailable`,
  `available_scanners`, `unavailable_scanners` columns; the audit persists
  the real executed/failed adapter lists; `security_status` and
  `build_security_mode_report` now read the persisted truth (report builder
  signature gains `manual_review_count` so the NeedsManualReview lifecycle
  state is counted from authoritative rows).

## Affected Files

- `crates/ac-security/src/mode.rs` — self-security scan, credential
  lifecycle, canonical final status, regression freshness
- `crates/ac-daemon/src/security.rs` — network allowlist gate, multi-seed
  canary proof, retest regression re-verification + RETEST_FAILED,
  secret-lifecycle + quality-metrics endpoints, persisted scanner truth
- `crates/ac-daemon/src/ipc.rs` — SecuritySecretLifecycle /
  SecurityQualityMetrics dispatch
- `crates/ac-daemon/Cargo.toml` — direct `url` dependency (DEP-ADM-007)
- `crates/ac-db/src/migrations.rs` — schema version 28 (0026 extension
  columns via add_column_if_missing)
- `crates/ac-db/src/models.rs`, `crates/ac-db/src/security_mode.rs` —
  session row + save/load with scanner-coverage fields
- `migrations/0026_security_mode.sql` — new-session defaults
- `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src/app/daemon.ts`,
  `apps/desktop/src/app/types.ts` — IPC bridge + typed wrappers
- `apps/desktop/src/app/SecurityView.tsx` — credential lifecycle steps in
  finding detail (approval-flagged), quality panel (confirmed rate,
  false-positive dismissal rate, canary proofs, blocked attempts,
  regression protections incl. broken), scanner-coverage status line
- `tests/integration/security_mode_flow.rs` — self-security +
  authorized-adversarial + scope-escape E2E; report/status assertions for
  real scanner availability and canonical states
- `docs/policy/dependency-admissions/DEP-ADM-007-url.md`,
  `docs/legal/third_party_manifest.json` (CAND-0029),
  `docs/legal/THIRD_PARTY_NOTICES.md` — url admission record

## Persistence

Migration step 28 (add_column_if_missing on security_mode_sessions; schema
CURRENT_SCHEMA_VERSION = 28). Existing version-27 databases upgrade
in place; fresh databases get the columns from the 0026 file. Databases
created by the earlier commit keep their data — no destructive change.

## Tests

- `security_mode_end_to_end_lifecycle_scope_validation_and_report`:
  16-phase deterministic lifecycle through real daemon IPC — now also
  asserts the report Tools section carries real adapter status and that
  the status endpoint exposes the persisted scanners-unavailable count
  consistent with the report Limitations section.
- `security_mode_self_security_and_authorized_adversarial_gates`:
  repository prompt-injection attempt becomes a High finding with the
  payload redacted; AI surfaces normalize into the same findings DB;
  authorized local scope proves the seeded canary (CanaryRetrieved →
  Confirmed); production read-only blocks the same validation
  (SECURITY-VALIDATION_NOT_AUTHORIZED); network target outside the
  allowlist is blocked (SECURITY-NETWORK_TARGET_BLOCKED); credential
  lifecycle exposes 5 steps with ROTATE_OR_REVOKE requiring human
  approval; quality metrics derive from persisted state.
- `schema_28_upgrades_security_mode_sessions_from_version_27_without_losing_data`
  (ac-db): recreates a pre-gap version-27 database, runs migrate(),
  and proves the security session row survives with its data, the new
  scanner-coverage columns default honestly, and save/read round-trips.
  **This test caught a real bug before commit**: the original
  positional `INSERT ... VALUES (?1..?16)` wrote values into the wrong
  physical columns on ALTER-upgraded databases (column order differs:
  appended columns land at the end). Fixed by switching to an explicit
  column-list INSERT in `save_security_mode_session`.
- Workspace suite: 370 passed / 5 failed — the 5 failures are pre-existing
  in `ac-tool` (P05 sandbox runtime) and fail identically on clean HEAD;
  root cause is this environment denying `sandbox-exec`
  (`sandbox_apply: Operation not permitted`, verified directly). Not a
  G5 regression; recorded as an environment blocker below.

## Verification Evidence

- `cargo test --test security_mode_flow` → 2 passed (2026-09-04).
- `cargo test -p ac-db` → 40 passed incl. the new schema 27→28
  upgrade-path test (2026-09-04).
- `cargo test --workspace` → 370+1 passed, 5 pre-existing failures
  (ac-tool sandbox; see Limitations).
- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets` → no warnings/errors.
- `pnpm run typecheck` (apps/desktop) → clean.
- `pnpm run lint` (apps/desktop) → clean, zero warnings.
- Sandbox-exec denial reproduced: `sandbox-exec -f <profile> /bin/true`
  exits 71 with "sandbox_apply: Operation not permitted".

## Limitations

- The 5 ac-tool sandbox tests require macOS `sandbox-exec`, which this
  environment denies at the platform level. They pre-date G5 (identical
  result on clean HEAD) and block only seatbelt-isolation verification,
  not Security Mode. REQUIRES CI / EXTERNAL HARDWARE to exercise.
- `SecuritySecretLifecycle` reports the required credential steps; the
  rotation action itself is deliberately NOT automated — it stays behind
  explicit human approval by design (G5-26), so no daemon command exists
  to rotate a credential.
- Quality metrics derive from persisted conversation state; they are not a
  cross-project aggregate (that would require a Kernel-level rollup WP).

## Acceptance Status

ACCEPTED for G5 gap closure. All security-mode surfaces run through the
existing Kernel/daemon/SQLite control plane; no second daemon, DB, or tool
broker was introduced; every new endpoint has a production caller (UI) and
a deterministic test.
