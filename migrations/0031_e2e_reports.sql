-- E2E testing mode (Codex-Playwright parity): AI-driven end-to-end runs
-- over the project's live app, with bug reports handed to missions.
CREATE TABLE IF NOT EXISTS e2e_reports (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    base_url TEXT NOT NULL,
    bug_count INTEGER NOT NULL DEFAULT 0,
    report_json TEXT NOT NULL DEFAULT '{}',
    mission_ref TEXT,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_e2e_reports_conversation
    ON e2e_reports(conversation_id, created_at_ms DESC);
