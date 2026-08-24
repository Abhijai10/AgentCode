pub struct ControlPlaneDb {
    connection: Connection,
}

pub type LspSessionRow = (String, String, String, Option<u32>, String, u8);
pub type SemanticEdgeRow = (String, String, String, String, String, String, u8);
pub type SemanticDiagnosticRow = (String, u32, String, String, u8);
pub type WorkspaceBoundaryRow = (String, String, Option<String>, String);
pub type OptionalIndexDecisionRow = (String, bool, String, usize, usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryFactRow {
    pub id: String,
    pub repository_id: String,
    pub mission_id: Option<String>,
    pub task_id: Option<String>,
    pub branch: Option<String>,
    pub statement: String,
    pub fact_type: String,
    pub source: String,
    pub confidence: u8,
    pub freshness: String,
    pub memory_class: String,
    pub observed_commit: String,
    pub conflict_set_id: Option<String>,
    pub valid_from_ms: i64,
    pub valid_until_ms: Option<i64>,
    pub superseded_by: Option<String>,
    pub last_validation_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryEvidenceRow {
    pub fact_id: String,
    pub evidence_ref: String,
    pub file_path: Option<String>,
    pub symbol: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDecisionRow {
    pub id: String,
    pub repository_id: String,
    pub mission_id: Option<String>,
    pub task_id: Option<String>,
    pub branch: Option<String>,
    pub decision: String,
    pub rationale: String,
    pub authority_refs: String,
    pub supersedes: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskMemoryRow {
    pub id: String,
    pub task_id: String,
    pub summary: String,
    pub evidence_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextSnapshotRow {
    pub id: String,
    pub repository_id: String,
    pub mission_id: Option<String>,
    pub task_id: Option<String>,
    pub branch: Option<String>,
    pub reason: String,
    pub content: String,
    pub source_fact_ids: String,
    pub decision_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextPackManifestRow {
    pub id: String,
    pub pack_id: String,
    pub task_id: String,
    pub role: String,
    pub profile: String,
    pub source_fragment_ids: String,
    pub omitted_fragment_ids: String,
    pub raw_evidence_refs: String,
    pub cache_keys: String,
    pub score_trace: String,
    pub total_input_tokens: u32,
    pub hard_ceiling: u32,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextCompressionReceiptRow {
    pub id: String,
    pub raw_evidence_ref: String,
    pub command_class: String,
    pub compressor_id: String,
    pub raw_hash: String,
    pub compressed_output: String,
    pub raw_token_estimate: u32,
    pub compressed_token_estimate: u32,
    pub omitted_lines: u32,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillRow {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub trust_tier: String,
    pub full_instructions: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookInvocationRow {
    pub id: String,
    pub hook_id: String,
    pub outcome: String,
    pub evidence_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpServerRow {
    pub id: String,
    pub name: String,
    pub health: String,
    pub restart_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityScanRow {
    pub id: String,
    pub repository_id: String,
    pub commit_ref: String,
    pub threat_model_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityFindingRow {
    pub id: String,
    pub root_cause: String,
    pub severity: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSecurityReportRow {
    pub id: String,
    pub repository_id: String,
    pub commit_ref: String,
    pub authorization_id: String,
    pub environment: String,
    pub cleanup_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiSecurityReportRow {
    pub id: String,
    pub repository_id: String,
    pub commit_ref: String,
    pub surfaces: String,
    pub findings_count: u32,
    pub mitigations_verified: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussSessionRow {
    pub id: String,
    pub repository_id: String,
    pub title: String,
    pub state: String,
    pub context_manifest_refs: String,
    pub accepted_decision_refs: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussMessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub context_ref: Option<String>,
    pub evidence_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussDecisionCandidateRow {
    pub id: String,
    pub session_id: String,
    pub decision: String,
    pub rationale: String,
    pub evidence_refs: String,
    pub accepted_decision_ref: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussPlanRow {
    pub id: String,
    pub session_id: String,
    pub requirements: String,
    pub tasks: String,
    pub constraints_json: String,
    pub open_questions: String,
    pub accepted_decision_refs: String,
    pub promoted_mission_id: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignSessionRow {
    pub id: String,
    pub repository_id: String,
    pub product: String,
    pub state: String,
    pub hard_constraints: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignArtifactRow {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub artifact_type: String,
    pub current_version: u32,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignArtifactVersionRow {
    pub id: String,
    pub artifact_id: String,
    pub version: u32,
    pub summary: String,
    pub content_hash: String,
    pub evidence_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignVisualEvaluationRow {
    pub id: String,
    pub artifact_version_id: String,
    pub passed: bool,
    pub findings: String,
    pub responsive_viewports: String,
    pub accessibility_checks: String,
    pub functional_flows: String,
    pub evidence_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopSessionRow {
    pub id: String,
    pub active_project_id: Option<String>,
    pub active_mission_id: Option<String>,
    pub selected_view: String,
    pub window_open: bool,
    pub daemon_connected: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopProjectRow {
    pub id: String,
    pub name: String,
    pub path: String,
    pub repository_id: String,
    pub last_opened_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopPreferenceRow {
    pub session_id: String,
    pub appearance: String,
    pub notifications_enabled: bool,
    pub completion_sound_enabled: bool,
    pub reduced_motion: bool,
    pub budget_limit_micros: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopUiStateRow {
    pub session_id: String,
    pub serialized_state: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopApprovalRecordRow {
    pub id: String,
    pub approval_id: String,
    pub mission_id: String,
    pub approval_kind: String,
    pub decision: String,
    pub explanation: String,
    pub evidence_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenUsageRecordRow {
    pub id: String,
    pub task_id: String,
    pub provider_call_id: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub context_tokens: u32,
    pub compressed_tokens: u32,
    pub estimated_cost_micros: u64,
    pub verified: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceTelemetryRecordRow {
    pub id: String,
    pub component: String,
    pub rss_bytes: u64,
    pub cpu_millis: u64,
    pub disk_bytes: u64,
    pub process_count: u32,
    pub worker_count: u32,
    pub browser_sessions: u32,
    pub lsp_sessions: u32,
    pub local_model_loaded: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizationReportRow {
    pub id: String,
    pub total_tokens: u32,
    pub verified_tokens: u32,
    pub total_cost_micros: u64,
    pub cost_per_verified_task_micros: Option<u64>,
    pub average_compression_ratio: u8,
    pub before_after: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextCacheEntryRow {
    pub cache_key: String,
    pub content_hash: String,
    pub token_estimate: u32,
    pub source_ref: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRetrievalRecordRow {
    pub id: String,
    pub pack_id: String,
    pub need: String,
    pub reason: String,
    pub query: String,
    pub result_fragment_ids: String,
    pub added_tokens: u32,
    pub degraded: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextPackMetricsRow {
    pub id: String,
    pub pack_id: String,
    pub role: String,
    pub selected_fragments: usize,
    pub omitted_fragments: usize,
    pub total_input_tokens: u32,
    pub budget_target: u32,
    pub hard_ceiling: u32,
    pub deduped_fragments: usize,
    pub redacted_fragments: usize,
    pub retrieval_steps: usize,
    pub cache_hits: usize,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBenchmarkResultRow {
    pub id: String,
    pub task_name: String,
    pub broad_tokens: u32,
    pub targeted_tokens: u32,
    pub broad_success: bool,
    pub targeted_success: bool,
    pub retry_delta: i32,
    pub latency_delta_ms: i64,
    pub passed: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionContractRevisionRow {
    pub id: String,
    pub mission_id: String,
    pub revision: u32,
    pub original_goal: String,
    pub reason: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementMatrixEntryRow {
    pub id: String,
    pub mission_id: String,
    pub contract_revision: u32,
    pub description: String,
    pub requirement_type: String,
    pub priority: u8,
    pub source: String,
    pub verification_strategy: String,
    pub blocking: bool,
    pub implementation_status: String,
    pub verification_status: String,
    pub evidence_refs: String,
    pub linked_task_ids: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskLeaseRow {
    pub task_id: String,
    pub worker_id: String,
    pub lease_epoch: u64,
    pub expires_at_ms: i64,
    pub heartbeat_interval_ms: i64,
    pub state: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutonomyMailboxMessageRow {
    pub id: String,
    pub mission_id: String,
    pub sender_worker_id: String,
    pub recipient_worker_id: Option<String>,
    pub message_type: String,
    pub subject_id: Option<String>,
    pub payload: String,
    pub delivered: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutonomyRecordRow {
    pub id: String,
    pub mission_id: String,
    pub category: String,
    pub subject_id: Option<String>,
    pub payload: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditTransactionRow {
    pub id: String,
    pub changeset_id: String,
    pub task_id: Option<String>,
    pub worktree_id: Option<String>,
    pub base_revision: String,
    pub state: String,
    pub formatter: Option<String>,
    pub degraded_reason: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditOperationRow {
    pub id: String,
    pub transaction_id: String,
    pub path: String,
    pub strategy: String,
    pub before_hash: String,
    pub after_hash: String,
    pub symbol_fingerprint: Option<String>,
    pub additions: u32,
    pub removals: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditJournalEntryRow {
    pub id: String,
    pub transaction_id: String,
    pub path: String,
    pub state: String,
    pub before_hash: String,
    pub after_hash: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditStrategyMetricRow {
    pub id: String,
    pub transaction_id: String,
    pub task_id: Option<String>,
    pub model_id: Option<String>,
    pub language: String,
    pub strategy: String,
    pub first_apply_success: bool,
    pub syntax_failures: u32,
    pub retries: u32,
    pub unrelated_diff_files: u32,
    pub verification_rejections: u32,
    pub degraded: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRunRow {
    pub id: String,
    pub profile_id: Option<String>,
    pub task_id: Option<String>,
    pub commit_ref: String,
    pub worktree_id: String,
    pub environment: String,
    pub command: String,
    pub tool_version: String,
    pub normalized_result: String,
    pub raw_artifact: String,
    pub evidence_ref: String,
    pub freshness_dependencies: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementVerificationRow {
    pub id: String,
    pub requirement_id: String,
    pub verification_run_id: String,
    pub evidence_ref: String,
    pub evidence_kind: String,
    pub verified: bool,
    pub freshness_key: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalAuditRow {
    pub id: String,
    pub mission_id: String,
    pub original_goal: String,
    pub requirements: String,
    pub evidence_refs: String,
    pub passed: bool,
    pub return_to_repair: bool,
    pub completion_allowed: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserSessionRow {
    pub id: String,
    pub task_id: String,
    pub process_id: String,
    pub current_url: Option<String>,
    pub profile: String,
    pub storage_state_ref: Option<String>,
    pub sensitive: bool,
    pub stale_evidence_refs: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserScreenshotRow {
    pub id: String,
    pub session_id: String,
    pub task_id: String,
    pub commit_ref: String,
    pub viewport: String,
    pub url: String,
    pub artifact_uri: String,
    pub sensitive: bool,
    pub evidence_ref: String,
    pub captured_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedMission {
    pub id: String,
    pub original_goal: String,
    pub state: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedSession {
    pub id: String,
    pub mission_id: String,
    pub state: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedCheckpoint {
    pub id: String,
    pub session_id: String,
    pub next_step: u32,
    pub state: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedWorktree {
    pub id: String,
    pub repository_id: String,
    pub owner_mission_id: String,
    pub owner_worker_id: String,
    pub lease_epoch: u64,
    pub path: String,
    pub branch: String,
    pub base_commit: String,
    pub current_commit: String,
    pub status: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedChangeSet {
    pub id: String,
    pub state: String,
    pub operations_json: String,
    pub metadata_json: Option<String>,
    pub rollback_json: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedGitCheckpoint {
    pub id: String,
    pub worktree_id: String,
    pub commit_ref: String,
    pub reason: String,
    pub task_attempt_id: Option<String>,
    pub test_summary: Option<String>,
    pub context_ref: Option<String>,
    pub blocker: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingDecisionRecord {
    pub id: String,
    pub task_id: String,
    pub candidates_json: String,
    pub selected_json: Option<String>,
    pub rejected_json: String,
    pub fallback_reason: Option<String>,
    pub latency_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost_micros: u64,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionRecord {
    pub id: String,
    pub tool_call_id: String,
    pub tool_id: String,
    pub status: String,
    pub manifest_json: String,
    pub raw_output: String,
    pub evidence_ref: String,
    pub created_at_ms: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerRecord {
    pub id: String,
    pub mission_id: String,
    pub session_id: String,
    pub state: String,
    pub workspace_ref: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskRecord {
    pub id: String,
    pub mission_id: String,
    pub title: String,
    pub state: String,
    pub dependencies_json: String,
    pub assigned_worker_id: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskAttemptRecord {
    pub id: String,
    pub task_id: String,
    pub worker_id: String,
    pub outcome: String,
    pub evidence_refs: String,
    pub failure_class: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosExperimentRow {
    pub id: String,
    pub gate_id: String,
    pub test_id: String,
    pub mission_id: String,
    pub fault_kind: String,
    pub expected_recovery: String,
    pub seed: u64,
    pub runs: u32,
    pub passes: u32,
    pub final_result: String,
    pub state_equivalent: bool,
    pub unresolved_failures: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosRecoveryEventRow {
    pub id: String,
    pub experiment_id: String,
    pub sequence_no: u32,
    pub phase: String,
    pub observed_behavior: String,
    pub recovery_action: String,
    pub evidence_ref: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChaosReportRow {
    pub id: String,
    pub scope: String,
    pub experiments: u32,
    pub recovered: u32,
    pub recovery_percent: u8,
    pub unresolved_failures: String,
    pub regression_list: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DogfoodMissionRow {
    pub id: String,
    pub repository_id: String,
    pub repository_path: String,
    pub mission_kind: String,
    pub objective: String,
    pub status: String,
    pub change_set_id: Option<String>,
    pub verification_report_id: Option<String>,
    pub evidence_refs: String,
    pub human_interventions: u32,
    pub provider_switches: u32,
    pub worker_replacements: u32,
    pub context_compactions: u32,
    pub verifier_rejections: u32,
    pub token_total: u32,
    pub paid_cost_micros: u64,
    pub wall_time_ms: u64,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DogfoodFindingRow {
    pub id: String,
    pub mission_id: String,
    pub severity: String,
    pub title: String,
    pub evidence_refs: String,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DogfoodProposalRow {
    pub id: String,
    pub mission_id: String,
    pub finding_id: String,
    pub summary: String,
    pub affected_files: String,
    pub change_set_id: String,
    pub decision: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DogfoodReportRow {
    pub id: String,
    pub scope: String,
    pub missions_executed: u32,
    pub findings: u32,
    pub accepted_improvements: u32,
    pub rejected_proposals: u32,
    pub regressions: String,
    pub recommendations: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyAuditRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    pub source: String,
    pub checksum: String,
    pub security_status: String,
    pub vulnerability_refs: String,
    pub release_blocking: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardeningReportRow {
    pub id: String,
    pub report_type: String,
    pub findings: String,
    pub mitigations: String,
    pub unresolved_risks: String,
    pub accepted_limitations: String,
    pub release_blocked: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseArtifactRow {
    pub id: String,
    pub version: String,
    pub platform: String,
    pub artifact_kind: String,
    pub build_hash: String,
    pub integrity_hash: String,
    pub source_commit: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseBuildRow {
    pub id: String,
    pub version: String,
    pub commit_ref: String,
    pub build_profile: String,
    pub environment: String,
    pub reproducible: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateRecordRow {
    pub id: String,
    pub current_version: String,
    pub available_version: String,
    pub decision: String,
    pub verified: bool,
    pub rollback_ref: Option<String>,
    pub recovery_action: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseCandidateRow {
    pub id: String,
    pub version: String,
    pub candidate_id: String,
    pub build_id: String,
    pub commit_hash: String,
    pub platform_target: String,
    pub validation_status: String,
    pub evidence_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseValidationRunRow {
    pub id: String,
    pub candidate_id: String,
    pub security_status: String,
    pub tests_status: String,
    pub migration_status: String,
    pub artifact_status: String,
    pub performance_status: String,
    pub release_approval_status: String,
    pub evidence_refs: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseApprovalDecisionRow {
    pub id: String,
    pub approved_version: String,
    pub validation_evidence_refs: String,
    pub security_status: String,
    pub approval_timestamp_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalReleaseManifestRow {
    pub id: String,
    pub version: String,
    pub features: String,
    pub migrations: String,
    pub artifacts: String,
    pub checksums: String,
    pub known_limitations: String,
    pub manifest_hash: String,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseEvidenceBundleRow {
    pub id: String,
    pub version: String,
    pub audit_report_ref: String,
    pub security_report_ref: String,
    pub validation_report_ref: String,
    pub artifact_report_ref: String,
    pub migration_report_ref: String,
    pub created_at_ms: i64,
}
