-- AgentCode conversations: project-bound chat containers shared by all
-- conversation modes (GOAL, DISCUSS, DESIGN, SECURITY).  Conversations are the
-- user-facing model that wraps missions; missions remain the authoritative
-- execution object.  This extends the single SQLite control plane — there is
-- no second database.

CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  project_path TEXT NOT NULL,
  mode TEXT NOT NULL DEFAULT 'GOAL',
  title TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'active',
  current_mission_id TEXT,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_conversations_project ON conversations(project_path, state, updated_at_ms);

CREATE TABLE IF NOT EXISTS conversation_messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  mission_ref TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_conversation_messages_conversation ON conversation_messages(conversation_id, created_at_ms);

-- Attachment metadata.  File bytes are persisted by the daemon into its own
-- runtime attachments directory (never localStorage, never arbitrary UI-side
-- paths); the DB row stores only safe metadata plus a daemon-relative storage
-- key.  storage_key is opaque to the UI and is never used as a path.
CREATE TABLE IF NOT EXISTS attachments (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  message_id TEXT,
  project_path TEXT NOT NULL,
  filename TEXT NOT NULL,
  mime_type TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  sha256 TEXT NOT NULL,
  storage_key TEXT NOT NULL,
  sensitivity TEXT NOT NULL DEFAULT 'normal',
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_attachments_conversation ON attachments(conversation_id);
CREATE INDEX IF NOT EXISTS idx_attachments_project ON attachments(project_path);
