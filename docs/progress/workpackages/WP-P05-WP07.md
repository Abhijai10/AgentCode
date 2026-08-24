# WP-P05-WP07 — Shell Executor

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** CRITICAL | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-tool`, `crates/ac-sandbox`

## Acceptance Evidence

`cmd.exec` consumes newline-delimited argv, sanitizes environment, validates cwd/network/capability policy in SandboxManager, captures stdout/stderr/exit status, and returns deterministic spawn, wait, timeout, and cancellation errors.

## Validation

`workspace_command_executes_through_sandbox_plan` and timeout/cancellation coverage PASS.
