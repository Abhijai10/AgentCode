export type View = "home" | "mission" | "chat" | "discuss" | "design" | "security" | "settings";

export type ConversationMode = "GOAL" | "DISCUSS" | "DESIGN" | "SECURITY";

export interface Conversation {
  id: string;
  project_path: string;
  mode: ConversationMode;
  title: string;
  state: "active" | "archived" | "deleted";
  current_mission_id?: string | null;
  created_at_ms: number;
  updated_at_ms: number;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: "user" | "assistant" | "system" | "researcher";
  content: string;
  mission_ref?: string | null;
  metadata?: string;
  created_at_ms: number;
}

export interface Attachment {
  id: string;
  conversation_id: string;
  message_id?: string | null;
  project_path: string;
  filename: string;
  mime_type: string;
  size_bytes: number;
  sha256: string;
  sensitivity: string;
  created_at_ms: number;
}

export interface ConversationDetail extends Conversation {
  messages: Message[];
  attachments: Attachment[];
}

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
  /** Always null in this build: the daemon IPC does not expose routing profile. */
  routing_profile: "free_first" | "local_first" | "quality_first" | "paid_allowed" | "offline" | null;
  /** Always null in this build: the daemon IPC does not expose preferred model. */
  preferred_model?: string | null;
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

// ── Real backend observability contract (ac-daemon H1 IPC) ──────────────

export interface MissionDetails {
  mission_id: string;
  session_id?: string | null;
  state: string;
  goal?: string;
  workspace_root?: string | null;
  created_at_ms?: number;
  updated_at_ms?: number;
  terminal: boolean;
  failure_code?: string | null;
  task_count: number;
  completed_task_count: number;
  failed_task_count: number;
  current_task?: string | null;
  progress?: number | null;
  state_counts: Record<string, number>;
  completion?: {
    passed: boolean;
    completion_allowed: boolean;
    created_at_ms: number;
  } | null;
}

export interface TaskAttempt {
  attempt_id: string;
  outcome: string;
  failure_class?: string | null;
  evidence_refs: string;
  created_at_ms: number;
}

export interface TaskDetail {
  task_id: string;
  title: string;
  state: string;
  dependencies: string[];
  retry_count: number;
  max_retries: number;
  assigned_worker_id?: string | null;
  updated_at_ms: number;
  attempts: TaskAttempt[];
  current_attempt_outcome?: string | null;
  current_attempt_age_ms?: number | null;
}

export interface MissionActivityEvent {
  id: string;
  kind: string;
  subject_id: string;
  created_at_ms: number;
  detail: string;
}

export interface ChangeSetFile {
  path: string;
  strategy: string;
  additions: number;
  removals: number;
}

export interface ChangeSetSummary {
  changeset_id: string;
  state: string;
  created_at_ms: number;
  applied: boolean;
  files: ChangeSetFile[];
}

export interface EvidenceSummaryItem {
  evidence_id: string;
  kind: string;
  created_at_ms: number;
  content_hash: string;
  sensitive: boolean;
  summary?: string | null;
  provenance_source: string;
  provenance_tool?: string | null;
}

export interface VerificationRun {
  verification_id: string;
  task_id?: string | null;
  commit_ref: string;
  environment: string;
  command: string;
  tool_version: string;
  status: string;
  passed: boolean;
  evidence_ref: string;
  created_at_ms: number;
}

export interface FinalAuditInfo {
  audit_id: string;
  passed: boolean;
  completion_allowed: boolean;
  remaining_uncertainty?: string;
  created_at_ms: number;
}

export interface VerificationSummary {
  verifications: VerificationRun[];
  final_audits: FinalAuditInfo[];
}

// ── Conversation activity projection (G2) ──────────────────────────────
// All data originates from authoritative backend state projected per mission
// referenced by the conversation's messages.  Nothing here is fabricated.

export interface ConversationActivityMission {
  mission_id: string;
  conversation_id: string;
  /** Live coordinator status (queued/running/paused/cancelled/failed/completed). */
  status?: string | null;
  details: MissionDetails;
  tasks: { tasks: TaskDetail[] };
  events: { events: MissionActivityEvent[] };
  changesets: { changesets: ChangeSetSummary[] };
  evidence: { evidence: EvidenceSummaryItem[] };
  verification: VerificationSummary;
  /** Factual execution summary derived from authoritative persisted state. */
  summary?: {
    tools_used: string[];
    commands: string[];
    files_changed: string[];
    failure_classes: string[];
    failed_task_count: number;
    retry_count: number;
  };
  /** Durable provider/model routing records for this mission. */
  provider_models?: ProviderModelRecord[];
  /** Remaining uncertainty persisted by the final audit (empty when none). */
  remaining_uncertainty?: string;
}

export interface ConversationActivity {
  conversation_id: string;
  project_path: string;
  missions: ConversationActivityMission[];
}

// ── Durable provider/model records & remaining uncertainty (G2 fixes) ─────
// Read from authoritative backend state, never fabricated by the UI.

export interface ProviderModelRecord {
  provider_id: string;
  provider_account_id?: string | null;
  model_id: string;
  model_name: string;
  routing_mode: string;
  attempt_number: number;
  success: boolean;
  failure_class?: string | null;
  created_at_ms: number;
}

// ── Design Studio (G4) ───────────────────────────────────────────────────
// Real Design Studio contract from the daemon IPC. Design conversations are
// DESIGN-mode conversations in the existing conversations table; the structured
// documents (analysis/brief/grammar/state), preview lifecycle, critique and
// browser evidence are exposed by Design* IPC commands below.

export interface ProductAnalysis {
  framework?: string | null;
  routes: string[];
  components: string[];
  style_files: string[];
  tokens: string[];
  fonts: string[];
  assets: string[];
  navigation: string[];
}

export interface DesignBrief {
  product: string;
  audience: string;
  personality: string;
  density: string;
  primary_workflow: string;
  framework?: string;
  routes?: number;
  components?: number;
  visual_goals: string[];
  patterns_to_avoid: string[];
  things_to_preserve?: string[];
  things_to_avoid?: string[];
}

export interface DesignGrammar {
  type_scale: string[];
  spacing: string[];
  radii: string[];
  surfaces: string[];
  color_roles: string[];
  semantic_states?: string[];
  navigation: string[];
  motion: string[];
  iconography: string[];
  component_principles: string[];
  density?: string;
  interaction?: string[];
  responsive_principles?: string[];
}

export interface DesignState {
  facts: string[];
  grammar_principles: string[];
  content: string;
}

export interface AntiSlopFinding {
  rule: string;
  severity: number;
  explanation: string;
}

export interface DesignCritique {
  passed: boolean;
  improvement_required: boolean;
  findings: AntiSlopFinding[];
}

export interface DesignRepair {
  passed: boolean;
  repairs: { issue: string; repair: string }[];
  improvement_required: boolean;
}

export interface DesignPreview {
  status: string;
  port?: number | null;
  ready_url?: string | null;
  pid?: number | null;
  command?: string;
  detail?: string;
}

export interface DesignPreviewStatus {
  process_alive: boolean;
  http_ready: boolean;
  port: number;
  ready_url: string;
  status?: string;
}

export interface DesignBrowserResult {
  url: string;
  visible_text: string;
  controls: string[];
  accessibility_tree: string[];
  diagnostics: {
    console_errors: string[];
    page_errors: string[];
    network_failures: string[];
    http_status: number;
  };
  screenshot_uri: string;
  screenshot_evidence_ref: string;
}

export interface DesignQaReport {
  passed: boolean;
  issues: string[];
  [key: string]: unknown;
}