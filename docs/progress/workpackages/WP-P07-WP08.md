# WP-P07-WP08 — Conflict Handling

- **Phase:** P07  
- **Status:** ACCEPTED  
- **Accepted commit:** phase completion commit

Integration returns a structured `MergeConflict` with files, base, ours, theirs and related task identity. It aborts and restores the pre-merge repository state; no ours/theirs strategy exists. Evidence: structured conflict test.
