ALTER TABLE worktrees ADD COLUMN lease_epoch INTEGER NOT NULL DEFAULT 1;
ALTER TABLE worktree_checkpoints ADD COLUMN task_attempt_id TEXT;
ALTER TABLE worktree_checkpoints ADD COLUMN test_summary TEXT;
ALTER TABLE worktree_checkpoints ADD COLUMN context_ref TEXT;
ALTER TABLE worktree_checkpoints ADD COLUMN blocker TEXT;
