# EXTRACTION-0604 — Opencode Git & Snapshot Services

**Campaign:** F — Git & Worktrees (Doc 07 §126)
**Donor:** opencode (MIT)
**Pinned SHA:** `9b0dd36cda0b9accb429a7f9f9ad9b054a27d04a`
**License:** MIT (LICENSE at repo root)
**Classification (overall):** `ADAPT HEAVILY` (Doc 07 §31) — git client, event model, service boundary
**Extraction date:** 20 August 2026

---

## 1. Source Layout

```
packages/opencode/src/
  git/index.ts        read-only git service + applyPatch (Effect-based)
  snapshot/index.ts   git-backed snapshot / checkpoint / revert store
  session/processor.ts  pre/post-turn snapshot wiring
  session/revert.ts     snapshot restore + per-message revert
```

## 2. Key Symbols (file:line)

### 2.1 Git service (read-only + apply)
- Config flags for every invocation — `packages/opencode/src/git/index.ts:6-18`:
  `--no-optional-locks -c core.autocrlf=false -c core.fsmonitor=false -c core.longpaths=true -c core.symlinks=true -c core.quotepath=false`.
- Interface — `index.ts:75-91`: `run`, `branch` (:164, `symbolic-ref --quiet --short HEAD`), `prefix` (:171), `defaultBranch` (:177 — remote HEAD symref → `init.defaultBranch` config → main/master fallback), `hasHead` (:195), `mergeBase` (:200), `show` (:207, `git show ref:file`), `status` (:215, `status --porcelain=v1 --untracked-files=all --no-renames -z -- .`), `diff` (:228, `--no-ext-diff --no-renames --name-status -z`), `stats` (:240, `--numstat -z`), `patch` (:263, `diff --patch --unified=N`), `patchAll` (:271), `patchUntracked` (:279, `diff --no-index ... /dev/null`), `statUntracked` (:301), `applyPatch` (:322, `git apply -` from stdin).
- Truncation: patch output capped at `maxOutputBytes` (`index.ts:68-70`).
- No branch create, commit, checkout, or reset — the service is strictly read + apply.

### 2.2 Snapshot / checkpoint store
- Layout — `packages/opencode/src/snapshot/index.ts:66-73`: separate git dir per project per worktree at `Global.Path.data/snapshot/<projectId>/<hash(worktree)>`; every command uses `--git-dir <dir> --work-tree <worktree>`.
- `track` — `index.ts:318-347`: ensure git dir initialized once with tuned config (`autocrlf/longpaths/symlinks/fsmonitor=false`, `feature.manyFiles`, `index.version=4`, `index.threads`, `untrackedCache`, `index.ts:328-336`); `seed` (:198) adds the source repo as object alternates + copies the source index (big-repo speed); `add` (:235-298): `diff-files` + `ls-files --others`, `check-ignore` for excludes, drop newly-ignored files from the index, block files >2MB (limit `2MB` at `index.ts:24`); stage via `add --all --sparse --pathspec-from-file=-`; then `write-tree` → tree hash is the snapshot id (:341).
- `restore` — `index.ts:382-406`: `git read-tree <snapshot>` + `git checkout-index -a -f` (overwrites the working tree from the snapshot).
- `revert` — `index.ts:408-524`: per-file `git checkout <hash> -- <file>` with batching and clash detection; files absent in the snapshot are deleted.
- `diff` / `diffFull` — `index.ts:526` / `:546`, using `cat-file --batch` for bulk object reads.
- `cleanup` — `index.ts:300-316`: `git gc --prune=7.days` (hourly schedule).

### 2.3 Session wiring
- `session/processor.ts:102` — `initialSnapshot = yield* snapshot.track()` captured BEFORE the LLM stream (pre-turn anchor).
- `session/processor.ts:436` — completed snapshot captured after the turn (post-turn anchor).
- `session/revert.ts:38-99` — revert: restore snapshot + apply per-message reverts; unrevert: restore snapshot. Recovery anchor = the tree-hash snapshot, not the message diffs.

## 3. Control Flow

Per message: pre-turn snapshot (write-tree hash) → LLM stream → post-turn snapshot. On revert: `read-tree` + `checkout-index -a -f` to the pre-turn snapshot, then per-file `git checkout <hash> -- <file>` for each reverted message. Snapshots are cheap tree hashes with object alternates seeding; storage pruned to 7 days / 2MB/file.

## 4. Lifecycle / State

- Snapshot store keyed by project+worktree; lifecycle = session lifetime + 7-day gc window.
- Git service is stateless; snapshot store owns durable checkpoint state on disk (separate git dir, never touches the source repo's git state — source repo only contributes object alternates).

## 5. Failure Behavior

- Patch apply failure → structured result; no partial-write surprises (apply via stdin with strict config).
- Snapshot block on >2MB files — large binary files excluded from checkpoints (documented behavior, `index.ts:24`).
- Restore is best-effort per-file with batching; failures surface per file.

## 6. Upstream Tests

- `packages/opencode/src/git/index.test.ts` — status/diff/patch/apply/truncation.
- `packages/opencode/src/snapshot/index.test.ts` — track/restore/revert, gitignore exclusions, size limits, alternates seeding.
- `packages/opencode/src/session/processor.test.ts` — snapshot lifecycle around LLM stream.

## 7. Platform Assumptions

- Node/TypeScript + system `git`; data dir under the app's `Global.Path.data`.
- Cross-platform git config handled explicitly (autocrlf=false, symlinks, longpaths) — needed on Windows/CI.

## 8. Licensing Implications

- MIT. Attribution notice required if source is copied; conceptual adaptation carries no notice obligation.

## 9. Classification

| Mechanism | Class | Reason |
|---|---|---|
| Read-only git service (status/diff/stat/patch/show/merge-base) with hardened config | ADAPT | AgentCode Git Engine read path (Doc 04 §33) |
| `applyPatch` via stdin with strict config | TAKE (concept) | Consistent with Codex `git apply` pattern (EXTRACTION-0601) |
| Separate `--git-dir` snapshot store + `--work-tree` pointing at real worktree | TAKE (concept) | AgentCode checkpoint store must not pollute the task worktree's `.git`; matches Doc 04 §37 checkpoints |
| Tree-hash snapshots (`write-tree`) + object alternates seeding | ADAPT | Cheap, content-addressed checkpoint IDs; fast seeding for big repos |
| Per-file revert with clash detection/batching + delete-if-absent | ADAPT | ChangeSet-scoped rollback mechanics (Doc 04 §30) |
| Config hardening (`quotepath=false`, `autocrlf=false`, `fsmonitor=false`, `--no-optional-locks`) | TAKE | Deterministic git behavior across platforms |
| `restore` overwrite (`read-tree` + `checkout-index -a -f`) | ADAPT (guarded) | Destructive overwrite must be gated by dirty/ownership checks and policy (Doc 04 §38) |
| Worktree creation | IGNORE | opencode does not create worktrees; lifecycle comes from OpenHands (EXTRACTION-0602) |

## 10. AgentCode Destination / Owner

- **Destination:** Git Engine read service + Checkpoint store (Doc 04 §30 ChangeSet, §37 checkpoints; Doc 11 H9 worktrees).
- **Owner boundary:** Kernel owns checkpoint timing; Git Engine owns snapshot mechanics.

## 11. MUST-NOT-INHERIT

1. **Unconditional `checkout-index -a -f` overwrite** (`snapshot/index.ts:382-406`) as a free-standing restore primitive. AgentCode restores must run only inside an owned ChangeSet/checkpoint flow with dirty-state and fencing checks (Doc 04 §38, D04-G003/G010). The snapshot store's git dir is disposable; the worktree it writes into is not.
2. **Delete-if-absent per-file revert** — deleting files that do not exist in the snapshot is safe only when the snapshot is known to be the full prior state; must not be used to clean "leftover" files outside the ChangeSet (AGENTS.md §8 preserves user work).

## 12. Prototype Required

- Checkpoint-store prototype: snapshot (write-tree) + guarded restore + per-file revert against the worktree overlay prototype (Doc 11 §30) — proves the recovery anchor and the 7-day/size-limit lifecycle.

## 13. Evidence

- Pinned SHA verified at clone: `9b0dd36cda0b9accb429a7f9f9ad9b054a27d04a` (`git rev-parse HEAD`, 20 Aug 2026).
- Doc 07 campaign context: §31 (opencode role), §126 (Campaign F).