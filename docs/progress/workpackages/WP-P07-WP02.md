# WP-P07-WP02 — Worktree Registry

- **Phase:** P07  
- **Status:** ACCEPTED  
- **Accepted commit:** phase completion commit

`GitCoordinator` records repository, mission, worker, lease epoch, branch, base/head, path and status. `ControlPlaneDb` persists the record and reopening tests preserve ownership. Gate evidence: `ac-git` isolation/replacement tests and `ac-db` reopen test. Limitation: filesystem reconciliation is invoked by the Kernel recovery path; scheduling of periodic reconciliation is deferred.
