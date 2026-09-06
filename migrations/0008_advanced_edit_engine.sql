CREATE TABLE IF NOT EXISTS edit_transactions (
    id TEXT PRIMARY KEY,
    changeset_id TEXT NOT NULL,
    task_id TEXT,
    worktree_id TEXT,
    base_revision TEXT NOT NULL,
    state TEXT NOT NULL,
    formatter TEXT,
    degraded_reason TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_edit_transactions_state
    ON edit_transactions(state, updated_at_ms);

CREATE TABLE IF NOT EXISTS edit_operations (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    path TEXT NOT NULL,
    strategy TEXT NOT NULL,
    before_hash TEXT NOT NULL,
    after_hash TEXT NOT NULL,
    symbol_fingerprint TEXT,
    additions INTEGER NOT NULL,
    removals INTEGER NOT NULL,
    FOREIGN KEY(transaction_id) REFERENCES edit_transactions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_edit_operations_transaction
    ON edit_operations(transaction_id, path);

CREATE TABLE IF NOT EXISTS edit_journal_entries (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    path TEXT NOT NULL,
    state TEXT NOT NULL,
    before_hash TEXT NOT NULL,
    after_hash TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(transaction_id) REFERENCES edit_transactions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_edit_journal_recovery
    ON edit_journal_entries(transaction_id, state, path);

CREATE TABLE IF NOT EXISTS edit_strategy_metrics (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL,
    task_id TEXT,
    model_id TEXT,
    language TEXT NOT NULL,
    strategy TEXT NOT NULL,
    first_apply_success INTEGER NOT NULL,
    syntax_failures INTEGER NOT NULL,
    retries INTEGER NOT NULL,
    unrelated_diff_files INTEGER NOT NULL,
    verification_rejections INTEGER NOT NULL,
    degraded INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(transaction_id) REFERENCES edit_transactions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_edit_strategy_metrics_strategy
    ON edit_strategy_metrics(strategy, language, created_at_ms);
