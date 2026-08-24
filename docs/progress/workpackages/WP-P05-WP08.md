# WP-P05-WP08 — Process Manager

- **Phase:** P05
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 5aa0fcefe726e4e40660fc633b8ef5542c663180
- **Accepted commit:** `TBD (phase 5 completion commit)`
- **Owner modules:** `crates/ac-tool`

## Acceptance Evidence

`ProcessManager` records process id, task, argv/cwd manifest, OS pid, lifecycle state, and timestamp. It supports foreground timeout/cancellation plus background start, inspect, and cancel cleanup.

## Validation

`process_manager_times_out_and_cancels_background_process` PASS.
