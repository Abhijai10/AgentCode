CREATE TABLE IF NOT EXISTS desktop_sessions (
  id TEXT PRIMARY KEY,
  active_project_id TEXT,
  active_mission_id TEXT,
  selected_view TEXT NOT NULL,
  window_open INTEGER NOT NULL,
  daemon_connected INTEGER NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS desktop_projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  repository_id TEXT NOT NULL,
  last_opened_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS desktop_preferences (
  session_id TEXT PRIMARY KEY,
  appearance TEXT NOT NULL,
  notifications_enabled INTEGER NOT NULL,
  completion_sound_enabled INTEGER NOT NULL,
  reduced_motion INTEGER NOT NULL,
  budget_limit_micros INTEGER
);

CREATE TABLE IF NOT EXISTS desktop_ui_state (
  session_id TEXT PRIMARY KEY,
  serialized_state TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS desktop_approval_records (
  id TEXT PRIMARY KEY,
  approval_id TEXT NOT NULL,
  mission_id TEXT NOT NULL,
  approval_kind TEXT NOT NULL,
  decision TEXT NOT NULL,
  explanation TEXT NOT NULL,
  evidence_refs TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_desktop_approvals_mission ON desktop_approval_records(mission_id);

CREATE TABLE IF NOT EXISTS token_usage_records (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  provider_call_id TEXT,
  input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL,
  context_tokens INTEGER NOT NULL,
  compressed_tokens INTEGER NOT NULL,
  estimated_cost_micros INTEGER NOT NULL,
  verified INTEGER NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_token_usage_task ON token_usage_records(task_id);

CREATE TABLE IF NOT EXISTS resource_telemetry_records (
  id TEXT PRIMARY KEY,
  component TEXT NOT NULL,
  rss_bytes INTEGER NOT NULL,
  cpu_millis INTEGER NOT NULL,
  disk_bytes INTEGER NOT NULL,
  process_count INTEGER NOT NULL,
  worker_count INTEGER NOT NULL,
  browser_sessions INTEGER NOT NULL,
  lsp_sessions INTEGER NOT NULL,
  local_model_loaded INTEGER NOT NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_resource_telemetry_component ON resource_telemetry_records(component);

CREATE TABLE IF NOT EXISTS optimization_reports (
  id TEXT PRIMARY KEY,
  total_tokens INTEGER NOT NULL,
  verified_tokens INTEGER NOT NULL,
  total_cost_micros INTEGER NOT NULL,
  cost_per_verified_task_micros INTEGER,
  average_compression_ratio INTEGER NOT NULL,
  before_after TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL
);
