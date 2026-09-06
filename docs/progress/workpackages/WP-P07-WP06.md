# WP-P07-WP06 — Rollback

- **Phase:** P07  
- **Status:** ACCEPTED  
- **Accepted commit:** phase completion commit

Recovery resets only the owning task worktree to a named checkpoint; `restore_to_base` restores its recorded base. Source and unrelated worktrees are not changed. Evidence: worktree isolation/recovery tests.
