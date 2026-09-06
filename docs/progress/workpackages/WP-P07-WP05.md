# WP-P07-WP05 — Worker Replacement

- **Phase:** P07  
- **Status:** ACCEPTED  
- **Accepted commit:** phase completion commit

Lease transfer increments a persisted fencing epoch. A stale worker receives `GIT-STALE_WORKTREE_LEASE`; a replacement can recover the latest checkpoint. Evidence: `phase_seven_isolates_workers_replaces_owner_and_detects_missing_worktree`.
