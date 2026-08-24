CREATE TABLE IF NOT EXISTS skills (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  version TEXT NOT NULL,
  source TEXT NOT NULL,
  scope TEXT NOT NULL,
  trust_tier TEXT NOT NULL,
  trigger_hints TEXT NOT NULL,
  required_capabilities TEXT NOT NULL,
  context_cost INTEGER NOT NULL,
  project_id TEXT,
  task_id TEXT,
  full_instructions TEXT NOT NULL,
  loaded_at_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_skills_scope ON skills(scope, project_id, task_id);

CREATE TABLE IF NOT EXISTS hook_manifests (
  id TEXT PRIMARY KEY,
  extension_id TEXT NOT NULL,
  event TEXT NOT NULL,
  priority INTEGER NOT NULL,
  timeout_ms INTEGER NOT NULL,
  idempotency_key TEXT NOT NULL,
  failure_policy TEXT NOT NULL,
  required_capabilities TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS hook_invocations (
  id TEXT PRIMARY KEY,
  hook_id TEXT NOT NULL,
  event TEXT NOT NULL,
  outcome TEXT NOT NULL,
  evidence_ref TEXT,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_hook_invocations_hook ON hook_invocations(hook_id);

CREATE TABLE IF NOT EXISTS mcp_servers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  version TEXT NOT NULL,
  transport TEXT NOT NULL,
  trust_tier TEXT NOT NULL,
  health TEXT NOT NULL,
  restart_count INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS mcp_tools (
  id TEXT PRIMARY KEY,
  server_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  schema_json TEXT NOT NULL,
  risk TEXT NOT NULL,
  required_capabilities TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mcp_tools_server ON mcp_tools(server_id);

CREATE TABLE IF NOT EXISTS mcp_invocations (
  id TEXT PRIMARY KEY,
  server_id TEXT NOT NULL,
  tool_id TEXT NOT NULL,
  status TEXT NOT NULL,
  output TEXT NOT NULL,
  evidence_ref TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);
