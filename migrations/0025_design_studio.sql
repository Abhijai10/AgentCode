-- Design Studio: structured design documents, preview lifecycle, critique/repair
-- tracking, all keyed by conversation_id for project-scoped design conversations.
-- Browser screenshots and dev server records continue to use the existing
-- browser_dev_servers and browser_screenshots tables from migration 0010.

CREATE TABLE IF NOT EXISTS design_documents (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  doc_type TEXT NOT NULL,
  content_json TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  evidence_refs TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_design_documents_conv_type
  ON design_documents(conversation_id, doc_type);

CREATE TABLE IF NOT EXISTS design_previews (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  port INTEGER,
  ready_url TEXT,
  process_id TEXT,
  process_alive INTEGER NOT NULL DEFAULT 0,
  http_ready INTEGER NOT NULL DEFAULT 0,
  browser_session_id TEXT,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_design_previews_conv
  ON design_previews(conversation_id);

CREATE TABLE IF NOT EXISTS design_critiques (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  doc_type TEXT,
  passed INTEGER NOT NULL,
  findings_json TEXT NOT NULL,
  improvement_required INTEGER NOT NULL DEFAULT 0,
  evidence_refs TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_design_critiques_conv
  ON design_critiques(conversation_id);