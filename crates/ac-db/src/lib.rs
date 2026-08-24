use std::path::Path;

use ac_changeset::{ChangeSet, ChangeSetTransaction};
use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::EvidenceRecord;
use ac_git::{CheckpointRecord, WorktreeRecord};
use ac_kernel::{KernelDecisionKind, KernelEvent, Mission, MissionState};
use ac_security::{
    HookInvocation, HookManifest, McpInvocationRecord, McpServerRecord, McpToolRecord,
    SecurityReportBundle, SecurityScanInput, SecurityScanReport, SkillManifest,
};
use ac_verification::{
    BrowserProcessRecord, BrowserSessionRecord, DevServerRecord, FinalAuditReport,
    RequirementEvidenceLink, ScreenshotEvidence, VerificationEvidenceManifest, VerificationProfile,
    VisualQaReport,
};
use rusqlite::{params, Connection, OptionalExtension};

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

impl ControlPlaneDb {
    pub fn open(path: impl AsRef<Path>) -> AcResult<Self> {
        let connection = Connection::open(path).map_err(db_error)?;
        let db = Self { connection };
        db.configure()?;
        Ok(db)
    }

    pub fn open_memory() -> AcResult<Self> {
        let connection = Connection::open_in_memory().map_err(db_error)?;
        let db = Self { connection };
        db.configure()?;
        Ok(db)
    }

    pub fn migrate(&mut self) -> AcResult<()> {
        let current_version = self.user_version()?;
        if current_version > 12 {
            return Err(AcError::conflict(
                "DB-FUTURE_VERSION",
                format!(
                    "database user_version {current_version} is newer than supported version 12"
                ),
            ));
        }
        let tx = self.connection.transaction().map_err(db_error)?;
        tx.execute_batch(include_str!("../../../migrations/0001_kernel_schema.sql"))
            .map_err(db_error)?;
        if current_version < 2 {
            tx.execute_batch(include_str!(
                "../../../migrations/0002_git_worktree_hardening.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 3 {
            tx.execute_batch(include_str!(
                "../../../migrations/0003_code_intelligence.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 4 {
            tx.execute_batch(include_str!(
                "../../../migrations/0004_semantic_repository_graph.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 5 {
            tx.execute_batch(include_str!(
                "../../../migrations/0005_persistent_memory.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 6 {
            tx.execute_batch(include_str!("../../../migrations/0006_context_engine.sql"))
                .map_err(db_error)?;
        }
        if current_version < 7 {
            tx.execute_batch(include_str!(
                "../../../migrations/0007_full_autonomy_kernel.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 8 {
            tx.execute_batch(include_str!(
                "../../../migrations/0008_advanced_edit_engine.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 9 {
            tx.execute_batch(include_str!(
                "../../../migrations/0009_verification_evidence_engine.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 10 {
            tx.execute_batch(include_str!("../../../migrations/0010_browser_runtime.sql"))
                .map_err(db_error)?;
        }
        if current_version < 11 {
            tx.execute_batch(include_str!(
                "../../../migrations/0011_extensions_skills_hooks_mcp.sql"
            ))
            .map_err(db_error)?;
        }
        if current_version < 12 {
            tx.execute_batch(include_str!(
                "../../../migrations/0012_baseline_security.sql"
            ))
            .map_err(db_error)?;
        }
        tx.pragma_update(None, "user_version", 12)
            .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(())
    }

    pub fn user_version(&self) -> AcResult<u32> {
        self.connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(db_error)
    }

    pub fn save_code_index(
        &self,
        repository_id: &str,
        root: &str,
        commit: &str,
        files: &[(String, String, String)],
        symbols: &[(String, String, String, u32)],
        imports: &[(String, String, u32)],
    ) -> AcResult<()> {
        self.connection.execute("INSERT INTO code_repositories (id, root, commit_ref, indexed_at_ms) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET root=excluded.root, commit_ref=excluded.commit_ref, indexed_at_ms=excluded.indexed_at_ms", params![repository_id, root, commit, millis(TimestampMillis::now())]).map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM code_files WHERE repository_id=?1",
                params![repository_id],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM code_symbols WHERE repository_id=?1",
                params![repository_id],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM code_imports WHERE repository_id=?1",
                params![repository_id],
            )
            .map_err(db_error)?;
        for (path, hash, language) in files {
            self.connection
                .execute(
                    "INSERT INTO code_files VALUES (?1, ?2, ?3, ?4, '{}')",
                    params![repository_id, path, hash, language],
                )
                .map_err(db_error)?;
        }
        for (path, name, kind, line) in symbols {
            self.connection
                .execute(
                    "INSERT INTO code_symbols VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![repository_id, path, name, kind, line],
                )
                .map_err(db_error)?;
        }
        for (path, target, line) in imports {
            self.connection
                .execute(
                    "INSERT INTO code_imports VALUES (?1, ?2, ?3, ?4)",
                    params![repository_id, path, target, line],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn code_symbol_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM code_symbols WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_semantic_index(
        &self,
        repository_id: &str,
        commit: &str,
        worktree_id: &str,
        lsp_sessions: &[LspSessionRow],
        edges: &[SemanticEdgeRow],
        diagnostics: &[SemanticDiagnosticRow],
        workspace_boundaries: &[WorkspaceBoundaryRow],
        optional_indexes: &[OptionalIndexDecisionRow],
    ) -> AcResult<()> {
        for table in [
            "lsp_server_sessions",
            "semantic_edges",
            "semantic_diagnostics",
            "workspace_boundaries",
            "optional_index_decisions",
        ] {
            self.connection
                .execute(
                    &format!("DELETE FROM {table} WHERE repository_id=?1"),
                    params![repository_id],
                )
                .map_err(db_error)?;
        }
        let now = millis(TimestampMillis::now());
        for (id, server, root, pid, state, restart_count) in lsp_sessions {
            self.connection
                .execute(
                    "INSERT INTO lsp_server_sessions (
                        id, repository_id, server_type, workspace_root, pid, state,
                        restart_count, last_activity_ms, degraded_reason
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
                    params![
                        id,
                        repository_id,
                        server,
                        root,
                        pid,
                        state,
                        restart_count,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        for (kind, from_path, from_symbol, to_path, to_symbol, source, confidence) in edges {
            self.connection
                .execute(
                    "INSERT INTO semantic_edges (
                        id, repository_id, edge_kind, from_path, from_symbol, to_path, to_symbol,
                        provenance_source, confidence, freshness, commit_ref, worktree_id, created_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'fresh', ?10, ?11, ?12)",
                    params![
                        StableId::new("edge").to_string(),
                        repository_id,
                        kind,
                        from_path,
                        from_symbol,
                        to_path,
                        to_symbol,
                        source,
                        confidence,
                        commit,
                        worktree_id,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        for (path, line, severity, message, confidence) in diagnostics {
            self.connection
                .execute(
                    "INSERT INTO semantic_diagnostics (
                        id, repository_id, path, line, severity, message, provenance_source,
                        confidence, freshness, commit_ref, worktree_id, created_at_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'lsp-diagnostics-normalized', ?7, 'fresh', ?8, ?9, ?10)",
                    params![
                        StableId::new("diag").to_string(),
                        repository_id,
                        path,
                        line,
                        severity,
                        message,
                        confidence,
                        commit,
                        worktree_id,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        for (kind, root_path, package_name, evidence_path) in workspace_boundaries {
            self.connection
                .execute(
                    "INSERT INTO workspace_boundaries VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![repository_id, kind, root_path, package_name, evidence_path],
                )
                .map_err(db_error)?;
        }
        for (engine, enabled, reason, measured_files, measured_edges) in optional_indexes {
            self.connection
                .execute(
                    "INSERT INTO optional_index_decisions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        repository_id,
                        engine,
                        if *enabled { 1_i64 } else { 0_i64 },
                        reason,
                        *measured_files as i64,
                        *measured_edges as i64,
                        now
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn semantic_edge_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM semantic_edges WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn workspace_boundary_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM workspace_boundaries WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn optional_index_enabled(
        &self,
        repository_id: &str,
        engine: &str,
    ) -> AcResult<Option<bool>> {
        self.connection
            .query_row(
                "SELECT enabled FROM optional_index_decisions WHERE repository_id=?1 AND engine=?2",
                params![repository_id, engine],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.map(|enabled| enabled != 0))
            .map_err(db_error)
    }

    pub fn save_memory_fact(
        &self,
        fact: &MemoryFactRow,
        evidence: &[MemoryEvidenceRow],
    ) -> AcResult<()> {
        if fact.statement.trim().is_empty() || evidence.is_empty() {
            return Err(AcError::validation(
                "DB-MEMORY_FACT_INVALID",
                "memory facts require statement and evidence",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO memory_facts (
                    id, repository_id, mission_id, task_id, branch, statement, fact_type, source,
                    confidence, freshness, memory_class, observed_commit, conflict_set_id,
                    valid_from_ms, valid_until_ms, superseded_by, last_validation_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                 ON CONFLICT(id) DO UPDATE SET
                    statement=excluded.statement,
                    confidence=excluded.confidence,
                    freshness=excluded.freshness,
                    conflict_set_id=excluded.conflict_set_id,
                    valid_until_ms=excluded.valid_until_ms,
                    superseded_by=excluded.superseded_by,
                    last_validation_ms=excluded.last_validation_ms",
                params![
                    fact.id,
                    fact.repository_id,
                    fact.mission_id,
                    fact.task_id,
                    fact.branch,
                    fact.statement,
                    fact.fact_type,
                    fact.source,
                    fact.confidence,
                    fact.freshness,
                    fact.memory_class,
                    fact.observed_commit,
                    fact.conflict_set_id,
                    fact.valid_from_ms,
                    fact.valid_until_ms,
                    fact.superseded_by,
                    fact.last_validation_ms
                ],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "DELETE FROM memory_fact_evidence WHERE fact_id=?1",
                params![fact.id],
            )
            .map_err(db_error)?;
        for row in evidence {
            self.connection
                .execute(
                    "INSERT INTO memory_fact_evidence VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        row.fact_id,
                        row.evidence_ref,
                        row.file_path,
                        row.symbol,
                        row.content_hash
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn memory_fact(&self, id: &str) -> AcResult<Option<MemoryFactRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, mission_id, task_id, branch, statement, fact_type,
                        source, confidence, freshness, memory_class, observed_commit,
                        conflict_set_id, valid_from_ms, valid_until_ms, superseded_by,
                        last_validation_ms
                 FROM memory_facts WHERE id=?1",
                params![id],
                memory_fact_from_row,
            )
            .optional()
            .map_err(db_error)
    }

    pub fn memory_fact_evidence(&self, fact_id: &str) -> AcResult<Vec<MemoryEvidenceRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT fact_id, evidence_ref, file_path, symbol, content_hash
                 FROM memory_fact_evidence WHERE fact_id=?1 ORDER BY evidence_ref ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![fact_id], |row| {
                Ok(MemoryEvidenceRow {
                    fact_id: row.get(0)?,
                    evidence_ref: row.get(1)?,
                    file_path: row.get(2)?,
                    symbol: row.get(3)?,
                    content_hash: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn mark_memory_for_source_change(
        &self,
        file_path: &str,
        symbol: Option<&str>,
        deleted: bool,
    ) -> AcResult<u64> {
        let freshness = if deleted { "INVALID" } else { "POSSIBLY_STALE" };
        let changed = if let Some(symbol) = symbol {
            self.connection.execute(
                "UPDATE memory_facts
                 SET freshness=?3, last_validation_ms=?4
                 WHERE id IN (
                    SELECT fact_id FROM memory_fact_evidence
                    WHERE file_path=?1 AND (symbol IS NULL OR symbol=?2)
                 )",
                params![file_path, symbol, freshness, millis(TimestampMillis::now())],
            )
        } else {
            self.connection.execute(
                "UPDATE memory_facts
                 SET freshness=?2, last_validation_ms=?3
                 WHERE id IN (
                    SELECT fact_id FROM memory_fact_evidence WHERE file_path=?1
                 )",
                params![file_path, freshness, millis(TimestampMillis::now())],
            )
        }
        .map_err(db_error)?;
        Ok(changed as u64)
    }

    pub fn save_memory_decision(&self, decision: &MemoryDecisionRow) -> AcResult<()> {
        if decision.decision.trim().is_empty() || decision.authority_refs.trim().is_empty() {
            return Err(AcError::validation(
                "DB-MEMORY_DECISION_INVALID",
                "decisions require text and authority refs",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO memory_decisions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    decision.id,
                    decision.repository_id,
                    decision.mission_id,
                    decision.task_id,
                    decision.branch,
                    decision.decision,
                    decision.rationale,
                    decision.authority_refs,
                    decision.supersedes,
                    decision.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn memory_decision_count(&self, repository_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM memory_decisions WHERE repository_id=?1",
                params![repository_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn save_task_memory(&self, memory: &TaskMemoryRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO task_memory VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    memory.id,
                    memory.task_id,
                    memory.summary,
                    memory.evidence_refs,
                    memory.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn task_memory_count(&self, task_id: &str) -> AcResult<u64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM task_memory WHERE task_id=?1",
                params![task_id],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn save_context_snapshot(&self, snapshot: &ContextSnapshotRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_snapshots VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    snapshot.id,
                    snapshot.repository_id,
                    snapshot.mission_id,
                    snapshot.task_id,
                    snapshot.branch,
                    snapshot.reason,
                    snapshot.content,
                    snapshot.source_fact_ids,
                    snapshot.decision_refs,
                    snapshot.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_snapshot(&self, id: &str) -> AcResult<Option<ContextSnapshotRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, mission_id, task_id, branch, reason, content,
                        source_fact_ids, decision_refs, created_at_ms
                 FROM context_snapshots WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextSnapshotRow {
                        id: row.get(0)?,
                        repository_id: row.get(1)?,
                        mission_id: row.get(2)?,
                        task_id: row.get(3)?,
                        branch: row.get(4)?,
                        reason: row.get(5)?,
                        content: row.get(6)?,
                        source_fact_ids: row.get(7)?,
                        decision_refs: row.get(8)?,
                        created_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_pack_manifest(&self, manifest: &ContextPackManifestRow) -> AcResult<()> {
        if manifest.source_fragment_ids.trim().is_empty() {
            return Err(AcError::validation(
                "DB-CONTEXT_MANIFEST_INVALID",
                "context manifest requires source fragment provenance",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO context_pack_manifests VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    manifest.id,
                    manifest.pack_id,
                    manifest.task_id,
                    manifest.role,
                    manifest.profile,
                    manifest.source_fragment_ids,
                    manifest.omitted_fragment_ids,
                    manifest.raw_evidence_refs,
                    manifest.cache_keys,
                    manifest.score_trace,
                    manifest.total_input_tokens,
                    manifest.hard_ceiling,
                    manifest.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_pack_manifest(&self, id: &str) -> AcResult<Option<ContextPackManifestRow>> {
        self.connection
            .query_row(
                "SELECT id, pack_id, task_id, role, profile, source_fragment_ids,
                        omitted_fragment_ids, raw_evidence_refs, cache_keys, score_trace,
                        total_input_tokens, hard_ceiling, created_at_ms
                 FROM context_pack_manifests WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextPackManifestRow {
                        id: row.get(0)?,
                        pack_id: row.get(1)?,
                        task_id: row.get(2)?,
                        role: row.get(3)?,
                        profile: row.get(4)?,
                        source_fragment_ids: row.get(5)?,
                        omitted_fragment_ids: row.get(6)?,
                        raw_evidence_refs: row.get(7)?,
                        cache_keys: row.get(8)?,
                        score_trace: row.get(9)?,
                        total_input_tokens: row.get(10)?,
                        hard_ceiling: row.get(11)?,
                        created_at_ms: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_compression_receipt(
        &self,
        receipt: &ContextCompressionReceiptRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_compression_receipts VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    receipt.id,
                    receipt.raw_evidence_ref,
                    receipt.command_class,
                    receipt.compressor_id,
                    receipt.raw_hash,
                    receipt.compressed_output,
                    receipt.raw_token_estimate,
                    receipt.compressed_token_estimate,
                    receipt.omitted_lines,
                    receipt.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_compression_receipt(
        &self,
        id: &str,
    ) -> AcResult<Option<ContextCompressionReceiptRow>> {
        self.connection
            .query_row(
                "SELECT id, raw_evidence_ref, command_class, compressor_id, raw_hash,
                        compressed_output, raw_token_estimate, compressed_token_estimate,
                        omitted_lines, created_at_ms
                 FROM context_compression_receipts WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextCompressionReceiptRow {
                        id: row.get(0)?,
                        raw_evidence_ref: row.get(1)?,
                        command_class: row.get(2)?,
                        compressor_id: row.get(3)?,
                        raw_hash: row.get(4)?,
                        compressed_output: row.get(5)?,
                        raw_token_estimate: row.get(6)?,
                        compressed_token_estimate: row.get(7)?,
                        omitted_lines: row.get(8)?,
                        created_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_cache_entry(&self, entry: &ContextCacheEntryRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_cache_entries VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(cache_key) DO UPDATE SET
                    content_hash=excluded.content_hash,
                    token_estimate=excluded.token_estimate,
                    source_ref=excluded.source_ref,
                    created_at_ms=excluded.created_at_ms",
                params![
                    entry.cache_key,
                    entry.content_hash,
                    entry.token_estimate,
                    entry.source_ref,
                    entry.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_cache_entry(&self, cache_key: &str) -> AcResult<Option<ContextCacheEntryRow>> {
        self.connection
            .query_row(
                "SELECT cache_key, content_hash, token_estimate, source_ref, created_at_ms
                 FROM context_cache_entries WHERE cache_key=?1",
                params![cache_key],
                |row| {
                    Ok(ContextCacheEntryRow {
                        cache_key: row.get(0)?,
                        content_hash: row.get(1)?,
                        token_estimate: row.get(2)?,
                        source_ref: row.get(3)?,
                        created_at_ms: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_retrieval_record(
        &self,
        record: &ContextRetrievalRecordRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_retrieval_records VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.pack_id,
                    record.need,
                    record.reason,
                    record.query,
                    record.result_fragment_ids,
                    record.added_tokens,
                    if record.degraded { 1_i64 } else { 0_i64 },
                    record.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_retrieval_records(
        &self,
        pack_id: &str,
    ) -> AcResult<Vec<ContextRetrievalRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, pack_id, need, reason, query, result_fragment_ids, added_tokens,
                        degraded, created_at_ms
                 FROM context_retrieval_records WHERE pack_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![pack_id], |row| {
                Ok(ContextRetrievalRecordRow {
                    id: row.get(0)?,
                    pack_id: row.get(1)?,
                    need: row.get(2)?,
                    reason: row.get(3)?,
                    query: row.get(4)?,
                    result_fragment_ids: row.get(5)?,
                    added_tokens: row.get(6)?,
                    degraded: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_context_pack_metrics(&self, metrics: &ContextPackMetricsRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_pack_metrics VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    metrics.id,
                    metrics.pack_id,
                    metrics.role,
                    metrics.selected_fragments as i64,
                    metrics.omitted_fragments as i64,
                    metrics.total_input_tokens,
                    metrics.budget_target,
                    metrics.hard_ceiling,
                    metrics.deduped_fragments as i64,
                    metrics.redacted_fragments as i64,
                    metrics.retrieval_steps as i64,
                    metrics.cache_hits as i64,
                    metrics.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_pack_metrics(&self, pack_id: &str) -> AcResult<Option<ContextPackMetricsRow>> {
        self.connection
            .query_row(
                "SELECT id, pack_id, role, selected_fragments, omitted_fragments,
                        total_input_tokens, budget_target, hard_ceiling, deduped_fragments,
                        redacted_fragments, retrieval_steps, cache_hits, created_at_ms
                 FROM context_pack_metrics WHERE pack_id=?1",
                params![pack_id],
                |row| {
                    Ok(ContextPackMetricsRow {
                        id: row.get(0)?,
                        pack_id: row.get(1)?,
                        role: row.get(2)?,
                        selected_fragments: row.get::<_, i64>(3)? as usize,
                        omitted_fragments: row.get::<_, i64>(4)? as usize,
                        total_input_tokens: row.get(5)?,
                        budget_target: row.get(6)?,
                        hard_ceiling: row.get(7)?,
                        deduped_fragments: row.get::<_, i64>(8)? as usize,
                        redacted_fragments: row.get::<_, i64>(9)? as usize,
                        retrieval_steps: row.get::<_, i64>(10)? as usize,
                        cache_hits: row.get::<_, i64>(11)? as usize,
                        created_at_ms: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_context_benchmark_result(
        &self,
        result: &ContextBenchmarkResultRow,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO context_benchmark_results VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    result.id,
                    result.task_name,
                    result.broad_tokens,
                    result.targeted_tokens,
                    if result.broad_success { 1_i64 } else { 0_i64 },
                    if result.targeted_success { 1_i64 } else { 0_i64 },
                    result.retry_delta,
                    result.latency_delta_ms,
                    if result.passed { 1_i64 } else { 0_i64 },
                    result.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn context_benchmark_result(
        &self,
        id: &str,
    ) -> AcResult<Option<ContextBenchmarkResultRow>> {
        self.connection
            .query_row(
                "SELECT id, task_name, broad_tokens, targeted_tokens, broad_success,
                        targeted_success, retry_delta, latency_delta_ms, passed, created_at_ms
                 FROM context_benchmark_results WHERE id=?1",
                params![id],
                |row| {
                    Ok(ContextBenchmarkResultRow {
                        id: row.get(0)?,
                        task_name: row.get(1)?,
                        broad_tokens: row.get(2)?,
                        targeted_tokens: row.get(3)?,
                        broad_success: row.get::<_, i64>(4)? != 0,
                        targeted_success: row.get::<_, i64>(5)? != 0,
                        retry_delta: row.get(6)?,
                        latency_delta_ms: row.get(7)?,
                        passed: row.get::<_, i64>(8)? != 0,
                        created_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn put_mission(&self, mission: &Mission) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO missions (id, original_goal, state, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at_ms = excluded.updated_at_ms",
                params![
                    mission.id.as_str(),
                    mission.original_goal,
                    mission_state(mission.state),
                    millis(mission.created_at),
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn get_mission(&self, id: &StableId) -> AcResult<Option<PersistedMission>> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, original_goal, state, created_at_ms FROM missions WHERE id = ?1")
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedMission {
                id: row.get(0).map_err(db_error)?,
                original_goal: row.get(1).map_err(db_error)?,
                state: row.get(2).map_err(db_error)?,
                created_at_ms: row.get(3).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn append_kernel_event(&self, event: &KernelEvent) -> AcResult<()> {
        let evidence_refs = event
            .evidence_refs
            .iter()
            .map(StableId::to_string)
            .collect::<Vec<_>>()
            .join(",");
        self.connection
            .execute(
                "INSERT OR IGNORE INTO kernel_events (id, decision_kind, subject_id, evidence_refs, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event.id.as_str(),
                    decision_kind(event.decision),
                    event.subject.as_str(),
                    evidence_refs,
                    millis(event.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn kernel_event_count(&self) -> AcResult<u64> {
        self.connection
            .query_row("SELECT COUNT(*) FROM kernel_events", [], |row| row.get(0))
            .map_err(db_error)
    }

    pub fn append_evidence(&self, evidence: &EvidenceRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO evidence_records (id, kind, provenance_json, artifact_uri, content_hash, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    evidence.id.as_str(),
                    format!("{:?}", evidence.kind),
                    format!("{:?}", evidence.provenance),
                    evidence.artifact_uri,
                    evidence.content_hash,
                    millis(evidence.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_edit_transaction(
        &self,
        transaction: &EditTransactionRow,
        operations: &[EditOperationRow],
        journal: &[EditJournalEntryRow],
        metrics: &[EditStrategyMetricRow],
    ) -> AcResult<()> {
        if transaction.base_revision.trim().is_empty() || operations.is_empty() {
            return Err(AcError::validation(
                "DB-EDIT_TRANSACTION_INVALID",
                "edit transactions require base revision and operations",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO edit_transactions (
                    id, changeset_id, task_id, worktree_id, base_revision, state, formatter,
                    degraded_reason, created_at_ms, updated_at_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    state=excluded.state,
                    formatter=excluded.formatter,
                    degraded_reason=excluded.degraded_reason,
                    updated_at_ms=excluded.updated_at_ms",
                params![
                    transaction.id,
                    transaction.changeset_id,
                    transaction.task_id,
                    transaction.worktree_id,
                    transaction.base_revision,
                    transaction.state,
                    transaction.formatter,
                    transaction.degraded_reason,
                    transaction.created_at_ms,
                    transaction.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        for table in [
            "edit_operations",
            "edit_journal_entries",
            "edit_strategy_metrics",
        ] {
            self.connection
                .execute(
                    &format!("DELETE FROM {table} WHERE transaction_id=?1"),
                    params![transaction.id],
                )
                .map_err(db_error)?;
        }
        for row in operations {
            self.connection
                .execute(
                    "INSERT INTO edit_operations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        row.id,
                        row.transaction_id,
                        row.path,
                        row.strategy,
                        row.before_hash,
                        row.after_hash,
                        row.symbol_fingerprint,
                        row.additions,
                        row.removals
                    ],
                )
                .map_err(db_error)?;
        }
        for row in journal {
            self.connection
                .execute(
                    "INSERT INTO edit_journal_entries VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        row.id,
                        row.transaction_id,
                        row.path,
                        row.state,
                        row.before_hash,
                        row.after_hash,
                        row.created_at_ms
                    ],
                )
                .map_err(db_error)?;
        }
        for row in metrics {
            self.connection
                .execute(
                    "INSERT INTO edit_strategy_metrics VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        row.id,
                        row.transaction_id,
                        row.task_id,
                        row.model_id,
                        row.language,
                        row.strategy,
                        if row.first_apply_success { 1_i64 } else { 0_i64 },
                        row.syntax_failures,
                        row.retries,
                        row.unrelated_diff_files,
                        row.verification_rejections,
                        if row.degraded { 1_i64 } else { 0_i64 },
                        row.created_at_ms
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn save_changeset_transaction(
        &self,
        transaction: &ChangeSetTransaction,
        task_id: Option<&str>,
        worktree_id: Option<&str>,
        base_revision: &str,
        language: &str,
    ) -> AcResult<()> {
        let now = millis(TimestampMillis::now());
        let row = EditTransactionRow {
            id: transaction.id.to_string(),
            changeset_id: transaction.changeset.id.to_string(),
            task_id: task_id.map(ToString::to_string),
            worktree_id: worktree_id.map(ToString::to_string),
            base_revision: base_revision.to_string(),
            state: format!("{:?}", transaction.changeset.state),
            formatter: transaction.format_plan.formatter.clone(),
            degraded_reason: transaction.format_plan.degraded_reason.clone(),
            created_at_ms: millis(transaction.changeset.created_at),
            updated_at_ms: now,
        };
        let operations = transaction
            .edits
            .iter()
            .map(|edit| EditOperationRow {
                id: StableId::new("editop").to_string(),
                transaction_id: row.id.clone(),
                path: edit.path.clone(),
                strategy: format!("{:?}", edit.strategy),
                before_hash: edit.before_hash.clone(),
                after_hash: edit.after_hash.clone(),
                symbol_fingerprint: edit.symbol_fingerprint.clone(),
                additions: edit.additions,
                removals: edit.removals,
            })
            .collect::<Vec<_>>();
        let journal = transaction
            .journal
            .entries
            .iter()
            .map(|entry| EditJournalEntryRow {
                id: entry.id.to_string(),
                transaction_id: row.id.clone(),
                path: entry.path.clone(),
                state: format!("{:?}", entry.state),
                before_hash: entry.before_hash.clone(),
                after_hash: entry.after_hash.clone(),
                created_at_ms: millis(entry.created_at),
            })
            .collect::<Vec<_>>();
        let metrics = vec![EditStrategyMetricRow {
            id: StableId::new("editmetric").to_string(),
            transaction_id: row.id.clone(),
            task_id: task_id.map(ToString::to_string),
            model_id: None,
            language: language.to_string(),
            strategy: format!("{:?}", transaction.metrics.strategy),
            first_apply_success: transaction.metrics.first_apply_success,
            syntax_failures: transaction.metrics.syntax_failures,
            retries: transaction.metrics.retries,
            unrelated_diff_files: transaction.metrics.unrelated_diff_files,
            verification_rejections: transaction.metrics.verification_rejections,
            degraded: transaction.metrics.degraded,
            created_at_ms: now,
        }];
        self.save_edit_transaction(&row, &operations, &journal, &metrics)
    }

    pub fn edit_transaction(&self, id: &str) -> AcResult<Option<EditTransactionRow>> {
        self.connection
            .query_row(
                "SELECT id, changeset_id, task_id, worktree_id, base_revision, state, formatter,
                        degraded_reason, created_at_ms, updated_at_ms
                 FROM edit_transactions WHERE id=?1",
                params![id],
                |row| {
                    Ok(EditTransactionRow {
                        id: row.get(0)?,
                        changeset_id: row.get(1)?,
                        task_id: row.get(2)?,
                        worktree_id: row.get(3)?,
                        base_revision: row.get(4)?,
                        state: row.get(5)?,
                        formatter: row.get(6)?,
                        degraded_reason: row.get(7)?,
                        created_at_ms: row.get(8)?,
                        updated_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn edit_journal_entries(&self, transaction_id: &str) -> AcResult<Vec<EditJournalEntryRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, transaction_id, path, state, before_hash, after_hash, created_at_ms
                 FROM edit_journal_entries WHERE transaction_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![transaction_id], |row| {
                Ok(EditJournalEntryRow {
                    id: row.get(0)?,
                    transaction_id: row.get(1)?,
                    path: row.get(2)?,
                    state: row.get(3)?,
                    before_hash: row.get(4)?,
                    after_hash: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn pending_edit_transactions(&self) -> AcResult<Vec<EditTransactionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, changeset_id, task_id, worktree_id, base_revision, state, formatter,
                        degraded_reason, created_at_ms, updated_at_ms
                 FROM edit_transactions
                 WHERE state IN ('Applying', 'RollingBack', 'UnknownEffect')
                 ORDER BY updated_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EditTransactionRow {
                    id: row.get(0)?,
                    changeset_id: row.get(1)?,
                    task_id: row.get(2)?,
                    worktree_id: row.get(3)?,
                    base_revision: row.get(4)?,
                    state: row.get(5)?,
                    formatter: row.get(6)?,
                    degraded_reason: row.get(7)?,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn edit_strategy_metrics(
        &self,
        transaction_id: &str,
    ) -> AcResult<Vec<EditStrategyMetricRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, transaction_id, task_id, model_id, language, strategy,
                        first_apply_success, syntax_failures, retries, unrelated_diff_files,
                        verification_rejections, degraded, created_at_ms
                 FROM edit_strategy_metrics WHERE transaction_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![transaction_id], |row| {
                Ok(EditStrategyMetricRow {
                    id: row.get(0)?,
                    transaction_id: row.get(1)?,
                    task_id: row.get(2)?,
                    model_id: row.get(3)?,
                    language: row.get(4)?,
                    strategy: row.get(5)?,
                    first_apply_success: row.get::<_, i64>(6)? != 0,
                    syntax_failures: row.get(7)?,
                    retries: row.get(8)?,
                    unrelated_diff_files: row.get(9)?,
                    verification_rejections: row.get(10)?,
                    degraded: row.get::<_, i64>(11)? != 0,
                    created_at_ms: row.get(12)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_verification_profile(&self, profile: &VerificationProfile) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO verification_profiles VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET required_layers=excluded.required_layers",
                params![
                    profile.id.to_string(),
                    profile.task_id.to_string(),
                    format!("{:?}", profile.risk),
                    profile
                        .required_layers
                        .iter()
                        .map(|layer| format!("{:?}", layer))
                        .collect::<Vec<_>>()
                        .join(","),
                    millis(profile.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_verification_manifest(
        &self,
        manifest: &VerificationEvidenceManifest,
        profile_id: Option<&str>,
        task_id: Option<&str>,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO verification_runs VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    manifest.id.to_string(),
                    profile_id,
                    task_id,
                    manifest.commit,
                    manifest.worktree,
                    manifest.environment,
                    manifest.command,
                    manifest.tool_version,
                    format!("{:?}", manifest.normalized_result),
                    manifest.raw_artifact,
                    manifest.evidence_ref.to_string(),
                    manifest.freshness_dependencies.join(","),
                    millis(manifest.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_requirement_verification(
        &self,
        run_id: &str,
        link: &RequirementEvidenceLink,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO requirement_verifications VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    StableId::new("reqver").to_string(),
                    link.requirement_id.to_string(),
                    run_id,
                    link.evidence_ref.to_string(),
                    link.evidence_kind,
                    if link.verified { 1_i64 } else { 0_i64 },
                    link.freshness_key,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_final_audit(
        &self,
        mission_id: &str,
        original_goal: &str,
        requirements: &[String],
        audit: &FinalAuditReport,
        completion_allowed: bool,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO final_audits VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    audit.id.to_string(),
                    mission_id,
                    original_goal,
                    requirements.join("\n"),
                    audit
                        .findings
                        .iter()
                        .map(|finding| finding.code.clone())
                        .collect::<Vec<_>>()
                        .join(","),
                    if audit.passed { 1_i64 } else { 0_i64 },
                    if audit.return_to_repair { 1_i64 } else { 0_i64 },
                    if completion_allowed { 1_i64 } else { 0_i64 },
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn verification_run(&self, id: &str) -> AcResult<Option<VerificationRunRow>> {
        self.connection
            .query_row(
                "SELECT id, profile_id, task_id, commit_ref, worktree_id, environment, command,
                        tool_version, normalized_result, raw_artifact, evidence_ref,
                        freshness_dependencies, created_at_ms
                 FROM verification_runs WHERE id=?1",
                params![id],
                |row| {
                    Ok(VerificationRunRow {
                        id: row.get(0)?,
                        profile_id: row.get(1)?,
                        task_id: row.get(2)?,
                        commit_ref: row.get(3)?,
                        worktree_id: row.get(4)?,
                        environment: row.get(5)?,
                        command: row.get(6)?,
                        tool_version: row.get(7)?,
                        normalized_result: row.get(8)?,
                        raw_artifact: row.get(9)?,
                        evidence_ref: row.get(10)?,
                        freshness_dependencies: row.get(11)?,
                        created_at_ms: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn requirement_verifications(
        &self,
        requirement_id: &str,
    ) -> AcResult<Vec<RequirementVerificationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, requirement_id, verification_run_id, evidence_ref, evidence_kind,
                        verified, freshness_key, created_at_ms
                 FROM requirement_verifications WHERE requirement_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![requirement_id], |row| {
                Ok(RequirementVerificationRow {
                    id: row.get(0)?,
                    requirement_id: row.get(1)?,
                    verification_run_id: row.get(2)?,
                    evidence_ref: row.get(3)?,
                    evidence_kind: row.get(4)?,
                    verified: row.get::<_, i64>(5)? != 0,
                    freshness_key: row.get(6)?,
                    created_at_ms: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn final_audits(&self, mission_id: &str) -> AcResult<Vec<FinalAuditRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, original_goal, requirements, evidence_refs, passed,
                        return_to_repair, completion_allowed, created_at_ms
                 FROM final_audits WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![mission_id], |row| {
                Ok(FinalAuditRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    original_goal: row.get(2)?,
                    requirements: row.get(3)?,
                    evidence_refs: row.get(4)?,
                    passed: row.get::<_, i64>(5)? != 0,
                    return_to_repair: row.get::<_, i64>(6)? != 0,
                    completion_allowed: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_browser_process(&self, process: &BrowserProcessRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_processes VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET state=excluded.state",
                params![
                    process.id.to_string(),
                    process.task_id.to_string(),
                    format!("{:?}", process.mode),
                    format!("{:?}", process.state),
                    process.profile_dir,
                    millis(process.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_session(&self, session: &BrowserSessionRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_sessions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    current_url=excluded.current_url,
                    stale_evidence_refs=excluded.stale_evidence_refs,
                    updated_at_ms=excluded.updated_at_ms",
                params![
                    session.id.to_string(),
                    session.task_id.to_string(),
                    session.process_id.to_string(),
                    session.current_url,
                    session.profile,
                    session.storage_state_ref,
                    if session.sensitive { 1_i64 } else { 0_i64 },
                    session
                        .stale_evidence_refs
                        .iter()
                        .map(StableId::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                    millis(session.created_at),
                    millis(session.updated_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_dev_server(&self, server: &DevServerRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_dev_servers VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    server.id.to_string(),
                    server.task_id.to_string(),
                    server.command.join("\n"),
                    server.port,
                    server.ready_url,
                    if server.process_alive { 1_i64 } else { 0_i64 },
                    if server.http_ready { 1_i64 } else { 0_i64 },
                    if server.route_loadable { 1_i64 } else { 0_i64 },
                    if server.retained { 1_i64 } else { 0_i64 }
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_screenshot(&self, screenshot: &ScreenshotEvidence) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_screenshots VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    screenshot.id.to_string(),
                    screenshot.session_id.to_string(),
                    screenshot.task_id.to_string(),
                    screenshot.commit,
                    screenshot.viewport.name,
                    screenshot.url,
                    screenshot.artifact_uri,
                    if screenshot.sensitive { 1_i64 } else { 0_i64 },
                    screenshot.evidence_ref.to_string(),
                    millis(screenshot.captured_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_browser_visual_qa(&self, report: &VisualQaReport) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO browser_visual_qa VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    report.id.to_string(),
                    report.screenshot_ref.to_string(),
                    if report.passed { 1_i64 } else { 0_i64 },
                    report
                        .findings
                        .iter()
                        .map(|finding| finding.code.clone())
                        .collect::<Vec<_>>()
                        .join(","),
                    report.adapter,
                    report.evidence_ref.to_string()
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn browser_session(&self, id: &str) -> AcResult<Option<BrowserSessionRow>> {
        self.connection
            .query_row(
                "SELECT id, task_id, process_id, current_url, profile, storage_state_ref,
                        sensitive, stale_evidence_refs, created_at_ms, updated_at_ms
                 FROM browser_sessions WHERE id=?1",
                params![id],
                |row| {
                    Ok(BrowserSessionRow {
                        id: row.get(0)?,
                        task_id: row.get(1)?,
                        process_id: row.get(2)?,
                        current_url: row.get(3)?,
                        profile: row.get(4)?,
                        storage_state_ref: row.get(5)?,
                        sensitive: row.get::<_, i64>(6)? != 0,
                        stale_evidence_refs: row.get(7)?,
                        created_at_ms: row.get(8)?,
                        updated_at_ms: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn browser_screenshots(&self, session_id: &str) -> AcResult<Vec<BrowserScreenshotRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, session_id, task_id, commit_ref, viewport, url, artifact_uri,
                        sensitive, evidence_ref, captured_at_ms
                 FROM browser_screenshots WHERE session_id=?1 ORDER BY captured_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(BrowserScreenshotRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    task_id: row.get(2)?,
                    commit_ref: row.get(3)?,
                    viewport: row.get(4)?,
                    url: row.get(5)?,
                    artifact_uri: row.get(6)?,
                    sensitive: row.get::<_, i64>(7)? != 0,
                    evidence_ref: row.get(8)?,
                    captured_at_ms: row.get(9)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_skill(&self, skill: &SkillManifest) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO skills VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(id) DO UPDATE SET
                    description=excluded.description,
                    trigger_hints=excluded.trigger_hints,
                    loaded_at_ms=excluded.loaded_at_ms",
                params![
                    skill.id.to_string(),
                    skill.name,
                    skill.description,
                    skill.version,
                    skill.source,
                    format!("{:?}", skill.scope),
                    format!("{:?}", skill.trust_tier),
                    skill.trigger_hints.join(","),
                    format!("{:?}", skill.required_capabilities),
                    skill.context_cost,
                    skill.project_id.as_ref().map(StableId::to_string),
                    skill.task_id.as_ref().map(StableId::to_string),
                    skill.full_instructions,
                    skill.loaded_at.map(millis)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn skill(&self, id: &str) -> AcResult<Option<SkillRow>> {
        self.connection
            .query_row(
                "SELECT id, name, scope, trust_tier, full_instructions FROM skills WHERE id=?1",
                params![id],
                |row| {
                    Ok(SkillRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        scope: row.get(2)?,
                        trust_tier: row.get(3)?,
                        full_instructions: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_hook_manifest(&self, hook: &HookManifest) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO hook_manifests VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET timeout_ms=excluded.timeout_ms",
                params![
                    hook.id.to_string(),
                    hook.extension_id.to_string(),
                    format!("{:?}", hook.event),
                    hook.priority,
                    hook.timeout_ms,
                    hook.idempotency_key,
                    format!("{:?}", hook.failure_policy),
                    format!("{:?}", hook.required_capabilities)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_hook_invocation(&self, invocation: &HookInvocation) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO hook_invocations VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    invocation.id.to_string(),
                    invocation.hook_id.to_string(),
                    format!("{:?}", invocation.event),
                    format!("{:?}", invocation.outcome),
                    invocation.evidence_ref.as_ref().map(StableId::to_string),
                    millis(invocation.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn hook_invocations(&self, hook_id: &str) -> AcResult<Vec<HookInvocationRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, hook_id, outcome, evidence_ref
                 FROM hook_invocations WHERE hook_id=?1 ORDER BY created_at_ms",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![hook_id], |row| {
                Ok(HookInvocationRow {
                    id: row.get(0)?,
                    hook_id: row.get(1)?,
                    outcome: row.get(2)?,
                    evidence_ref: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_mcp_server(&self, server: &McpServerRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO mcp_servers VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET health=excluded.health, restart_count=excluded.restart_count, trust_tier=excluded.trust_tier",
                params![
                    server.id.to_string(),
                    server.name,
                    server.version,
                    format!("{:?}", server.transport),
                    format!("{:?}", server.trust_tier),
                    format!("{:?}", server.health),
                    server.restart_count
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_mcp_tool(&self, tool: &McpToolRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO mcp_tools VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET schema_json=excluded.schema_json",
                params![
                    tool.id.to_string(),
                    tool.server_id.to_string(),
                    tool.name,
                    tool.description,
                    tool.schema,
                    format!("{:?}", tool.risk),
                    format!("{:?}", tool.required_capabilities)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_mcp_invocation(&self, invocation: &McpInvocationRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO mcp_invocations VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    invocation.id.to_string(),
                    invocation.server_id.to_string(),
                    invocation.tool_id.to_string(),
                    format!("{:?}", invocation.status),
                    invocation.output,
                    invocation.evidence_ref.to_string(),
                    millis(invocation.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn mcp_server(&self, id: &str) -> AcResult<Option<McpServerRow>> {
        self.connection
            .query_row(
                "SELECT id, name, health, restart_count FROM mcp_servers WHERE id=?1",
                params![id],
                |row| {
                    Ok(McpServerRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        health: row.get(2)?,
                        restart_count: row.get::<_, i64>(3)? as u32,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_security_scan(
        &self,
        input: &SecurityScanInput,
        report: &SecurityScanReport,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_threat_models VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET evidence_refs=excluded.evidence_refs",
                params![
                    report.threat_model.id.to_string(),
                    input.repository_id.to_string(),
                    input.commit,
                    report.threat_model.entry_points.join(","),
                    report.threat_model.auth_boundaries.join(","),
                    report.threat_model.data_stores.join(","),
                    report.threat_model.admin_operations.join(","),
                    report.threat_model.cloud_configuration.join(","),
                    report.threat_model.sensitive_assets.join(","),
                    stable_ids_csv(&report.threat_model.evidence_refs)
                ],
            )
            .map_err(db_error)?;
        self.connection
            .execute(
                "INSERT INTO security_scan_reports VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET missing_adapters=excluded.missing_adapters",
                params![
                    report.id.to_string(),
                    input.repository_id.to_string(),
                    input.commit,
                    format!("{:?}", report.adapters_run),
                    report.missing_adapters.join(","),
                    report.threat_model.id.to_string()
                ],
            )
            .map_err(db_error)?;
        for item in &report.instances {
            self.connection
                .execute(
                    "INSERT INTO security_finding_instances VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                     ON CONFLICT(id) DO NOTHING",
                    params![
                        item.id.to_string(),
                        report.id.to_string(),
                        format!("{:?}", item.adapter),
                        item.rule_id,
                        format!("{:?}", item.severity),
                        item.confidence,
                        format!("{:?}", item.proof_level),
                        item.file_path,
                        item.line,
                        item.fingerprint,
                        item.redacted_evidence,
                        item.raw_evidence_ref.to_string()
                    ],
                )
                .map_err(db_error)?;
        }
        for finding in &report.findings {
            self.connection
                .execute(
                    "INSERT INTO security_findings VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT(id) DO UPDATE SET status=excluded.status",
                    params![
                        finding.id.to_string(),
                        report.id.to_string(),
                        finding.root_cause,
                        format!("{:?}", finding.severity),
                        finding.confidence,
                        finding.exploitability,
                        format!("{:?}", finding.status),
                        finding.affected_code.join(","),
                        stable_ids_csv(&finding.evidence_refs),
                        finding.remediation,
                        stable_ids_csv(&finding.instance_ids)
                    ],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }

    pub fn save_security_reports(
        &self,
        scan_id: &StableId,
        bundle: &SecurityReportBundle,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO security_reports VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scan_id) DO UPDATE SET markdown=excluded.markdown, json_report=excluded.json_report, sarif=excluded.sarif",
                params![scan_id.to_string(), bundle.markdown, bundle.json, bundle.sarif],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn security_scan(&self, id: &str) -> AcResult<Option<SecurityScanRow>> {
        self.connection
            .query_row(
                "SELECT id, repository_id, commit_ref, threat_model_id FROM security_scan_reports WHERE id=?1",
                params![id],
                |row| {
                    Ok(SecurityScanRow {
                        id: row.get(0)?,
                        repository_id: row.get(1)?,
                        commit_ref: row.get(2)?,
                        threat_model_id: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn security_findings(&self, scan_id: &str) -> AcResult<Vec<SecurityFindingRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, root_cause, severity, status FROM security_findings
                 WHERE scan_id=?1 ORDER BY root_cause",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![scan_id], |row| {
                Ok(SecurityFindingRow {
                    id: row.get(0)?,
                    root_cause: row.get(1)?,
                    severity: row.get(2)?,
                    status: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    fn configure(&self) -> AcResult<()> {
        self.connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_error)?;
        self.connection
            .busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(db_error)?;
        Ok(())
    }
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

impl ControlPlaneDb {
    pub fn save_session(
        &self,
        session_id: &StableId,
        mission_id: &StableId,
        state: &str,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO agent_sessions (id, mission_id, state, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at_ms = excluded.updated_at_ms",
                params![
                    session_id.as_str(),
                    mission_id.as_str(),
                    state,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_checkpoint(
        &self,
        id: &StableId,
        session_id: &StableId,
        next_step: u32,
        state: &str,
    ) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO agent_checkpoints (id, session_id, next_step, state, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id.as_str(),
                    session_id.as_str(),
                    next_step,
                    state,
                    millis(TimestampMillis::now())
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn update_session_state(&self, session_id: &StableId, state: &str) -> AcResult<()> {
        let changed = self
            .connection
            .execute(
                "UPDATE agent_sessions
                 SET state = ?2, updated_at_ms = ?3
                 WHERE id = ?1",
                params![session_id.as_str(), state, millis(TimestampMillis::now())],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err(AcError::conflict(
                "DB-SESSION_NOT_FOUND",
                format!("agent session {} was not found", session_id),
            ));
        }
        Ok(())
    }

    pub fn interrupted_sessions(&self) -> AcResult<Vec<PersistedSession>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, state, updated_at_ms
                 FROM agent_sessions
                 WHERE state NOT IN ('completed', 'cancelled', 'failed')",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PersistedSession {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    state: row.get(2)?,
                    updated_at_ms: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn get_session(&self, session_id: &StableId) -> AcResult<Option<PersistedSession>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, state, updated_at_ms FROM agent_sessions WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![session_id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedSession {
                id: row.get(0).map_err(db_error)?,
                mission_id: row.get(1).map_err(db_error)?,
                state: row.get(2).map_err(db_error)?,
                updated_at_ms: row.get(3).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn checkpoints_for_session(
        &self,
        session_id: &StableId,
    ) -> AcResult<Vec<PersistedCheckpoint>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, session_id, next_step, state, created_at_ms
                 FROM agent_checkpoints
                 WHERE session_id = ?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![session_id.as_str()], |row| {
                Ok(PersistedCheckpoint {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    next_step: row.get(2)?,
                    state: row.get(3)?,
                    created_at_ms: row.get(4)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_worktree(&self, worktree: &WorktreeRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO worktrees (
                    id, repository_id, owner_mission_id, owner_worker_id, lease_epoch, path, branch,
                    base_commit, current_commit, status, created_at_ms
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    owner_worker_id = excluded.owner_worker_id,
                    lease_epoch = excluded.lease_epoch,
                    current_commit = excluded.current_commit,
                    status = excluded.status",
                params![
                    worktree.id.as_str(),
                    worktree.repository_id.as_str(),
                    worktree.owner_mission_id.as_str(),
                    worktree.owner_worker_id.as_str(),
                    worktree.lease_epoch,
                    worktree.path.display().to_string(),
                    worktree.branch.as_str(),
                    worktree.base_commit.as_str(),
                    worktree.current_commit.as_str(),
                    format!("{:?}", worktree.status),
                    millis(worktree.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn get_worktree(&self, id: &StableId) -> AcResult<Option<PersistedWorktree>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, repository_id, owner_mission_id, owner_worker_id, lease_epoch, path, branch,
                        base_commit, current_commit, status, created_at_ms
                 FROM worktrees
                 WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedWorktree {
                id: row.get(0).map_err(db_error)?,
                repository_id: row.get(1).map_err(db_error)?,
                owner_mission_id: row.get(2).map_err(db_error)?,
                owner_worker_id: row.get(3).map_err(db_error)?,
                lease_epoch: row.get(4).map_err(db_error)?,
                path: row.get(5).map_err(db_error)?,
                branch: row.get(6).map_err(db_error)?,
                base_commit: row.get(7).map_err(db_error)?,
                current_commit: row.get(8).map_err(db_error)?,
                status: row.get(9).map_err(db_error)?,
                created_at_ms: row.get(10).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn save_git_checkpoint(&self, checkpoint: &CheckpointRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO worktree_checkpoints (id, worktree_id, commit_ref, reason, task_attempt_id, test_summary, context_ref, blocker, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    checkpoint.id.as_str(),
                    checkpoint.worktree_id.as_str(),
                    checkpoint.commit_ref.as_str(),
                    checkpoint.reason.as_str(),
                    checkpoint.task_attempt_id.as_ref().map(StableId::as_str),
                    checkpoint.test_summary.as_deref(),
                    checkpoint.context_ref.as_ref().map(StableId::as_str),
                    checkpoint.blocker.as_deref(),
                    millis(checkpoint.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn git_checkpoints_for_worktree(
        &self,
        worktree_id: &StableId,
    ) -> AcResult<Vec<PersistedGitCheckpoint>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, worktree_id, commit_ref, reason, task_attempt_id, test_summary, context_ref, blocker, created_at_ms
                 FROM worktree_checkpoints
                 WHERE worktree_id = ?1
                 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![worktree_id.as_str()], |row| {
                Ok(PersistedGitCheckpoint {
                    id: row.get(0)?,
                    worktree_id: row.get(1)?,
                    commit_ref: row.get(2)?,
                    reason: row.get(3)?,
                    task_attempt_id: row.get(4)?,
                    test_summary: row.get(5)?,
                    context_ref: row.get(6)?,
                    blocker: row.get(7)?,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_changeset(&self, changeset: &ChangeSet) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO changesets (id, state, operations_json, metadata_json, rollback_json, created_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    state = excluded.state,
                    operations_json = excluded.operations_json,
                    metadata_json = excluded.metadata_json,
                    rollback_json = excluded.rollback_json",
                params![
                    changeset.id.as_str(),
                    format!("{:?}", changeset.state),
                    format!("{:?}", changeset.operations),
                    changeset.metadata.as_ref().map(|metadata| format!("{:?}", metadata)),
                    changeset.rollback.as_ref().map(|rollback| format!("{:?}", rollback)),
                    millis(changeset.created_at)
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn get_changeset(&self, id: &StableId) -> AcResult<Option<PersistedChangeSet>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, state, operations_json, metadata_json, rollback_json, created_at_ms
                 FROM changesets
                 WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(PersistedChangeSet {
                id: row.get(0).map_err(db_error)?,
                state: row.get(1).map_err(db_error)?,
                operations_json: row.get(2).map_err(db_error)?,
                metadata_json: row.get(3).map_err(db_error)?,
                rollback_json: row.get(4).map_err(db_error)?,
                created_at_ms: row.get(5).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn save_routing_decision(&self, decision: &RoutingDecisionRecord) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO provider_routing_decisions (
                    id, task_id, candidates_json, selected_json, rejected_json, fallback_reason,
                    latency_ms, input_tokens, output_tokens, estimated_cost_micros, created_at_ms
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    candidates_json = excluded.candidates_json,
                    selected_json = excluded.selected_json,
                    rejected_json = excluded.rejected_json,
                    fallback_reason = excluded.fallback_reason,
                    latency_ms = excluded.latency_ms,
                    input_tokens = excluded.input_tokens,
                    output_tokens = excluded.output_tokens,
                    estimated_cost_micros = excluded.estimated_cost_micros",
                params![
                    decision.id.as_str(),
                    decision.task_id.as_str(),
                    decision.candidates_json.as_str(),
                    decision.selected_json.as_deref(),
                    decision.rejected_json.as_str(),
                    decision.fallback_reason.as_deref(),
                    decision.latency_ms,
                    decision.input_tokens,
                    decision.output_tokens,
                    decision.estimated_cost_micros,
                    decision.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn routing_decision(&self, id: &str) -> AcResult<Option<RoutingDecisionRecord>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, task_id, candidates_json, selected_json, rejected_json, fallback_reason,
                        latency_ms, input_tokens, output_tokens, estimated_cost_micros, created_at_ms
                 FROM provider_routing_decisions
                 WHERE id = ?1",
            )
            .map_err(db_error)?;
        let mut rows = stmt.query(params![id]).map_err(db_error)?;
        if let Some(row) = rows.next().map_err(db_error)? {
            return Ok(Some(RoutingDecisionRecord {
                id: row.get(0).map_err(db_error)?,
                task_id: row.get(1).map_err(db_error)?,
                candidates_json: row.get(2).map_err(db_error)?,
                selected_json: row.get(3).map_err(db_error)?,
                rejected_json: row.get(4).map_err(db_error)?,
                fallback_reason: row.get(5).map_err(db_error)?,
                latency_ms: row.get(6).map_err(db_error)?,
                input_tokens: row.get(7).map_err(db_error)?,
                output_tokens: row.get(8).map_err(db_error)?,
                estimated_cost_micros: row.get(9).map_err(db_error)?,
                created_at_ms: row.get(10).map_err(db_error)?,
            }));
        }
        Ok(None)
    }

    pub fn save_tool_execution(&self, record: &ToolExecutionRecord) -> AcResult<()> {
        self.connection.execute(
            "INSERT INTO tool_execution_records (id, tool_call_id, tool_id, status, manifest_json, raw_output, evidence_ref, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO NOTHING",
            params![record.id, record.tool_call_id, record.tool_id, record.status, record.manifest_json, record.raw_output, record.evidence_ref, record.created_at_ms as i64],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn tool_execution(&self, id: &str) -> AcResult<Option<ToolExecutionRecord>> {
        self.connection.query_row(
            "SELECT id, tool_call_id, tool_id, status, manifest_json, raw_output, evidence_ref, created_at_ms FROM tool_execution_records WHERE id = ?1",
            [id],
            |row| Ok(ToolExecutionRecord {
                id: row.get(0)?, tool_call_id: row.get(1)?, tool_id: row.get(2)?, status: row.get(3)?, manifest_json: row.get(4)?, raw_output: row.get(5)?, evidence_ref: row.get(6)?, created_at_ms: row.get::<_, i64>(7)? as u128,
            }),
        ).optional().map_err(db_error)
    }

    pub fn save_worker(&self, record: &WorkerRecord) -> AcResult<()> {
        self.connection.execute(
            "INSERT INTO workers (id, mission_id, session_id, state, workspace_ref, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET state = excluded.state, workspace_ref = excluded.workspace_ref, updated_at_ms = excluded.updated_at_ms",
            params![record.id, record.mission_id, record.session_id, record.state, record.workspace_ref, record.updated_at_ms],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn worker(&self, id: &str) -> AcResult<Option<WorkerRecord>> {
        self.connection.query_row(
            "SELECT id, mission_id, session_id, state, workspace_ref, updated_at_ms FROM workers WHERE id = ?1", [id],
            |row| Ok(WorkerRecord { id: row.get(0)?, mission_id: row.get(1)?, session_id: row.get(2)?, state: row.get(3)?, workspace_ref: row.get(4)?, updated_at_ms: row.get(5)? }),
        ).optional().map_err(db_error)
    }

    pub fn save_task(&self, record: &TaskRecord) -> AcResult<()> {
        self.connection.execute(
            "INSERT INTO tasks (id, mission_id, title, state, dependencies_json, assigned_worker_id, retry_count, max_retries, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET state = excluded.state, assigned_worker_id = excluded.assigned_worker_id, retry_count = excluded.retry_count, updated_at_ms = excluded.updated_at_ms",
            params![record.id, record.mission_id, record.title, record.state, record.dependencies_json, record.assigned_worker_id, record.retry_count, record.max_retries, record.updated_at_ms],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn tasks_for_mission(&self, mission_id: &str) -> AcResult<Vec<TaskRecord>> {
        let mut stmt = self.connection.prepare("SELECT id, mission_id, title, state, dependencies_json, assigned_worker_id, retry_count, max_retries, updated_at_ms FROM tasks WHERE mission_id = ?1 ORDER BY id").map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(TaskRecord {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    title: row.get(2)?,
                    state: row.get(3)?,
                    dependencies_json: row.get(4)?,
                    assigned_worker_id: row.get(5)?,
                    retry_count: row.get(6)?,
                    max_retries: row.get(7)?,
                    updated_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_task_attempt(&self, record: &TaskAttemptRecord) -> AcResult<()> {
        self.connection.execute(
            "INSERT INTO task_attempts (id, task_id, worker_id, outcome, evidence_refs, failure_class, created_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![record.id, record.task_id, record.worker_id, record.outcome, record.evidence_refs, record.failure_class, record.created_at_ms],
        ).map_err(db_error)?;
        Ok(())
    }

    pub fn task_attempts(&self, task_id: &str) -> AcResult<Vec<TaskAttemptRecord>> {
        let mut stmt = self.connection.prepare("SELECT id, task_id, worker_id, outcome, evidence_refs, failure_class, created_at_ms FROM task_attempts WHERE task_id = ?1 ORDER BY created_at_ms").map_err(db_error)?;
        let rows = stmt
            .query_map([task_id], |row| {
                Ok(TaskAttemptRecord {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    worker_id: row.get(2)?,
                    outcome: row.get(3)?,
                    evidence_refs: row.get(4)?,
                    failure_class: row.get(5)?,
                    created_at_ms: row.get(6)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_mission_contract_revision(&self, row: &MissionContractRevisionRow) -> AcResult<()> {
        if row.original_goal.trim().is_empty() || row.reason.trim().is_empty() {
            return Err(AcError::validation(
                "DB-MISSION_CONTRACT_INVALID",
                "mission contract revisions require original goal and reason",
            ));
        }
        self.connection
            .execute(
                "INSERT INTO mission_contract_revisions VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    row.id,
                    row.mission_id,
                    row.revision,
                    row.original_goal,
                    row.reason,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn mission_contract_revisions(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<MissionContractRevisionRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, revision, original_goal, reason, created_at_ms
             FROM mission_contract_revisions WHERE mission_id=?1 ORDER BY revision ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(MissionContractRevisionRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    revision: row.get(2)?,
                    original_goal: row.get(3)?,
                    reason: row.get(4)?,
                    created_at_ms: row.get(5)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_requirement_matrix_entry(&self, row: &RequirementMatrixEntryRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO requirement_matrix_entries VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    row.id,
                    row.mission_id,
                    row.contract_revision,
                    row.description,
                    row.requirement_type,
                    row.priority,
                    row.source,
                    row.verification_strategy,
                    if row.blocking { 1_i64 } else { 0_i64 },
                    row.implementation_status,
                    row.verification_status,
                    row.evidence_refs,
                    row.linked_task_ids,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn requirement_matrix_entries(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<RequirementMatrixEntryRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, contract_revision, description, requirement_type, priority,
                    source, verification_strategy, blocking, implementation_status,
                    verification_status, evidence_refs, linked_task_ids, created_at_ms
             FROM requirement_matrix_entries WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(RequirementMatrixEntryRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    contract_revision: row.get(2)?,
                    description: row.get(3)?,
                    requirement_type: row.get(4)?,
                    priority: row.get(5)?,
                    source: row.get(6)?,
                    verification_strategy: row.get(7)?,
                    blocking: row.get::<_, i64>(8)? != 0,
                    implementation_status: row.get(9)?,
                    verification_status: row.get(10)?,
                    evidence_refs: row.get(11)?,
                    linked_task_ids: row.get(12)?,
                    created_at_ms: row.get(13)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_task_lease(&self, row: &TaskLeaseRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO task_leases VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(task_id) DO UPDATE SET
                    worker_id=excluded.worker_id,
                    lease_epoch=excluded.lease_epoch,
                    expires_at_ms=excluded.expires_at_ms,
                    heartbeat_interval_ms=excluded.heartbeat_interval_ms,
                    state=excluded.state,
                    updated_at_ms=excluded.updated_at_ms",
                params![
                    row.task_id,
                    row.worker_id,
                    row.lease_epoch,
                    row.expires_at_ms,
                    row.heartbeat_interval_ms,
                    row.state,
                    row.updated_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn task_lease(&self, task_id: &str) -> AcResult<Option<TaskLeaseRow>> {
        self.connection
            .query_row(
                "SELECT task_id, worker_id, lease_epoch, expires_at_ms, heartbeat_interval_ms,
                        state, updated_at_ms FROM task_leases WHERE task_id=?1",
                params![task_id],
                |row| {
                    Ok(TaskLeaseRow {
                        task_id: row.get(0)?,
                        worker_id: row.get(1)?,
                        lease_epoch: row.get(2)?,
                        expires_at_ms: row.get(3)?,
                        heartbeat_interval_ms: row.get(4)?,
                        state: row.get(5)?,
                        updated_at_ms: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(db_error)
    }

    pub fn save_autonomy_mailbox_message(&self, row: &AutonomyMailboxMessageRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO autonomy_mailbox_messages VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    row.id,
                    row.mission_id,
                    row.sender_worker_id,
                    row.recipient_worker_id,
                    row.message_type,
                    row.subject_id,
                    row.payload,
                    if row.delivered { 1_i64 } else { 0_i64 },
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn autonomy_mailbox_messages(
        &self,
        mission_id: &str,
    ) -> AcResult<Vec<AutonomyMailboxMessageRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, sender_worker_id, recipient_worker_id, message_type,
                    subject_id, payload, delivered, created_at_ms
             FROM autonomy_mailbox_messages WHERE mission_id=?1 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map([mission_id], |row| {
                Ok(AutonomyMailboxMessageRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    sender_worker_id: row.get(2)?,
                    recipient_worker_id: row.get(3)?,
                    message_type: row.get(4)?,
                    subject_id: row.get(5)?,
                    payload: row.get(6)?,
                    delivered: row.get::<_, i64>(7)? != 0,
                    created_at_ms: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn save_autonomy_record(&self, row: &AutonomyRecordRow) -> AcResult<()> {
        self.connection
            .execute(
                "INSERT INTO autonomy_records VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    row.id,
                    row.mission_id,
                    row.category,
                    row.subject_id,
                    row.payload,
                    row.created_at_ms
                ],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn autonomy_records(
        &self,
        mission_id: &str,
        category: &str,
    ) -> AcResult<Vec<AutonomyRecordRow>> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id, mission_id, category, subject_id, payload, created_at_ms
             FROM autonomy_records WHERE mission_id=?1 AND category=?2 ORDER BY created_at_ms ASC",
            )
            .map_err(db_error)?;
        let rows = stmt
            .query_map(params![mission_id, category], |row| {
                Ok(AutonomyRecordRow {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    category: row.get(2)?,
                    subject_id: row.get(3)?,
                    payload: row.get(4)?,
                    created_at_ms: row.get(5)?,
                })
            })
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }
}

fn memory_fact_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryFactRow> {
    Ok(MemoryFactRow {
        id: row.get(0)?,
        repository_id: row.get(1)?,
        mission_id: row.get(2)?,
        task_id: row.get(3)?,
        branch: row.get(4)?,
        statement: row.get(5)?,
        fact_type: row.get(6)?,
        source: row.get(7)?,
        confidence: row.get(8)?,
        freshness: row.get(9)?,
        memory_class: row.get(10)?,
        observed_commit: row.get(11)?,
        conflict_set_id: row.get(12)?,
        valid_from_ms: row.get(13)?,
        valid_until_ms: row.get(14)?,
        superseded_by: row.get(15)?,
        last_validation_ms: row.get(16)?,
    })
}

fn millis(ts: TimestampMillis) -> i64 {
    ts.as_millis().min(i64::MAX as u128) as i64
}

fn stable_ids_csv(ids: &[StableId]) -> String {
    ids.iter()
        .map(StableId::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn mission_state(state: MissionState) -> &'static str {
    match state {
        MissionState::Created => "created",
        MissionState::Active => "active",
        MissionState::Completed => "completed",
        MissionState::Cancelled => "cancelled",
    }
}

fn decision_kind(kind: KernelDecisionKind) -> &'static str {
    match kind {
        KernelDecisionKind::CreateMission => "create_mission",
        KernelDecisionKind::ActivateMission => "activate_mission",
        KernelDecisionKind::CompleteMission => "complete_mission",
        KernelDecisionKind::CancelMission => "cancel_mission",
        KernelDecisionKind::ApproveChangeSet => "approve_changeset",
    }
}

fn db_error(error: rusqlite::Error) -> AcError {
    AcError::new(
        "DB-SQLITE",
        error.to_string(),
        ac_common::ErrorKind::Internal,
        ac_common::Retryability::NotRetryable,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_changeset::{ChangeOperation, ChangeSet};
    use ac_git::GitCoordinator;
    use ac_kernel::{AllowAllPolicy, Kernel};
    use std::collections::BTreeSet;
    use std::fs;
    use std::process::Command;

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn sqlite_store_persists_kernel_state() {
        let mut db = ControlPlaneDb::open_memory().unwrap();
        db.migrate().unwrap();
        assert_eq!(db.user_version().unwrap(), 12);

        let mut kernel = Kernel::new(AllowAllPolicy);
        kernel.start().unwrap();
        let mission_id = kernel.create_mission("persist me").unwrap();
        let mission = kernel.mission(&mission_id).unwrap();
        db.put_mission(mission).unwrap();
        for event in kernel.events() {
            db.append_kernel_event(event).unwrap();
        }

        let persisted = db.get_mission(&mission_id).unwrap().unwrap();
        assert_eq!(persisted.original_goal, "persist me");
        assert_eq!(persisted.state, "created");
    }

    #[test]
    fn sqlite_file_survives_reopen_with_event_history() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let mission_id;
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let mut kernel = Kernel::new(AllowAllPolicy);
            kernel.start().unwrap();
            mission_id = kernel.create_mission("durable goal").unwrap();
            db.put_mission(kernel.mission(&mission_id).unwrap())
                .unwrap();
            for event in kernel.events() {
                db.append_kernel_event(event).unwrap();
            }
            assert_eq!(db.kernel_event_count().unwrap(), 1);
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let mission = db.get_mission(&mission_id).unwrap().unwrap();
            assert_eq!(mission.original_goal, "durable goal");
            assert_eq!(db.kernel_event_count().unwrap(), 1);
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn interrupted_sessions_are_recovered_after_reopen() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let session_id = StableId::new("session");
        let mission_id = StableId::new("mission");
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_session(&session_id, &mission_id, "executing")
                .unwrap();
            db.save_checkpoint(&StableId::new("cp"), &session_id, 2, "executing")
                .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let interrupted = db.interrupted_sessions().unwrap();
            assert_eq!(interrupted.len(), 1);
            assert_eq!(interrupted[0].id, session_id.to_string());
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn lifecycle_state_recovers_after_reopen() {
        let db_path =
            std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let source = std::env::temp_dir().join(format!("agentcode-src-{}", StableId::new("tmp")));
        let worktree_path =
            std::env::temp_dir().join(format!("agentcode-wt-{}", StableId::new("tmp")));
        let _ = fs::remove_file(&db_path);
        let _ = fs::remove_dir_all(&source);
        let _ = fs::remove_dir_all(&worktree_path);
        fs::create_dir_all(source.join("src")).unwrap();
        fs::write(source.join("src/lib.rs"), "pub fn answer() -> u32 { 41 }\n").unwrap();
        run_git(&source, ["init"]);
        run_git(&source, ["add", "."]);
        run_git(
            &source,
            [
                "-c",
                "user.name=AgentCode Test",
                "-c",
                "user.email=agentcode@example.test",
                "commit",
                "-m",
                "initial",
            ],
        );

        let session_id = StableId::new("session");
        let mission_id = StableId::new("mission");
        let changeset_id;
        let worktree_id;
        let checkpoint_id;
        {
            let mut db = ControlPlaneDb::open(&db_path).unwrap();
            db.migrate().unwrap();
            db.save_session(&session_id, &mission_id, "executing")
                .unwrap();

            let mut git = GitCoordinator::new();
            worktree_id = git
                .create_task_workspace(
                    source.clone(),
                    worktree_path.clone(),
                    mission_id.clone(),
                    session_id.clone(),
                )
                .unwrap();
            fs::write(
                worktree_path.join("src/lib.rs"),
                "pub fn answer() -> u32 { 42 }\n",
            )
            .unwrap();
            checkpoint_id = git.checkpoint_current(&worktree_id, "fix answer").unwrap();
            db.save_worktree(git.worktree(&worktree_id).unwrap())
                .unwrap();
            db.save_git_checkpoint(git.checkpoint_record(&checkpoint_id).unwrap())
                .unwrap();

            let mut changeset = ChangeSet::propose(
                vec![ChangeOperation::WriteFile {
                    path: "src/lib.rs".to_string(),
                    expected_hash: None,
                    new_hash: "len:29".to_string(),
                }],
                None,
            )
            .unwrap();
            changeset.validate().unwrap();
            changeset_id = changeset.id.clone();
            db.save_changeset(&changeset).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&db_path).unwrap();
            let session = db.get_session(&session_id).unwrap().unwrap();
            let worktree = db.get_worktree(&worktree_id).unwrap().unwrap();
            let checkpoints = db.git_checkpoints_for_worktree(&worktree_id).unwrap();
            let changeset = db.get_changeset(&changeset_id).unwrap().unwrap();

            assert_eq!(session.state, "executing");
            assert_eq!(worktree.owner_mission_id, mission_id.to_string());
            assert_eq!(checkpoints[0].commit_ref, worktree.current_commit);
            assert_eq!(checkpoints[0].reason, "fix answer");
            assert_eq!(checkpoints[0].id, checkpoint_id.to_string());
            assert_eq!(changeset.state, "Validated");
            assert!(changeset.operations_json.contains("src/lib.rs"));
        }
        let _ = fs::remove_file(db_path);
        let _ = fs::remove_dir_all(worktree_path);
        let _ = fs::remove_dir_all(source);
    }

    #[test]
    fn routing_decision_evidence_survives_reopen_without_secrets() {
        let path = std::env::temp_dir().join(format!("agentcode-{}.sqlite", StableId::new("db")));
        let decision = RoutingDecisionRecord {
            id: StableId::new("routing").to_string(),
            task_id: StableId::new("task").to_string(),
            candidates_json: "[{\"connection\":\"free\",\"score\":90}]".to_string(),
            selected_json: Some("{\"connection\":\"free\"}".to_string()),
            rejected_json: "[\"paid_disallowed\"]".to_string(),
            fallback_reason: Some("connection-1:RateLimit".to_string()),
            latency_ms: 12,
            input_tokens: 7,
            output_tokens: 11,
            estimated_cost_micros: 0,
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_routing_decision(&decision).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let loaded = db.routing_decision(&decision.id).unwrap().unwrap();
            assert_eq!(loaded.task_id, decision.task_id);
            assert_eq!(loaded.output_tokens, 11);
            assert!(!format!("{:?}", loaded).contains("SECRET"));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tool_output_evidence_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-tool-{}.sqlite", StableId::new("db")));
        let record = ToolExecutionRecord {
            id: StableId::new("toolrun").to_string(),
            tool_call_id: StableId::new("toolreq").to_string(),
            tool_id: "cmd.exec".to_string(),
            status: "Succeeded".to_string(),
            manifest_json: "argv:/usr/bin/env".to_string(),
            raw_output: "status:0\nstdout:ok".to_string(),
            evidence_ref: StableId::new("ev").to_string(),
            created_at_ms: millis(TimestampMillis::now()) as u128,
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_tool_execution(&record).unwrap();
        }
        let db = ControlPlaneDb::open(&path).unwrap();
        let loaded = db.tool_execution(&record.id).unwrap().unwrap();
        assert_eq!(loaded.raw_output, record.raw_output);
        assert_eq!(loaded.evidence_ref, record.evidence_ref);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn semantic_graph_evidence_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-semantic-{}.sqlite", StableId::new("db")));
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_semantic_index(
                "repo-1",
                "abc",
                "wt-1",
                &[(
                    "lsp-1".to_string(),
                    "typescript".to_string(),
                    "/repo".to_string(),
                    None,
                    "running".to_string(),
                    0,
                )],
                &[(
                    "api_route".to_string(),
                    "app/api/users/route.ts".to_string(),
                    "/api/users".to_string(),
                    "db/schema.sql".to_string(),
                    "users".to_string(),
                    "api-route-adapter".to_string(),
                    70,
                )],
                &[(
                    "app/api/users/route.ts".to_string(),
                    1,
                    "warning".to_string(),
                    "sample diagnostic".to_string(),
                    70,
                )],
                &[(
                    "npm-package".to_string(),
                    "apps/web".to_string(),
                    Some("web".to_string()),
                    "apps/web/package.json".to_string(),
                )],
                &[(
                    "scip".to_string(),
                    false,
                    "optional until benchmark threshold".to_string(),
                    3,
                    1,
                )],
            )
            .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(db.semantic_edge_count("repo-1").unwrap(), 1);
            assert_eq!(db.workspace_boundary_count("repo-1").unwrap(), 1);
            assert_eq!(
                db.optional_index_enabled("repo-1", "scip").unwrap(),
                Some(false)
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn persistent_memory_survives_reopen_and_source_changes_stale_facts() {
        let path =
            std::env::temp_dir().join(format!("agentcode-memory-{}.sqlite", StableId::new("db")));
        let fact = MemoryFactRow {
            id: StableId::new("mem").to_string(),
            repository_id: "repo-1".to_string(),
            mission_id: Some("mission-1".to_string()),
            task_id: Some("task-1".to_string()),
            branch: Some("main".to_string()),
            statement: "A calls B".to_string(),
            fact_type: "MODULE_RELATIONSHIP".to_string(),
            source: "LSP".to_string(),
            confidence: 90,
            freshness: "FRESH".to_string(),
            memory_class: "LONG_LIVED_REPO".to_string(),
            observed_commit: "abc".to_string(),
            conflict_set_id: None,
            valid_from_ms: millis(TimestampMillis::now()),
            valid_until_ms: None,
            superseded_by: None,
            last_validation_ms: millis(TimestampMillis::now()),
        };
        let evidence = MemoryEvidenceRow {
            fact_id: fact.id.clone(),
            evidence_ref: "ev-1".to_string(),
            file_path: Some("src/a.rs".to_string()),
            symbol: Some("A".to_string()),
            content_hash: Some("h1".to_string()),
        };
        let decision = MemoryDecisionRow {
            id: StableId::new("decision").to_string(),
            repository_id: "repo-1".to_string(),
            mission_id: Some("mission-1".to_string()),
            task_id: None,
            branch: Some("main".to_string()),
            decision: "Keep CONTEXT.md derived".to_string(),
            rationale: "Summaries are not authority".to_string(),
            authority_refs: "ev-1".to_string(),
            supersedes: None,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let task_memory = TaskMemoryRow {
            id: StableId::new("taskmem").to_string(),
            task_id: "task-1".to_string(),
            summary: "Investigated freshness fixture".to_string(),
            evidence_refs: "ev-1".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let snapshot = ContextSnapshotRow {
            id: StableId::new("snapshot").to_string(),
            repository_id: "repo-1".to_string(),
            mission_id: Some("mission-1".to_string()),
            task_id: Some("task-1".to_string()),
            branch: Some("main".to_string()),
            reason: "replacement-agent".to_string(),
            content: "# CONTEXT.md\nnot authority over current repository".to_string(),
            source_fact_ids: fact.id.clone(),
            decision_refs: decision.id.clone(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_memory_fact(&fact, &[evidence]).unwrap();
            db.save_memory_decision(&decision).unwrap();
            db.save_task_memory(&task_memory).unwrap();
            db.save_context_snapshot(&snapshot).unwrap();
            assert_eq!(
                db.mark_memory_for_source_change("src/a.rs", Some("A"), false)
                    .unwrap(),
                1
            );
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let loaded = db.memory_fact(&fact.id).unwrap().unwrap();
            assert_eq!(loaded.freshness, "POSSIBLY_STALE");
            assert_eq!(db.memory_fact_evidence(&fact.id).unwrap().len(), 1);
            assert_eq!(db.memory_decision_count("repo-1").unwrap(), 1);
            assert_eq!(db.task_memory_count("task-1").unwrap(), 1);
            assert!(db
                .context_snapshot(&snapshot.id)
                .unwrap()
                .unwrap()
                .content
                .contains("not authority"));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn context_engine_receipts_survive_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-context-{}.sqlite", StableId::new("db")));
        let manifest = ContextPackManifestRow {
            id: StableId::new("ctxmanifest").to_string(),
            pack_id: StableId::new("ctx").to_string(),
            task_id: "task-11".to_string(),
            role: "WORKER".to_string(),
            profile: "NORMAL".to_string(),
            source_fragment_ids: "ctxfrag-1,ctxfrag-2".to_string(),
            omitted_fragment_ids: "ctxfrag-3".to_string(),
            raw_evidence_refs: "raw-1".to_string(),
            cache_keys: "commit:abc:src/lib.rs".to_string(),
            score_trace: "ctxfrag-1:TARGET_SOURCE:195".to_string(),
            total_input_tokens: 320,
            hard_ceiling: 500,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let compression = ContextCompressionReceiptRow {
            id: StableId::new("ctxcompress").to_string(),
            raw_evidence_ref: "raw-1".to_string(),
            command_class: "tests".to_string(),
            compressor_id: "agentcode-rtk-fallback-v1".to_string(),
            raw_hash: "hash".to_string(),
            compressed_output: "error: failed assertion".to_string(),
            raw_token_estimate: 80,
            compressed_token_estimate: 12,
            omitted_lines: 9,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let cache = ContextCacheEntryRow {
            cache_key: "commit:abc:src/lib.rs".to_string(),
            content_hash: "hash".to_string(),
            token_estimate: 120,
            source_ref: "src-lib".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let retrieval = ContextRetrievalRecordRow {
            id: StableId::new("ctxret").to_string(),
            pack_id: manifest.pack_id.clone(),
            need: "need_related_tests".to_string(),
            reason: "worker requested tests".to_string(),
            query: "auth".to_string(),
            result_fragment_ids: "ctxfrag-2".to_string(),
            added_tokens: 30,
            degraded: false,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let metrics = ContextPackMetricsRow {
            id: StableId::new("ctxmetric").to_string(),
            pack_id: manifest.pack_id.clone(),
            role: "WORKER".to_string(),
            selected_fragments: 2,
            omitted_fragments: 1,
            total_input_tokens: 320,
            budget_target: 400,
            hard_ceiling: 500,
            deduped_fragments: 1,
            redacted_fragments: 1,
            retrieval_steps: 1,
            cache_hits: 1,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let benchmark = ContextBenchmarkResultRow {
            id: StableId::new("ctxbench").to_string(),
            task_name: "cross-module bug".to_string(),
            broad_tokens: 1_200,
            targeted_tokens: 320,
            broad_success: true,
            targeted_success: true,
            retry_delta: 0,
            latency_delta_ms: -12,
            passed: true,
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_context_pack_manifest(&manifest).unwrap();
            db.save_context_compression_receipt(&compression).unwrap();
            db.save_context_cache_entry(&cache).unwrap();
            db.save_context_retrieval_record(&retrieval).unwrap();
            db.save_context_pack_metrics(&metrics).unwrap();
            db.save_context_benchmark_result(&benchmark).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(
                db.context_pack_manifest(&manifest.id).unwrap().unwrap(),
                manifest
            );
            assert_eq!(
                db.context_compression_receipt(&compression.id)
                    .unwrap()
                    .unwrap()
                    .raw_evidence_ref,
                "raw-1"
            );
            assert_eq!(
                db.context_cache_entry(&cache.cache_key)
                    .unwrap()
                    .unwrap()
                    .source_ref,
                "src-lib"
            );
            assert_eq!(
                db.context_retrieval_records(&metrics.pack_id).unwrap()[0].need,
                "need_related_tests"
            );
            assert_eq!(
                db.context_pack_metrics(&metrics.pack_id)
                    .unwrap()
                    .unwrap()
                    .deduped_fragments,
                1
            );
            assert!(
                db.context_benchmark_result(&benchmark.id)
                    .unwrap()
                    .unwrap()
                    .passed
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase12_autonomy_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p12-{}.sqlite", StableId::new("db")));
        let contract = MissionContractRevisionRow {
            id: StableId::new("contract").to_string(),
            mission_id: "mission-p12".to_string(),
            revision: 1,
            original_goal: "complete long mission".to_string(),
            reason: "initial extraction".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let requirement = RequirementMatrixEntryRow {
            id: StableId::new("req").to_string(),
            mission_id: contract.mission_id.clone(),
            contract_revision: 1,
            description: "scheduler only runs ready tasks".to_string(),
            requirement_type: "FUNCTIONAL".to_string(),
            priority: 100,
            source: "original_goal".to_string(),
            verification_strategy: "unit test".to_string(),
            blocking: true,
            implementation_status: "IMPLEMENTED".to_string(),
            verification_status: "VERIFIED".to_string(),
            evidence_refs: "ev-1".to_string(),
            linked_task_ids: "task-1".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        let lease = TaskLeaseRow {
            task_id: "task-1".to_string(),
            worker_id: "worker-1".to_string(),
            lease_epoch: 1,
            expires_at_ms: millis(TimestampMillis::now()) + 5000,
            heartbeat_interval_ms: 1000,
            state: "active".to_string(),
            updated_at_ms: millis(TimestampMillis::now()),
        };
        let message = AutonomyMailboxMessageRow {
            id: StableId::new("msg").to_string(),
            mission_id: contract.mission_id.clone(),
            sender_worker_id: "worker-1".to_string(),
            recipient_worker_id: Some("verifier-1".to_string()),
            message_type: "FINDING".to_string(),
            subject_id: Some("task-1".to_string()),
            payload: "review diff".to_string(),
            delivered: false,
            created_at_ms: millis(TimestampMillis::now()),
        };
        let record = AutonomyRecordRow {
            id: StableId::new("autonomy").to_string(),
            mission_id: contract.mission_id.clone(),
            category: "RECOVERY".to_string(),
            subject_id: Some("task-1".to_string()),
            payload: "provider failure -> switch route".to_string(),
            created_at_ms: millis(TimestampMillis::now()),
        };
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            db.save_mission_contract_revision(&contract).unwrap();
            db.save_requirement_matrix_entry(&requirement).unwrap();
            db.save_task_lease(&lease).unwrap();
            db.save_autonomy_mailbox_message(&message).unwrap();
            db.save_autonomy_record(&record).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(
                db.mission_contract_revisions(&contract.mission_id)
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                db.requirement_matrix_entries(&contract.mission_id).unwrap()[0].verification_status,
                "VERIFIED"
            );
            assert_eq!(
                db.task_lease(&lease.task_id).unwrap().unwrap().worker_id,
                "worker-1"
            );
            assert_eq!(
                db.autonomy_mailbox_messages(&contract.mission_id).unwrap()[0].message_type,
                "FINDING"
            );
            assert_eq!(
                db.autonomy_records(&contract.mission_id, "RECOVERY")
                    .unwrap()[0]
                    .payload,
                "provider failure -> switch route"
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase13_edit_transactions_survive_reopen_and_expose_recovery_rows() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p13-{}.sqlite", StableId::new("db")));
        let mut repo = ac_changeset::MemoryFileRepository::new("rev-p13");
        repo.put("src/lib.rs", "pub fn answer() -> u32 { 41 }\n");
        let request = ac_changeset::EditRequest {
            path: "src/lib.rs".to_string(),
            precondition: ac_changeset::EditPrecondition {
                path: "src/lib.rs".to_string(),
                expected_hash: repo.hash("src/lib.rs").unwrap(),
                base_revision: "rev-p13".to_string(),
                symbol_fingerprint: None,
            },
            strategy: ac_changeset::EditStrategy::SearchReplace {
                search: "41".to_string(),
                replace: "42".to_string(),
                expected_matches: 1,
            },
        };
        let mut transaction = ac_changeset::EditEngine
            .prepare(&repo, vec![request])
            .unwrap();
        ac_changeset::EditEngine
            .apply(&mut repo, &mut transaction)
            .unwrap();
        transaction.changeset.mark_validating().unwrap();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            db.save_changeset_transaction(
                &transaction,
                Some("task-p13"),
                Some("worktree-p13"),
                "rev-p13",
                "rust",
            )
            .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let saved = db
                .edit_transaction(&transaction.id.to_string())
                .unwrap()
                .unwrap();
            assert_eq!(saved.state, "Validating");
            assert_eq!(
                db.edit_journal_entries(&transaction.id.to_string())
                    .unwrap()[0]
                    .state,
                "Applied"
            );
            assert_eq!(
                db.edit_strategy_metrics(&transaction.id.to_string())
                    .unwrap()[0]
                    .strategy,
                "SearchReplace"
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase14_verification_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p14-{}.sqlite", StableId::new("db")));
        let engine = ac_verification::VerificationEngine::new(ac_security::CapabilityPolicy::new());
        let mut evidence = ac_evidence::EvidenceStore::new();
        let profile = engine.derive_profile(
            StableId::new("task"),
            ac_verification::VerificationRisk::High,
            &ac_verification::ProjectCapabilities {
                cargo: true,
                makefile: false,
                package_json: false,
                browser: false,
                security: false,
            },
        );
        let requirement_id = StableId::new("req");
        let manifest = engine
            .record_evidence_manifest(
                "commit-p14",
                "worktree-p14",
                "cargo test --workspace",
                ac_verification::GateStatus::Passed,
                vec![requirement_id.clone()],
                vec!["src/lib.rs".to_string()],
                &mut evidence,
            )
            .unwrap();
        let link = engine.link_requirement_evidence(requirement_id.clone(), &manifest);
        let audit = engine
            .final_audit(
                ac_verification::FinalAuditInput {
                    original_goal: "finish phase 14".to_string(),
                    requirements: vec!["verification evidence exists".to_string()],
                    verified_requirement_ids: vec![requirement_id.clone()],
                    evidence_refs: vec![manifest.evidence_ref.clone()],
                    worker_completion_text: "verified".to_string(),
                    unresolved_limitations: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            db.save_verification_profile(&profile).unwrap();
            db.save_verification_manifest(
                &manifest,
                Some(profile.id.as_str()),
                Some(profile.task_id.as_str()),
            )
            .unwrap();
            db.save_requirement_verification(manifest.id.as_str(), &link)
                .unwrap();
            db.save_final_audit(
                "mission-p14",
                "finish phase 14",
                &["verification evidence exists".to_string()],
                &audit,
                true,
            )
            .unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            assert_eq!(
                db.verification_run(manifest.id.as_str())
                    .unwrap()
                    .unwrap()
                    .normalized_result,
                "Passed"
            );
            assert!(
                db.requirement_verifications(requirement_id.as_str())
                    .unwrap()[0]
                    .verified
            );
            assert!(db.final_audits("mission-p14").unwrap()[0].completion_allowed);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase15_browser_runtime_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-p15-{}.sqlite", StableId::new("db")));
        let mut runtime = ac_verification::BrowserRuntime::new(
            ac_security::CapabilityPolicy::new().allow(ac_security::Capability::BrowserAutomation),
        );
        let mut evidence = ac_evidence::EvidenceStore::new();
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime
            .create_session(task.clone(), process.id.clone())
            .unwrap();
        let dev_server = runtime
            .manage_dev_server(
                task.clone(),
                vec!["npm".to_string(), "run".to_string(), "dev".to_string()],
                3000,
                "http://127.0.0.1:3000/login",
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                ac_verification::BrowserAction::Open {
                    url: dev_server.ready_url.clone(),
                    html: "<h1>Login</h1><button id=\"submit\">Submit</button>".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        let screenshot = runtime
            .capture_screenshot(
                &session.id,
                task.clone(),
                "commit-p15",
                runtime.default_viewports()[2],
                &mut evidence,
            )
            .unwrap();
        let visual = runtime.visual_qa(&screenshot, &mut evidence).unwrap();
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            db.save_browser_process(&process).unwrap();
            db.save_browser_session(&session).unwrap();
            db.save_browser_dev_server(&dev_server).unwrap();
            db.save_browser_screenshot(&screenshot).unwrap();
            db.save_browser_visual_qa(&visual).unwrap();
        }
        {
            let db = ControlPlaneDb::open(&path).unwrap();
            let loaded = db.browser_session(session.id.as_str()).unwrap().unwrap();
            assert_eq!(loaded.task_id, task.to_string());
            assert!(loaded.sensitive);
            let screenshots = db.browser_screenshots(session.id.as_str()).unwrap();
            assert_eq!(screenshots[0].viewport, "desktop");
            assert_eq!(
                screenshots[0].evidence_ref,
                screenshot.evidence_ref.to_string()
            );
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase16_extension_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-phase16-{}.sqlite", StableId::new("db")));
        let skill_id = StableId::new("skill");
        let hook_id = StableId::new("hook");
        let server_id = StableId::new("mcp");
        let tool_id = StableId::new("mcptool");
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            let skill = SkillManifest {
                id: skill_id.clone(),
                name: "Rust".to_string(),
                description: "Rust testing".to_string(),
                version: "1".to_string(),
                source: "builtin/rust".to_string(),
                scope: ac_security::SkillScope::BuiltIn,
                trust_tier: ac_security::TrustTier::BuiltIn,
                trigger_hints: vec!["rust".to_string()],
                required_capabilities: [ac_security::Capability::ProcessExec("*".to_string())]
                    .into_iter()
                    .collect(),
                context_cost: 7,
                project_id: None,
                task_id: None,
                full_instructions: "Run cargo tests.".to_string(),
                loaded_at: Some(TimestampMillis::now()),
            };
            db.save_skill(&skill).unwrap();
            let hook = HookManifest {
                id: hook_id.clone(),
                extension_id: StableId::new("ext"),
                event: ac_security::HookEvent::BeforeTaskComplete,
                priority: 1,
                timeout_ms: 50,
                idempotency_key: "before-complete".to_string(),
                failure_policy: ac_security::HookFailurePolicy::BlockOperation,
                required_capabilities: BTreeSet::new(),
            };
            db.save_hook_manifest(&hook).unwrap();
            db.save_hook_invocation(&HookInvocation {
                id: StableId::new("hookrun"),
                hook_id: hook_id.clone(),
                event: ac_security::HookEvent::BeforeTaskComplete,
                outcome: ac_security::HookOutcome::Blocked,
                evidence_ref: Some(StableId::new("ev")),
                created_at: TimestampMillis::now(),
            })
            .unwrap();
            db.save_mcp_server(&McpServerRecord {
                id: server_id.clone(),
                name: "fixture".to_string(),
                version: "1".to_string(),
                transport: ac_security::McpTransport::Stdio,
                trust_tier: ac_security::TrustTier::Project,
                health: ac_security::McpHealth::Connected,
                restart_count: 1,
            })
            .unwrap();
            db.save_mcp_tool(&McpToolRecord {
                id: tool_id.clone(),
                server_id: server_id.clone(),
                name: "read".to_string(),
                description: "Read fixture".to_string(),
                schema: "{}".to_string(),
                risk: ac_security::RiskClass::R1,
                required_capabilities: [ac_security::Capability::FilesystemRead("*".to_string())]
                    .into_iter()
                    .collect(),
            })
            .unwrap();
            db.save_mcp_invocation(&McpInvocationRecord {
                id: StableId::new("mcpinvoke"),
                server_id: server_id.clone(),
                tool_id,
                status: ac_security::SecurityDecision::Allow,
                output: "structured output".to_string(),
                evidence_ref: StableId::new("ev"),
                created_at: TimestampMillis::now(),
            })
            .unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let loaded_skill = db.skill(skill_id.as_str()).unwrap().unwrap();
            assert_eq!(loaded_skill.scope, "BuiltIn");
            assert!(loaded_skill.full_instructions.contains("cargo tests"));
            let hook_runs = db.hook_invocations(hook_id.as_str()).unwrap();
            assert_eq!(hook_runs[0].outcome, "Blocked");
            let server = db.mcp_server(server_id.as_str()).unwrap().unwrap();
            assert_eq!(server.health, "Connected");
            assert_eq!(server.restart_count, 1);
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn phase17_security_scan_state_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("agentcode-phase17-{}.sqlite", StableId::new("db")));
        let repo_id = StableId::new("repo");
        let scan_id;
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            assert_eq!(db.user_version().unwrap(), 12);
            let orchestrator = ac_security::BaselineSecurityOrchestrator::new(
                ac_security::SecurityPolicy::baseline(),
            );
            let input = ac_security::SecurityScanInput {
                repository_id: repo_id.clone(),
                commit: "abc123".to_string(),
                files: vec![(
                    "src/api.rs".to_string(),
                    "fn handler() { let token = \"SECRET=value\"; }".to_string(),
                )],
                dependency_manifest: Some("vulnerable-package = \"0.1.0\"".to_string()),
                include_iac: false,
            };
            let report = orchestrator.run(&input).unwrap();
            let bundle = orchestrator.reports(&report);
            scan_id = report.id.clone();
            db.save_security_scan(&input, &report).unwrap();
            db.save_security_reports(&report.id, &bundle).unwrap();
        }
        {
            let mut db = ControlPlaneDb::open(&path).unwrap();
            db.migrate().unwrap();
            let scan = db.security_scan(scan_id.as_str()).unwrap().unwrap();
            assert_eq!(scan.repository_id, repo_id.to_string());
            let findings = db.security_findings(scan_id.as_str()).unwrap();
            assert!(findings
                .iter()
                .any(|finding| finding.root_cause == "secret-exposure"));
            assert!(findings
                .iter()
                .any(|finding| finding.root_cause == "vulnerable-dependency"));
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn future_schema_version_is_rejected() {
        let mut db = ControlPlaneDb::open_memory().unwrap();
        db.connection
            .pragma_update(None, "user_version", 99)
            .unwrap();
        let error = db.migrate().unwrap_err();
        assert_eq!(error.code(), "DB-FUTURE_VERSION");
    }
}
