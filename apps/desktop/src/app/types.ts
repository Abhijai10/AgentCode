export type View = "home" | "mission" | "discuss" | "design" | "security" | "settings";

export interface MissionSummary {
  mission_id: string;
  session_id: string;
  state: string;
  goal?: string;
  task_count: number;
}

export interface TaskState {
  task_id: string;
  state: string;
  title?: string;
}

export interface DaemonHealth {
  lifecycle: string;
  recovered_sessions: number;
}

export interface ProviderInfo {
  id: string;
  name: string;
  description: string;
  category: "free" | "free-tier" | "paid" | "local";
  health: "healthy" | "degraded" | "rate_limited" | "auth_failed" | "unavailable" | "disabled";
  connected_accounts: number;
  model_count: number;
  website?: string;
  credential_url?: string;
  local?: boolean;
  paid?: boolean;
  accounts: ProviderAccount[];
}

export interface ProviderAccount {
  id: string;
  provider_id?: string;
  label: string;
  credential_masked: string;
  organization?: string;
  project?: string;
  workspace?: string;
  health: "healthy" | "degraded" | "auth_failed" | "rate_limited" | "unknown";
  quota_state?: string;
  quota_remaining?: number;
  quota_reset_at_ms?: number;
  last_connected?: string | number;
  last_failure?: string | number;
  enabled: boolean;
  paid: boolean;
  local: boolean;
}

export interface LocalModel {
  name: string;
  size?: string;
  capabilities: string[];
  loaded: boolean;
  discovered_at: string;
}

export interface OllamaStatus {
  running: boolean;
  models: LocalModel[];
  error?: string;
}

export interface DiscussSession {
  id: string;
  title: string;
  state: string;
  created_at: string;
}

export interface DiscussMessage {
  id: string;
  role: "user" | "assistant" | "researcher";
  content: string;
  context_ref?: string;
  sources: { path: string; snippet: string }[];
  created_at: string;
}

export interface DesignSession {
  id: string;
  product: string;
  state: string;
  artifacts: DesignArtifact[];
}

export interface DesignArtifact {
  id: string;
  name: string;
  artifact_type: string;
  current_version: number;
  versions: { version: number; summary: string }[];
}

export interface SettingsState {
  appearance: "light" | "dark" | "system";
  notifications_enabled: boolean;
  completion_sound: boolean;
  reduced_motion: boolean;
  routing_profile: "free_first" | "local_first" | "quality_first" | "paid_allowed" | "offline";
  preferred_model?: string;
  budget_limit_micros?: number;
}

export interface DaemonStatus {
  state: "running" | "disconnected" | "recovering";
  instance_id: string;
  uptime: string;
  socket: string;
  database: string;
  recovered_sessions: number;
}

export interface ActivityItem {
  id: string;
  category: "analysis" | "implementation" | "testing" | "verification" | "recovery";
  summary: string;
  detail?: string;
  files_changed?: string[];
  tests?: { passed: number; total: number };
  errors?: string[];
  timestamp: string;
  expandable?: boolean;
  raw_events?: string[];
}

export interface MissionProgress {
  mission_id: string;
  goal: string;
  state: string;
  progress_pct: number;
  active_task?: string;
  waiting_reason?: string;
  activity: ActivityItem[];
  files_read?: number;
  files_changed?: number;
  errors?: number;
  approvals_required?: number;
}

export interface ToolInfo {
  id: string;
  name: string;
  state: "available" | "unavailable" | "running";
}

export interface ScannerInfo {
  id: string;
  name: string;
  state: "available" | "unavailable" | "running" | "passed" | "findings";
  required: boolean;
}

export interface MemoryInfo {
  id: string;
  statement: string;
  label: string;
  freshness: string;
  source: string;
  date: string;
}

export interface SecurityFinding {
  id: string;
  scanner: string;
  severity: "info" | "low" | "medium" | "high";
  title: string;
  file?: string;
  line?: string;
  description: string;
  remediation?: string;
}

export interface ChangeSet {
  mission_id?: string;
  available?: boolean;
  files?: { path: string; additions: number; deletions: number }[];
  summary?: string[];
  safe?: boolean;
  verification_state: string;
}