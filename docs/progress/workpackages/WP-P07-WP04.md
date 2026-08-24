# WP-P07-WP04 — Checkpoint Service

- **Phase:** P07  
- **Status:** ACCEPTED  
- **Accepted commit:** phase completion commit

Checkpoints commit the task worktree and record commit, reason, task attempt, test summary, context reference and blocker metadata. SQLite persists these recovery anchors. Evidence: checkpoint/reopen tests in `ac-git` and `ac-db`.
