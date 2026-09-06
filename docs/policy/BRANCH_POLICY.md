# Branch / Integration Policy

Status: ACCEPTED (P00-WP07). Owner: project governance / CI.

## Model

Simple branch + integration flow. No overengineering.

```text
main            — integration branch; every commit on main is ACCEPTED-state work
batch/<name>    — short-lived integration batch branch for a phase/batch of WPs
wp/<wp-id>      — (optional) per-Work-Package branches/worktrees for parallel work
```

## Rules

1. **main = integration.** Only validated work lands on `main`; each merge is a normal
   `git merge --no-ff` (never force-push, never `--reset --hard` to look clean).
2. **Work Packages** run on `wp/<phase>-<wp>` branches (or worktrees per Doc 11 H9)
   when parallelism is needed. A WP is merged only when its record is ACCEPTED and its
   gates pass.
3. **Preserve user work.** Never destroy uncommitted or unreferenced work with
   destructive commands to make the tree clean.
4. **Checkpoints** — coherent commits at capability boundaries; stage only intended
   files; never commit secrets or generated garbage (Doc 11 H10–H11).
5. **Review/integration** — every batch merged to `main` passes `make validate`
   (format, lint, typecheck, tests, migration check, architecture check, secret scan,
   fixture smoke) plus CI (ADR-0013).
6. **Phase checkpoints** — at phase completion, the completion package
   (`docs/progress/phase-NN-completion.md` + `.json`) is committed on `main` together
   with the phase's final commit.
7. **Remote**: `origin/main` is the shared integration branch. Push only validated
   states; rebasing shared branches is discouraged (normal merges only).
8. **Worktrees** (Doc 11 H9): use `git worktree add` for parallel WPs; record
   worktree path in the WP record. Never point a worktree at the reference library.

## Integration Checklist (before merging to main)

- [ ] WP record status = ACCEPTED, evidence refs recorded
- [ ] `make validate` passes locally
- [ ] CI green (`.github/workflows/ci.yml`)
- [ ] Diff inspected; no unrelated churn; no generated garbage; no secrets
- [ ] ADRs/registry/progress updated where the change affects them