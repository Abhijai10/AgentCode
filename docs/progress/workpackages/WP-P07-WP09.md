# WP-P07-WP09 — Cleanup

- **Phase:** P07  
- **Status:** ACCEPTED  
- **Accepted commit:** phase completion commit

Cleanup refuses dirty worktrees, never uses forced removal, and marks deleted directories `Missing` during reconciliation. Evidence: P7 isolation/reconciliation test.
