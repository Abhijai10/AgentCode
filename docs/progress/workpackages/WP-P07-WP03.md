# WP-P07-WP03 — Task Branch Creation

- **Phase:** P07  
- **Status:** ACCEPTED  
- **Accepted commit:** phase completion commit

Task worktrees are created through `git worktree add -b` from a clean, explicit base. Dirty user bases fail with `GIT-DIRTY_BASE` without alteration. Branch validation rejects unsafe names. Evidence: `ac-git` dirty-base and parallel-isolation tests.
