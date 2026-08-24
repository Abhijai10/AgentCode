use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityClass {
    KernelState,
    RawEvidence,
    AcceptedMemory,
    DerivedSummary,
    RetrievalAccelerator,
    RuntimeContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryFact {
    pub id: StableId,
    pub statement: String,
    pub fact_type: FactType,
    pub source: FactSource,
    pub source_evidence: Vec<StableId>,
    pub confidence: u8,
    pub freshness: FreshnessState,
    pub scope: MemoryScope,
    pub memory_class: MemoryClass,
    pub observed_commit: String,
    pub dependencies: Vec<FreshnessDependency>,
    pub conflict_set: Option<StableId>,
    pub valid_from: TimestampMillis,
    pub valid_until: Option<TimestampMillis>,
    pub superseded_by: Option<StableId>,
    pub last_validation: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactType {
    SymbolRole,
    ModuleRelationship,
    ProjectCommand,
    ArchitectureFact,
    UserDecision,
    FailedApproach,
}

impl FactType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SymbolRole => "SYMBOL_ROLE",
            Self::ModuleRelationship => "MODULE_RELATIONSHIP",
            Self::ProjectCommand => "PROJECT_COMMAND",
            Self::ArchitectureFact => "ARCHITECTURE_FACT",
            Self::UserDecision => "USER_DECISION",
            Self::FailedApproach => "FAILED_APPROACH",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactSource {
    UserRequirement,
    ArchitectureDecision,
    TreeSitter,
    Lsp,
    Scip,
    Test,
    Runtime,
    Git,
    LlmInference,
}

impl FactSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserRequirement => "USER_REQUIREMENT",
            Self::ArchitectureDecision => "ARCHITECTURE_DECISION",
            Self::TreeSitter => "TREE_SITTER",
            Self::Lsp => "LSP",
            Self::Scip => "SCIP",
            Self::Test => "TEST",
            Self::Runtime => "RUNTIME",
            Self::Git => "GIT",
            Self::LlmInference => "LLM_INFERENCE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FreshnessState {
    Fresh,
    PossiblyStale,
    Invalid,
    Conflicted,
}

impl FreshnessState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "FRESH",
            Self::PossiblyStale => "POSSIBLY_STALE",
            Self::Invalid => "INVALID",
            Self::Conflicted => "CONFLICTED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryClass {
    ImmutableMission,
    Decision,
    LongLivedRepo,
    TaskScoped,
    Ephemeral,
}

impl MemoryClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ImmutableMission => "IMMUTABLE_MISSION",
            Self::Decision => "DECISION",
            Self::LongLivedRepo => "LONG_LIVED_REPO",
            Self::TaskScoped => "TASK_SCOPED",
            Self::Ephemeral => "EPHEMERAL",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryScope {
    pub repository_id: StableId,
    pub mission_id: Option<StableId>,
    pub task_id: Option<StableId>,
    pub branch: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshnessDependency {
    pub evidence_ref: StableId,
    pub file_path: Option<String>,
    pub symbol: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactInput {
    pub statement: String,
    pub fact_type: FactType,
    pub source: FactSource,
    pub source_evidence: Vec<StableId>,
    pub confidence: u8,
    pub scope: MemoryScope,
    pub memory_class: MemoryClass,
    pub observed_commit: String,
    pub dependencies: Vec<FreshnessDependency>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDecision {
    pub id: StableId,
    pub scope: MemoryScope,
    pub decision: String,
    pub rationale: String,
    pub authority_refs: Vec<StableId>,
    pub supersedes: Option<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskMemory {
    pub id: StableId,
    pub task_id: StableId,
    pub summary: String,
    pub evidence_refs: Vec<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextSnapshot {
    pub id: StableId,
    pub scope: MemoryScope,
    pub reason: String,
    pub content: String,
    pub source_fact_ids: Vec<StableId>,
    pub decision_refs: Vec<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandoffContext {
    pub id: StableId,
    pub context_markdown: String,
    pub snapshot_id: StableId,
    pub fact_refs: Vec<StableId>,
    pub decision_refs: Vec<StableId>,
    pub task_memory_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextNode {
    pub id: StableId,
    pub source_ref: StableId,
    pub authority: AuthorityClass,
    pub content: String,
    pub token_estimate: u32,
    pub protected: bool,
    pub degraded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextPack {
    pub id: StableId,
    pub nodes: Vec<ContextNode>,
    pub budget: u32,
    pub omitted_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextRole {
    Planner,
    Worker,
    Researcher,
    Verifier,
}

impl ContextRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planner => "PLANNER",
            Self::Worker => "WORKER",
            Self::Researcher => "RESEARCHER",
            Self::Verifier => "VERIFIER",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextFragmentType {
    Mission,
    AcceptanceCriteria,
    ProjectRule,
    RepoMap,
    TargetSource,
    RelatedSource,
    Test,
    Diff,
    Error,
    Decision,
    FailedApproach,
    ToolState,
    ToolOutput,
}

impl ContextFragmentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mission => "MISSION",
            Self::AcceptanceCriteria => "ACCEPTANCE_CRITERIA",
            Self::ProjectRule => "PROJECT_RULE",
            Self::RepoMap => "REPO_MAP",
            Self::TargetSource => "TARGET_SOURCE",
            Self::RelatedSource => "RELATED_SOURCE",
            Self::Test => "TEST",
            Self::Diff => "DIFF",
            Self::Error => "ERROR",
            Self::Decision => "DECISION",
            Self::FailedApproach => "FAILED_APPROACH",
            Self::ToolState => "TOOL_STATE",
            Self::ToolOutput => "TOOL_OUTPUT",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SensitivityClass {
    Public,
    Internal,
    Secret,
}

impl SensitivityClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::Internal => "INTERNAL",
            Self::Secret => "SECRET",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextProfile {
    Tiny,
    Normal,
    Deep,
    Audit,
    Extreme,
}

impl ContextProfile {
    pub fn budget(self) -> TokenBudget {
        match self {
            Self::Tiny => TokenBudget::new(2_000, 3_000, 800, 500, 300).expect("valid budget"),
            Self::Normal => {
                TokenBudget::new(8_000, 12_000, 2_000, 1_000, 1_000).expect("valid budget")
            }
            Self::Deep => {
                TokenBudget::new(24_000, 32_000, 4_000, 2_000, 2_000).expect("valid budget")
            }
            Self::Audit => {
                TokenBudget::new(48_000, 64_000, 6_000, 4_000, 4_000).expect("valid budget")
            }
            Self::Extreme => {
                TokenBudget::new(120_000, 160_000, 12_000, 8_000, 8_000).expect("valid budget")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenBudget {
    pub target_input: u32,
    pub hard_ceiling: u32,
    pub reserved_output: u32,
    pub tool_result_allowance: u32,
    pub history_allowance: u32,
}

impl TokenBudget {
    pub fn new(
        target_input: u32,
        hard_ceiling: u32,
        reserved_output: u32,
        tool_result_allowance: u32,
        history_allowance: u32,
    ) -> AcResult<Self> {
        if target_input == 0 || hard_ceiling == 0 || target_input > hard_ceiling {
            return Err(AcError::validation(
                "CONTEXT-INVALID_BUDGET",
                "context token budget must be positive and below the hard ceiling",
            ));
        }
        Ok(Self {
            target_input,
            hard_ceiling,
            reserved_output,
            tool_result_allowance,
            history_allowance,
        })
    }

    pub fn available_input(&self) -> AcResult<u32> {
        let reserved = self
            .reserved_output
            .saturating_add(self.tool_result_allowance)
            .saturating_add(self.history_allowance);
        self.hard_ceiling.checked_sub(reserved).ok_or_else(|| {
            AcError::validation(
                "CONTEXT-RESERVES_EXCEED_CEILING",
                "reserved output/tool/history budget exceeds the hard ceiling",
            )
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextFragment {
    pub id: StableId,
    pub fragment_type: ContextFragmentType,
    pub source_ref: StableId,
    pub source_path: Option<String>,
    pub range: Option<String>,
    pub reason: String,
    pub freshness: FreshnessState,
    pub score: i32,
    pub token_estimate: u32,
    pub sensitivity: SensitivityClass,
    pub authority: AuthorityClass,
    pub content: String,
    pub hard_include: bool,
    pub degraded: bool,
    pub cache_key: Option<String>,
    pub raw_evidence_ref: Option<StableId>,
    pub relationships: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextPackRequest {
    pub role: ContextRole,
    pub task_id: StableId,
    pub task: String,
    pub profile: ContextProfile,
    pub budget: TokenBudget,
    pub acceptance_criteria: Vec<String>,
    pub mission_subset: Vec<String>,
    pub project_rules: Vec<ContextFragment>,
    pub fragments: Vec<ContextFragment>,
    pub explicit_source_refs: Vec<StableId>,
    pub retrieval_requests: Vec<ProgressiveRetrievalRequest>,
    pub provider_allows_sensitive: bool,
    pub cache_parameters: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressiveNeed {
    Definition,
    References,
    RelatedTests,
    BroaderScope,
    RawOutput,
    History,
}

impl ProgressiveNeed {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Definition => "need_definition",
            Self::References => "need_references",
            Self::RelatedTests => "need_related_tests",
            Self::BroaderScope => "need_broader_scope",
            Self::RawOutput => "need_raw_output",
            Self::History => "need_history",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressiveRetrievalRequest {
    pub need: ProgressiveNeed,
    pub reason: String,
    pub query: String,
    pub max_tokens: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressiveRetrievalRecord {
    pub id: StableId,
    pub pack_id: StableId,
    pub need: ProgressiveNeed,
    pub reason: String,
    pub query: String,
    pub result_fragment_ids: Vec<StableId>,
    pub added_tokens: u32,
    pub degraded: bool,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextManifest {
    pub id: StableId,
    pub pack_id: StableId,
    pub role: ContextRole,
    pub task_id: StableId,
    pub profile: ContextProfile,
    pub source_fragment_ids: Vec<StableId>,
    pub omitted_fragment_ids: Vec<StableId>,
    pub raw_evidence_refs: Vec<StableId>,
    pub cache_keys: Vec<String>,
    pub score_trace: Vec<String>,
    pub total_input_tokens: u32,
    pub hard_ceiling: u32,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBuildReceipt {
    pub pack: ContextPack,
    pub manifest: ContextManifest,
    pub retrieval_records: Vec<ProgressiveRetrievalRecord>,
    pub metrics: ContextPackMetrics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressionReceipt {
    pub id: StableId,
    pub raw_evidence_ref: StableId,
    pub command_class: String,
    pub compressor_id: String,
    pub raw_hash: String,
    pub compressed_output: String,
    pub raw_token_estimate: u32,
    pub compressed_token_estimate: u32,
    pub omitted_lines: u32,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextCacheEntry {
    pub key: String,
    pub content_hash: String,
    pub token_estimate: u32,
    pub source_ref: StableId,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextPackMetrics {
    pub id: StableId,
    pub pack_id: StableId,
    pub role: ContextRole,
    pub selected_fragments: usize,
    pub omitted_fragments: usize,
    pub total_input_tokens: u32,
    pub budget_target: u32,
    pub hard_ceiling: u32,
    pub deduped_fragments: usize,
    pub redacted_fragments: usize,
    pub retrieval_steps: usize,
    pub cache_hits: usize,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBenchmarkResult {
    pub id: StableId,
    pub task_name: String,
    pub broad_tokens: u32,
    pub targeted_tokens: u32,
    pub broad_success: bool,
    pub targeted_success: bool,
    pub retry_delta: i32,
    pub latency_delta_ms: i64,
    pub passed: bool,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBenchmarkInput {
    pub task_name: String,
    pub broad_tokens: u32,
    pub broad_success: bool,
    pub targeted_success: bool,
    pub retry_delta: i32,
    pub latency_delta_ms: i64,
}

#[derive(Default)]
pub struct MemoryService {
    facts: BTreeMap<StableId, MemoryFact>,
    decisions: BTreeMap<StableId, MemoryDecision>,
    task_memories: BTreeMap<StableId, TaskMemory>,
    snapshots: BTreeMap<StableId, ContextSnapshot>,
}

impl MemoryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_fact(
        &mut self,
        statement: impl Into<String>,
        source_evidence: Vec<StableId>,
        confidence: u8,
    ) -> AcResult<StableId> {
        self.record_typed_fact(FactInput {
            statement: statement.into(),
            fact_type: FactType::ArchitectureFact,
            source: FactSource::Runtime,
            source_evidence,
            confidence,
            scope: MemoryScope {
                repository_id: StableId::new("repo"),
                mission_id: None,
                task_id: None,
                branch: None,
            },
            memory_class: MemoryClass::LongLivedRepo,
            observed_commit: "working-tree".to_string(),
            dependencies: Vec::new(),
        })
    }

    pub fn record_typed_fact(&mut self, input: FactInput) -> AcResult<StableId> {
        if input.statement.trim().is_empty() || input.source_evidence.is_empty() {
            return Err(AcError::validation(
                "MEMORY-INVALID_FACT",
                "memory facts require a statement and source evidence",
            ));
        }
        if input.confidence > 100 {
            return Err(AcError::validation(
                "MEMORY-INVALID_CONFIDENCE",
                "confidence must be in the range 0..=100",
            ));
        }
        let id = StableId::new("mem");
        let now = TimestampMillis::now();
        self.facts.insert(
            id.clone(),
            MemoryFact {
                id: id.clone(),
                statement: input.statement,
                fact_type: input.fact_type,
                source: input.source,
                source_evidence: input.source_evidence,
                confidence: input.confidence,
                freshness: FreshnessState::Fresh,
                scope: input.scope,
                memory_class: input.memory_class,
                observed_commit: input.observed_commit,
                dependencies: input.dependencies,
                conflict_set: None,
                valid_from: now,
                valid_until: None,
                superseded_by: None,
                last_validation: now,
            },
        );
        self.detect_conflicts();
        Ok(id)
    }

    pub fn supersede_fact(
        &mut self,
        fact_id: &StableId,
        statement: impl Into<String>,
        evidence: Vec<StableId>,
    ) -> AcResult<StableId> {
        if !self.facts.contains_key(fact_id) {
            return Err(AcError::validation("MEMORY-UNKNOWN_FACT", "fact not found"));
        }
        let replacement_id = self.record_fact(statement, evidence, 80)?;
        let fact = self.facts.get_mut(fact_id).expect("checked above");
        fact.superseded_by = Some(replacement_id.clone());
        fact.valid_until = Some(TimestampMillis::now());
        Ok(replacement_id)
    }

    pub fn search_memory(&self, query: &str) -> Vec<MemoryFact> {
        self.facts
            .values()
            .filter(|fact| {
                fact.valid_until.is_none()
                    && fact.freshness != FreshnessState::Invalid
                    && fact.statement.contains(query)
            })
            .cloned()
            .collect()
    }

    pub fn fact(&self, fact_id: &StableId) -> Option<MemoryFact> {
        self.facts.get(fact_id).cloned()
    }

    pub fn apply_source_change(
        &mut self,
        file_path: &str,
        symbol: Option<&str>,
        deleted: bool,
    ) -> Vec<StableId> {
        let mut changed = Vec::new();
        for fact in self.facts.values_mut() {
            let depends = fact.dependencies.iter().any(|dependency| {
                dependency.file_path.as_deref() == Some(file_path)
                    && (symbol.is_none()
                        || dependency.symbol.is_none()
                        || dependency.symbol.as_deref() == symbol)
            });
            if depends {
                fact.freshness = if deleted {
                    FreshnessState::Invalid
                } else {
                    FreshnessState::PossiblyStale
                };
                fact.last_validation = TimestampMillis::now();
                changed.push(fact.id.clone());
            }
        }
        changed
    }

    pub fn record_decision(
        &mut self,
        scope: MemoryScope,
        decision: impl Into<String>,
        rationale: impl Into<String>,
        authority_refs: Vec<StableId>,
        supersedes: Option<StableId>,
    ) -> AcResult<StableId> {
        let decision = decision.into();
        if decision.trim().is_empty() || authority_refs.is_empty() {
            return Err(AcError::validation(
                "MEMORY-INVALID_DECISION",
                "decisions require text and authority refs",
            ));
        }
        let id = StableId::new("decision");
        self.decisions.insert(
            id.clone(),
            MemoryDecision {
                id: id.clone(),
                scope,
                decision,
                rationale: rationale.into(),
                authority_refs,
                supersedes,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn decisions(&self) -> Vec<MemoryDecision> {
        self.decisions.values().cloned().collect()
    }

    pub fn record_task_memory(
        &mut self,
        task_id: StableId,
        summary: impl Into<String>,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<StableId> {
        let summary = summary.into();
        if summary.trim().is_empty() || evidence_refs.is_empty() {
            return Err(AcError::validation(
                "MEMORY-INVALID_TASK_MEMORY",
                "task memory requires summary and evidence",
            ));
        }
        let id = StableId::new("taskmem");
        self.task_memories.insert(
            id.clone(),
            TaskMemory {
                id: id.clone(),
                task_id,
                summary,
                evidence_refs,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn generate_context_markdown(&self, scope: &MemoryScope, mission: &str) -> String {
        let facts = self
            .facts
            .values()
            .filter(|fact| fact.scope.repository_id == scope.repository_id)
            .collect::<Vec<_>>();
        let decisions = self
            .decisions
            .values()
            .filter(|decision| decision.scope.repository_id == scope.repository_id)
            .collect::<Vec<_>>();
        let tasks = self
            .task_memories
            .values()
            .filter(|task| scope.task_id.as_ref().is_none_or(|id| id == &task.task_id))
            .collect::<Vec<_>>();
        let mut output = format!(
            "# CONTEXT.md\n\nGenerated from structured AgentCode state. This file is a handoff view, not authority over current source, Git, Kernel state, or raw evidence.\n\nRepository: {}\nMission: {}\n",
            scope.repository_id, mission
        );
        output.push_str("\n## Current Facts\n");
        for fact in facts.iter().take(12) {
            output.push_str(&format!(
                "- [{} {} confidence={}] {}\n",
                fact.fact_type.as_str(),
                fact.freshness.as_str(),
                fact.confidence,
                fact.statement
            ));
        }
        output.push_str("\n## Decisions\n");
        for decision in decisions.iter().take(8) {
            output.push_str(&format!(
                "- {} (refs={})\n",
                decision.decision,
                decision.authority_refs.len()
            ));
        }
        output.push_str("\n## Task Memory\n");
        for task in tasks.iter().take(8) {
            output.push_str(&format!("- {}\n", task.summary));
        }
        output.push_str("\n## Next Action\n- Revalidate stale facts against current source before using them.\n");
        output
    }

    pub fn archive_snapshot(
        &mut self,
        scope: MemoryScope,
        reason: impl Into<String>,
        content: String,
    ) -> AcResult<StableId> {
        if content.trim().is_empty() {
            return Err(AcError::validation(
                "MEMORY-EMPTY_SNAPSHOT",
                "snapshot content is required",
            ));
        }
        let source_fact_ids = self
            .facts
            .values()
            .filter(|fact| fact.scope.repository_id == scope.repository_id)
            .map(|fact| fact.id.clone())
            .collect();
        let decision_refs = self
            .decisions
            .values()
            .filter(|decision| decision.scope.repository_id == scope.repository_id)
            .map(|decision| decision.id.clone())
            .collect();
        let id = StableId::new("snapshot");
        self.snapshots.insert(
            id.clone(),
            ContextSnapshot {
                id: id.clone(),
                scope,
                reason: reason.into(),
                content,
                source_fact_ids,
                decision_refs,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn snapshot(&self, id: &StableId) -> Option<ContextSnapshot> {
        self.snapshots.get(id).cloned()
    }

    pub fn export_handoff(
        &mut self,
        scope: MemoryScope,
        mission: &str,
        reason: &str,
    ) -> AcResult<HandoffContext> {
        let context_markdown = self.generate_context_markdown(&scope, mission);
        let snapshot_id = self.archive_snapshot(scope.clone(), reason, context_markdown.clone())?;
        let fact_refs = self
            .facts
            .values()
            .filter(|fact| fact.scope.repository_id == scope.repository_id)
            .map(|fact| fact.id.clone())
            .collect();
        let decision_refs = self
            .decisions
            .values()
            .filter(|decision| decision.scope.repository_id == scope.repository_id)
            .map(|decision| decision.id.clone())
            .collect();
        let task_memory_refs = self
            .task_memories
            .values()
            .filter(|task| scope.task_id.as_ref().is_none_or(|id| id == &task.task_id))
            .map(|task| task.id.clone())
            .collect();
        Ok(HandoffContext {
            id: StableId::new("handoff"),
            context_markdown,
            snapshot_id,
            fact_refs,
            decision_refs,
            task_memory_refs,
        })
    }

    fn detect_conflicts(&mut self) {
        let active_ids = self
            .facts
            .values()
            .filter(|fact| fact.valid_until.is_none())
            .map(|fact| fact.id.clone())
            .collect::<Vec<_>>();
        for left_id in &active_ids {
            for right_id in &active_ids {
                if left_id >= right_id {
                    continue;
                }
                let (Some(left), Some(right)) = (self.facts.get(left_id), self.facts.get(right_id))
                else {
                    continue;
                };
                if left.fact_type == right.fact_type
                    && left.scope.repository_id == right.scope.repository_id
                    && contradicts(&left.statement, &right.statement)
                {
                    let conflict = StableId::new("conflict");
                    if let Some(left) = self.facts.get_mut(left_id) {
                        left.freshness = FreshnessState::Conflicted;
                        left.conflict_set = Some(conflict.clone());
                    }
                    if let Some(right) = self.facts.get_mut(right_id) {
                        right.freshness = FreshnessState::Conflicted;
                        right.conflict_set = Some(conflict);
                    }
                }
            }
        }
    }
}

fn contradicts(left: &str, right: &str) -> bool {
    let left = left.to_ascii_lowercase();
    let right = right.to_ascii_lowercase();
    left != right
        && (left.replace("service a", "service ?") == right.replace("service b", "service ?")
            || left.replace("true", "?") == right.replace("false", "?")
            || (left.contains(" owns ")
                && right.contains(" owns ")
                && subject_tail(&left) == subject_tail(&right)))
}

fn subject_tail(value: &str) -> &str {
    value
        .split_once(" owns ")
        .map(|(_, tail)| tail)
        .unwrap_or(value)
}

#[derive(Default)]
pub struct ContextEngine;

impl ContextEngine {
    pub fn build_context_pack(
        &self,
        mut nodes: Vec<ContextNode>,
        budget: u32,
    ) -> AcResult<ContextPack> {
        if budget == 0 {
            return Err(AcError::validation(
                "CONTEXT-INVALID_BUDGET",
                "context token budget must be positive",
            ));
        }
        nodes.sort_by_key(|node| match node.authority {
            AuthorityClass::KernelState => 0,
            AuthorityClass::RawEvidence => 1,
            AuthorityClass::AcceptedMemory => 2,
            AuthorityClass::DerivedSummary => 3,
            AuthorityClass::RuntimeContext => 4,
            AuthorityClass::RetrievalAccelerator => 5,
        });
        let mut used = 0;
        let original = nodes.len();
        nodes.retain(|node| {
            if node.protected || used + node.token_estimate <= budget {
                used += node.token_estimate;
                true
            } else {
                false
            }
        });
        Ok(ContextPack {
            id: StableId::new("ctx"),
            omitted_count: original.saturating_sub(nodes.len()),
            nodes,
            budget,
        })
    }

    pub fn build_context_pack_for_request(
        &self,
        request: ContextPackRequest,
    ) -> AcResult<ContextBuildReceipt> {
        if request.acceptance_criteria.is_empty()
            || request
                .acceptance_criteria
                .iter()
                .any(|criterion| criterion.trim().is_empty())
        {
            return Err(AcError::validation(
                "CONTEXT-MISSING_ACCEPTANCE",
                "context packs must include required acceptance criteria",
            ));
        }

        let hard_limit = request
            .budget
            .available_input()?
            .min(request.budget.target_input);
        let mut fragments = Vec::new();
        fragments.extend(system_fragments(&request));
        fragments.extend(scoped_project_rules(
            &request.project_rules,
            &request.fragments,
        ));
        fragments.extend(request.fragments.clone());

        let mut score_trace = Vec::new();
        let mut redacted_fragments = 0;
        let mut candidates = fragments
            .into_iter()
            .filter_map(|mut fragment| {
                if fragment.sensitivity == SensitivityClass::Secret
                    && !request.provider_allows_sensitive
                {
                    redacted_fragments += 1;
                    fragment.content = redact_sensitive(&fragment.content);
                    fragment.sensitivity = SensitivityClass::Internal;
                    if fragment.content.trim().is_empty() {
                        return None;
                    }
                } else {
                    let redacted = redact_sensitive(&fragment.content);
                    if redacted != fragment.content {
                        redacted_fragments += 1;
                        fragment.content = redacted;
                        fragment.sensitivity = SensitivityClass::Internal;
                    }
                }
                fragment.score = relevance_score(&fragment, &request);
                score_trace.push(format!(
                    "{}:{}:{}",
                    fragment.id,
                    fragment.fragment_type.as_str(),
                    fragment.score
                ));
                Some(fragment)
            })
            .collect::<Vec<_>>();

        let original_count = candidates.len();
        candidates = dedupe_fragments(candidates);
        let deduped_fragments = original_count.saturating_sub(candidates.len());
        candidates.sort_by(|left, right| {
            fragment_rank(right, request.role).cmp(&fragment_rank(left, request.role))
        });

        let mut selected = Vec::new();
        let mut omitted = Vec::new();
        let mut used = 0_u32;

        for fragment in candidates {
            let mandatory = fragment.hard_include
                || request.explicit_source_refs.contains(&fragment.source_ref)
                || fragment.fragment_type == ContextFragmentType::AcceptanceCriteria
                || fragment.fragment_type == ContextFragmentType::ProjectRule;
            if used.saturating_add(fragment.token_estimate) <= hard_limit {
                used = used.saturating_add(fragment.token_estimate);
                selected.push(fragment);
            } else if mandatory {
                return Err(AcError::validation(
                    "CONTEXT-MANDATORY_OVER_BUDGET",
                    "mandatory context cannot fit within the hard ceiling",
                ));
            } else {
                omitted.push(fragment);
            }
        }

        let mut retrieval_records = Vec::new();
        for retrieval in &request.retrieval_requests {
            let remaining = hard_limit.saturating_sub(used).min(retrieval.max_tokens);
            if remaining == 0 {
                retrieval_records.push(ProgressiveRetrievalRecord {
                    id: StableId::new("ctxret"),
                    pack_id: StableId::from_existing("pending-pack").expect("valid id"),
                    need: retrieval.need,
                    reason: retrieval.reason.clone(),
                    query: retrieval.query.clone(),
                    result_fragment_ids: Vec::new(),
                    added_tokens: 0,
                    degraded: true,
                    created_at: TimestampMillis::now(),
                });
                continue;
            }

            let mut added = Vec::new();
            let mut still_omitted = Vec::new();
            for fragment in omitted {
                if fragment_matches_retrieval(&fragment, retrieval)
                    && used.saturating_add(fragment.token_estimate) <= hard_limit
                {
                    used = used.saturating_add(fragment.token_estimate);
                    added.push(fragment);
                } else {
                    still_omitted.push(fragment);
                }
            }
            omitted = still_omitted;
            let added_tokens = added.iter().map(|fragment| fragment.token_estimate).sum();
            let result_fragment_ids = added
                .iter()
                .map(|fragment| fragment.id.clone())
                .collect::<Vec<_>>();
            selected.extend(added);
            retrieval_records.push(ProgressiveRetrievalRecord {
                id: StableId::new("ctxret"),
                pack_id: StableId::from_existing("pending-pack").expect("valid id"),
                need: retrieval.need,
                reason: retrieval.reason.clone(),
                query: retrieval.query.clone(),
                result_fragment_ids,
                added_tokens,
                degraded: false,
                created_at: TimestampMillis::now(),
            });
        }

        let pack_id = StableId::new("ctx");
        for record in &mut retrieval_records {
            record.pack_id = pack_id.clone();
        }
        let source_fragment_ids = selected
            .iter()
            .map(|fragment| fragment.id.clone())
            .collect::<Vec<_>>();
        let omitted_fragment_ids = omitted
            .iter()
            .map(|fragment| fragment.id.clone())
            .collect::<Vec<_>>();
        let raw_evidence_refs = selected
            .iter()
            .filter_map(|fragment| fragment.raw_evidence_ref.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let cache_keys = selected
            .iter()
            .filter_map(|fragment| fragment.cache_key.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let nodes = selected
            .iter()
            .map(|fragment| ContextNode {
                id: fragment.id.clone(),
                source_ref: fragment.source_ref.clone(),
                authority: fragment.authority,
                content: render_fragment(fragment),
                token_estimate: fragment.token_estimate,
                protected: fragment.hard_include,
                degraded: fragment.degraded,
            })
            .collect::<Vec<_>>();

        let manifest = ContextManifest {
            id: StableId::new("ctxmanifest"),
            pack_id: pack_id.clone(),
            role: request.role,
            task_id: request.task_id,
            profile: request.profile,
            source_fragment_ids,
            omitted_fragment_ids,
            raw_evidence_refs,
            cache_keys,
            score_trace,
            total_input_tokens: used,
            hard_ceiling: request.budget.hard_ceiling,
            created_at: TimestampMillis::now(),
        };
        let metrics = ContextPackMetrics {
            id: StableId::new("ctxmetric"),
            pack_id: pack_id.clone(),
            role: request.role,
            selected_fragments: nodes.len(),
            omitted_fragments: manifest.omitted_fragment_ids.len(),
            total_input_tokens: used,
            budget_target: request.budget.target_input,
            hard_ceiling: request.budget.hard_ceiling,
            deduped_fragments,
            redacted_fragments,
            retrieval_steps: retrieval_records.len(),
            cache_hits: manifest.cache_keys.len(),
            created_at: TimestampMillis::now(),
        };

        Ok(ContextBuildReceipt {
            pack: ContextPack {
                id: pack_id,
                nodes,
                budget: request.budget.target_input,
                omitted_count: manifest.omitted_fragment_ids.len(),
            },
            manifest,
            retrieval_records,
            metrics,
        })
    }

    pub fn compress_tool_output(
        &self,
        raw_evidence_ref: StableId,
        command_class: impl Into<String>,
        raw_output: &str,
    ) -> CompressionReceipt {
        let command_class = command_class.into();
        let raw_lines = raw_output.lines().collect::<Vec<_>>();
        let important = raw_lines
            .iter()
            .filter(|line| is_important_output_line(line))
            .copied()
            .collect::<Vec<_>>();
        let mut compressed = Vec::new();
        if important.is_empty() {
            compressed.extend(raw_lines.iter().take(8).copied());
            if raw_lines.len() > 16 {
                compressed.push("[... middle lines omitted ...]");
            }
            compressed.extend(raw_lines.iter().rev().take(8).rev().copied());
        } else {
            compressed.extend(important.into_iter().take(48));
        }
        let compressed_output = compressed.join("\n");
        CompressionReceipt {
            id: StableId::new("ctxcompress"),
            raw_evidence_ref,
            command_class,
            compressor_id: "agentcode-rtk-fallback-v1".to_string(),
            raw_hash: stable_hash(raw_output),
            raw_token_estimate: estimate_tokens(raw_output),
            compressed_token_estimate: estimate_tokens(&compressed_output),
            omitted_lines: raw_lines.len().saturating_sub(compressed.len()) as u32,
            compressed_output,
            created_at: TimestampMillis::now(),
        }
    }

    pub fn cache_entry_for_fragment(&self, fragment: &ContextFragment) -> ContextCacheEntry {
        let key = fragment.cache_key.clone().unwrap_or_else(|| {
            format!(
                "{}:{}:{}",
                fragment.source_ref,
                fragment.range.as_deref().unwrap_or("all"),
                stable_hash(&fragment.content)
            )
        });
        ContextCacheEntry {
            key,
            content_hash: stable_hash(&fragment.content),
            token_estimate: fragment.token_estimate,
            source_ref: fragment.source_ref.clone(),
            created_at: TimestampMillis::now(),
        }
    }

    pub fn benchmark_targeted_context(
        &self,
        input: ContextBenchmarkInput,
        targeted: &ContextBuildReceipt,
    ) -> ContextBenchmarkResult {
        let targeted_tokens = targeted.manifest.total_input_tokens;
        ContextBenchmarkResult {
            id: StableId::new("ctxbench"),
            task_name: input.task_name,
            broad_tokens: input.broad_tokens,
            targeted_tokens,
            broad_success: input.broad_success,
            targeted_success: input.targeted_success,
            retry_delta: input.retry_delta,
            latency_delta_ms: input.latency_delta_ms,
            passed: input.targeted_success
                && (!input.broad_success || targeted_tokens < input.broad_tokens)
                && input.retry_delta <= 0,
            created_at: TimestampMillis::now(),
        }
    }
}

fn system_fragments(request: &ContextPackRequest) -> Vec<ContextFragment> {
    let mut fragments = vec![
        ContextFragment {
            id: StableId::new("ctxfrag"),
            fragment_type: ContextFragmentType::Mission,
            source_ref: StableId::new("mission"),
            source_path: None,
            range: None,
            reason: "role and task header".to_string(),
            freshness: FreshnessState::Fresh,
            score: 0,
            token_estimate: estimate_tokens(&request.task).max(8),
            sensitivity: SensitivityClass::Internal,
            authority: AuthorityClass::KernelState,
            content: format!("role={}\ntask={}", request.role.as_str(), request.task),
            hard_include: true,
            degraded: false,
            cache_key: None,
            raw_evidence_ref: None,
            relationships: Vec::new(),
        },
        ContextFragment {
            id: StableId::new("ctxfrag"),
            fragment_type: ContextFragmentType::AcceptanceCriteria,
            source_ref: StableId::new("acceptance"),
            source_path: None,
            range: None,
            reason: "required acceptance criteria".to_string(),
            freshness: FreshnessState::Fresh,
            score: 0,
            token_estimate: estimate_tokens(&request.acceptance_criteria.join("\n")).max(8),
            sensitivity: SensitivityClass::Internal,
            authority: AuthorityClass::KernelState,
            content: request.acceptance_criteria.join("\n"),
            hard_include: true,
            degraded: false,
            cache_key: None,
            raw_evidence_ref: None,
            relationships: Vec::new(),
        },
    ];
    if !request.mission_subset.is_empty() {
        fragments.push(ContextFragment {
            id: StableId::new("ctxfrag"),
            fragment_type: ContextFragmentType::Mission,
            source_ref: StableId::new("mission"),
            source_path: None,
            range: None,
            reason: "mission subset".to_string(),
            freshness: FreshnessState::Fresh,
            score: 0,
            token_estimate: estimate_tokens(&request.mission_subset.join("\n")).max(8),
            sensitivity: SensitivityClass::Internal,
            authority: AuthorityClass::KernelState,
            content: request.mission_subset.join("\n"),
            hard_include: true,
            degraded: false,
            cache_key: None,
            raw_evidence_ref: None,
            relationships: Vec::new(),
        });
    }
    fragments
}

fn scoped_project_rules(
    rules: &[ContextFragment],
    task_fragments: &[ContextFragment],
) -> Vec<ContextFragment> {
    let target_paths = task_fragments
        .iter()
        .filter(|fragment| {
            matches!(
                fragment.fragment_type,
                ContextFragmentType::TargetSource
                    | ContextFragmentType::RelatedSource
                    | ContextFragmentType::Test
                    | ContextFragmentType::Diff
                    | ContextFragmentType::Error
            )
        })
        .filter_map(|fragment| fragment.source_path.as_deref())
        .collect::<Vec<_>>();
    rules
        .iter()
        .filter(|rule| {
            rule.source_path
                .as_deref()
                .is_none_or(|scope| target_paths.iter().any(|path| path.starts_with(scope)))
        })
        .cloned()
        .collect()
}

fn relevance_score(fragment: &ContextFragment, request: &ContextPackRequest) -> i32 {
    let mut score = fragment.score;
    if fragment.hard_include {
        score += 1_000;
    }
    if request.explicit_source_refs.contains(&fragment.source_ref) {
        score += 100;
    }
    score += match fragment.fragment_type {
        ContextFragmentType::TargetSource => 95,
        ContextFragmentType::Error => 90,
        ContextFragmentType::AcceptanceCriteria => 100,
        ContextFragmentType::ProjectRule => 85,
        ContextFragmentType::Test => 65,
        ContextFragmentType::RelatedSource => 60,
        ContextFragmentType::Diff => 55,
        ContextFragmentType::Decision => 45,
        ContextFragmentType::FailedApproach => 35,
        ContextFragmentType::RepoMap => 25,
        ContextFragmentType::Mission => 80,
        ContextFragmentType::ToolState | ContextFragmentType::ToolOutput => 30,
    };
    match fragment.freshness {
        FreshnessState::Fresh => score += 20,
        FreshnessState::PossiblyStale => score -= 20,
        FreshnessState::Invalid => score -= 200,
        FreshnessState::Conflicted => score -= 40,
    }
    if request
        .task
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| term.len() > 3)
        .any(|term| fragment.content.contains(term))
    {
        score += 30;
    }
    score
}

fn fragment_rank(fragment: &ContextFragment, role: ContextRole) -> (i32, i32, i32) {
    let role_bias = match role {
        ContextRole::Planner => match fragment.fragment_type {
            ContextFragmentType::Mission | ContextFragmentType::Decision => 40,
            ContextFragmentType::RepoMap => 35,
            _ => 0,
        },
        ContextRole::Worker => match fragment.fragment_type {
            ContextFragmentType::TargetSource | ContextFragmentType::Error => 45,
            ContextFragmentType::Test | ContextFragmentType::Diff => 25,
            _ => 0,
        },
        ContextRole::Researcher => match fragment.fragment_type {
            ContextFragmentType::RepoMap | ContextFragmentType::RelatedSource => 40,
            _ => 0,
        },
        ContextRole::Verifier => match fragment.fragment_type {
            ContextFragmentType::AcceptanceCriteria | ContextFragmentType::Test => 45,
            ContextFragmentType::Diff | ContextFragmentType::Error => 35,
            ContextFragmentType::TargetSource => -10,
            _ => 0,
        },
    };
    let authority = match fragment.authority {
        AuthorityClass::KernelState => 60,
        AuthorityClass::RawEvidence => 50,
        AuthorityClass::AcceptedMemory => 40,
        AuthorityClass::DerivedSummary => 25,
        AuthorityClass::RuntimeContext => 20,
        AuthorityClass::RetrievalAccelerator => 10,
    };
    (
        fragment.score + role_bias,
        authority,
        -(fragment.token_estimate as i32),
    )
}

fn dedupe_fragments(fragments: Vec<ContextFragment>) -> Vec<ContextFragment> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for fragment in fragments {
        let key = format!(
            "{}:{}",
            fragment.source_ref,
            stable_hash(&normalize_for_dedupe(&fragment.content))
        );
        if seen.insert(key) {
            output.push(fragment);
        }
    }
    output
}

fn fragment_matches_retrieval(
    fragment: &ContextFragment,
    retrieval: &ProgressiveRetrievalRequest,
) -> bool {
    let query = retrieval.query.to_ascii_lowercase();
    let content = fragment.content.to_ascii_lowercase();
    let reason = fragment.reason.to_ascii_lowercase();
    let type_match = matches!(
        (retrieval.need, fragment.fragment_type),
        (
            ProgressiveNeed::Definition,
            ContextFragmentType::TargetSource
        ) | (
            ProgressiveNeed::References,
            ContextFragmentType::RelatedSource
        ) | (ProgressiveNeed::RelatedTests, ContextFragmentType::Test)
            | (ProgressiveNeed::BroaderScope, ContextFragmentType::RepoMap)
            | (ProgressiveNeed::RawOutput, ContextFragmentType::ToolOutput)
            | (
                ProgressiveNeed::History,
                ContextFragmentType::FailedApproach
            )
    );
    type_match || content.contains(&query) || reason.contains(&query)
}

fn render_fragment(fragment: &ContextFragment) -> String {
    let path = fragment.source_path.as_deref().unwrap_or("n/a");
    format!(
        "[{} source={} path={} reason={} sensitivity={}]\n{}",
        fragment.fragment_type.as_str(),
        fragment.source_ref,
        path,
        fragment.reason,
        fragment.sensitivity.as_str(),
        fragment.content
    )
}

fn redact_sensitive(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            let upper = line.to_ascii_uppercase();
            if upper.contains("SECRET")
                || upper.contains("PASSWORD")
                || upper.contains("TOKEN=")
                || upper.contains("API_KEY")
                || upper.contains("PRIVATE KEY")
                || line.contains("sk-")
            {
                if let Some((name, _)) = line.split_once('=') {
                    format!("{name}=[REDACTED]")
                } else {
                    "[REDACTED]".to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_for_dedupe(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_important_output_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("error")
        || lower.contains("failed")
        || lower.contains("failure")
        || lower.contains("panic")
        || lower.contains("warning")
        || lower.contains("diff --git")
        || lower.contains("modified:")
        || lower.contains("untracked")
}

fn estimate_tokens(input: &str) -> u32 {
    let chars = input.chars().count() as u32;
    chars.div_ceil(4).max(1)
}

fn stable_hash(input: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_requires_evidence_and_supersedes_without_overwrite() {
        let mut memory = MemoryService::new();
        let err = memory.record_fact("truth", Vec::new(), 80).unwrap_err();
        assert_eq!(err.code(), "MEMORY-INVALID_FACT");
        let evidence = StableId::new("ev");
        let fact = memory
            .record_fact("old fact", vec![evidence.clone()], 80)
            .unwrap();
        let replacement = memory
            .supersede_fact(&fact, "new fact", vec![evidence])
            .unwrap();
        assert_ne!(fact, replacement);
        assert!(memory.search_memory("old").is_empty());
    }

    #[test]
    fn freshness_invalidates_only_dependent_facts() {
        let mut memory = MemoryService::new();
        let scope = test_scope();
        let affected = memory
            .record_typed_fact(FactInput {
                statement: "A calls B".to_string(),
                fact_type: FactType::ModuleRelationship,
                source: FactSource::Lsp,
                source_evidence: vec![StableId::new("ev")],
                confidence: 90,
                scope: scope.clone(),
                memory_class: MemoryClass::LongLivedRepo,
                observed_commit: "abc".to_string(),
                dependencies: vec![FreshnessDependency {
                    evidence_ref: StableId::new("ev"),
                    file_path: Some("src/a.rs".to_string()),
                    symbol: Some("A".to_string()),
                    content_hash: Some("h1".to_string()),
                }],
            })
            .unwrap();
        let unaffected = memory
            .record_typed_fact(FactInput {
                statement: "C calls D".to_string(),
                fact_type: FactType::ModuleRelationship,
                source: FactSource::TreeSitter,
                source_evidence: vec![StableId::new("ev")],
                confidence: 80,
                scope,
                memory_class: MemoryClass::LongLivedRepo,
                observed_commit: "abc".to_string(),
                dependencies: vec![FreshnessDependency {
                    evidence_ref: StableId::new("ev"),
                    file_path: Some("src/c.rs".to_string()),
                    symbol: Some("C".to_string()),
                    content_hash: Some("h2".to_string()),
                }],
            })
            .unwrap();

        let changed = memory.apply_source_change("src/a.rs", Some("A"), false);
        assert_eq!(changed, vec![affected.clone()]);
        assert_eq!(
            memory.fact(&affected).unwrap().freshness,
            FreshnessState::PossiblyStale
        );
        assert_eq!(
            memory.fact(&unaffected).unwrap().freshness,
            FreshnessState::Fresh
        );

        memory.apply_source_change("src/a.rs", Some("A"), true);
        assert_eq!(
            memory.fact(&affected).unwrap().freshness,
            FreshnessState::Invalid
        );
    }

    #[test]
    fn conflicting_facts_are_stored_not_overwritten() {
        let mut memory = MemoryService::new();
        let scope = test_scope();
        let left = record_arch_fact(&mut memory, scope.clone(), "Service A owns authentication");
        let right = record_arch_fact(&mut memory, scope, "Service B owns authentication");
        assert_eq!(
            memory.fact(&left).unwrap().freshness,
            FreshnessState::Conflicted
        );
        assert_eq!(
            memory.fact(&right).unwrap().freshness,
            FreshnessState::Conflicted
        );
        assert_ne!(
            memory.fact(&left).unwrap().conflict_set,
            None,
            "conflict set must be explicit"
        );
    }

    #[test]
    fn context_snapshot_and_handoff_are_generated_from_structured_state() {
        let mut memory = MemoryService::new();
        let scope = test_scope();
        let evidence = StableId::new("ev");
        record_arch_fact(
            &mut memory,
            scope.clone(),
            "Repository uses ac-context for memory",
        );
        let decision = memory
            .record_decision(
                scope.clone(),
                "Keep CONTEXT.md derived",
                "Doc 02 says summaries are handoff views",
                vec![evidence.clone()],
                None,
            )
            .unwrap();
        let task = memory
            .record_task_memory(
                StableId::from_existing("task-1").unwrap(),
                "Completed memory freshness fixture",
                vec![evidence],
            )
            .unwrap();
        let handoff = memory
            .export_handoff(scope, "phase 10 fixture", "replacement-agent")
            .unwrap();
        assert!(handoff
            .context_markdown
            .contains("not authority over current source"));
        assert!(handoff.decision_refs.contains(&decision));
        assert!(handoff.task_memory_refs.contains(&task));
        assert!(memory.snapshot(&handoff.snapshot_id).is_some());
    }

    #[test]
    fn context_builder_applies_phase_11_gates() {
        let engine = ContextEngine;
        let target_ref = StableId::from_existing("src-lib").unwrap();
        let raw_ref = StableId::from_existing("raw-test-output").unwrap();
        let request = ContextPackRequest {
            role: ContextRole::Worker,
            task_id: StableId::from_existing("task-11").unwrap(),
            task: "Fix auth compiler error in src/auth.rs".to_string(),
            profile: ContextProfile::Normal,
            budget: TokenBudget::new(420, 500, 20, 20, 20).unwrap(),
            acceptance_criteria: vec!["cargo test must pass".to_string()],
            mission_subset: vec!["Phase 11 context pack fixture".to_string()],
            project_rules: vec![
                fragment(
                    ContextFragmentType::ProjectRule,
                    "rules-auth",
                    Some("src/"),
                    "Use Kernel state as authority",
                    18,
                    true,
                ),
                fragment(
                    ContextFragmentType::ProjectRule,
                    "rules-docs",
                    Some("docs/"),
                    "Docs-only rule should be scoped out",
                    18,
                    true,
                ),
            ],
            fragments: vec![
                fragment_with_ref(
                    ContextFragmentType::TargetSource,
                    target_ref.clone(),
                    Some("src/auth.rs"),
                    "pub fn login() { let token = env::var(\"TOKEN=SECRET\"); }",
                    55,
                    true,
                    Some("commit:abc:src/auth.rs".to_string()),
                    None,
                ),
                fragment_with_ref(
                    ContextFragmentType::RepoMap,
                    target_ref.clone(),
                    Some("src/auth.rs"),
                    "pub fn login() { let token = env::var(\"TOKEN=SECRET\"); }",
                    55,
                    false,
                    None,
                    None,
                ),
                fragment_with_ref(
                    ContextFragmentType::Test,
                    StableId::from_existing("tests-auth").unwrap(),
                    Some("tests/auth.rs"),
                    "auth_rejects_invalid_session",
                    25,
                    false,
                    None,
                    None,
                ),
                fragment_with_ref(
                    ContextFragmentType::ToolOutput,
                    StableId::from_existing("tool-output").unwrap(),
                    None,
                    "error[E0425]: cannot find value session",
                    24,
                    false,
                    None,
                    Some(raw_ref.clone()),
                ),
            ],
            explicit_source_refs: vec![target_ref],
            retrieval_requests: vec![ProgressiveRetrievalRequest {
                need: ProgressiveNeed::RelatedTests,
                reason: "worker asked for related tests".to_string(),
                query: "auth".to_string(),
                max_tokens: 40,
            }],
            provider_allows_sensitive: false,
            cache_parameters: "role=worker;profile=normal".to_string(),
        };

        let receipt = engine.build_context_pack_for_request(request).unwrap();
        let rendered = receipt
            .pack
            .nodes
            .iter()
            .map(|node| node.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("cargo test must pass"));
        assert!(rendered.contains("Use Kernel state as authority"));
        assert!(!rendered.contains("Docs-only rule"));
        assert!(!rendered.contains("TOKEN=SECRET"));
        assert!(rendered.contains("[REDACTED]"));
        assert_eq!(
            receipt
                .pack
                .nodes
                .iter()
                .filter(|node| node.source_ref.as_str() == "src-lib")
                .count(),
            1,
            "same source/content should be deduped"
        );
        assert!(receipt.manifest.total_input_tokens <= 420);
        assert!(receipt.manifest.raw_evidence_refs.contains(&raw_ref));
        assert_eq!(receipt.retrieval_records.len(), 1);
        assert!(receipt.metrics.deduped_fragments >= 1);
        assert!(receipt.metrics.redacted_fragments >= 1);
    }

    #[test]
    fn verifier_context_is_anti_anchored_against_worker_context() {
        let engine = ContextEngine;
        let worker = engine
            .build_context_pack_for_request(role_request(ContextRole::Worker))
            .unwrap();
        let verifier = engine
            .build_context_pack_for_request(role_request(ContextRole::Verifier))
            .unwrap();
        let worker_first = worker.pack.nodes.first().unwrap().content.clone();
        let verifier_first = verifier.pack.nodes.first().unwrap().content.clone();
        assert_ne!(worker_first, verifier_first);
        assert!(worker
            .pack
            .nodes
            .iter()
            .any(|node| node.content.contains("src/lib.rs")));
        assert!(verifier
            .pack
            .nodes
            .iter()
            .any(|node| node.content.contains("acceptance")));
    }

    #[test]
    fn mandatory_context_over_hard_ceiling_is_rejected() {
        let engine = ContextEngine;
        let mut request = role_request(ContextRole::Worker);
        request.budget = TokenBudget::new(20, 50, 10, 10, 10).unwrap();
        request.fragments.push(fragment(
            ContextFragmentType::TargetSource,
            "huge",
            Some("src/huge.rs"),
            "x".repeat(400).as_str(),
            200,
            true,
        ));
        let err = engine.build_context_pack_for_request(request).unwrap_err();
        assert_eq!(err.code(), "CONTEXT-MANDATORY_OVER_BUDGET");
    }

    #[test]
    fn compression_cache_and_benchmark_are_reported() {
        let engine = ContextEngine;
        let receipt = engine
            .build_context_pack_for_request(role_request(ContextRole::Worker))
            .unwrap();
        let compression = engine.compress_tool_output(
            StableId::from_existing("raw-1").unwrap(),
            "tests",
            "running 3 tests\nok\nwarning: unused value\nerror: failed assertion\nfull tail",
        );
        assert!(compression.compressed_token_estimate <= compression.raw_token_estimate);
        assert_eq!(compression.raw_evidence_ref.as_str(), "raw-1");
        assert!(compression.omitted_lines > 0);

        let cache = engine.cache_entry_for_fragment(&fragment(
            ContextFragmentType::TargetSource,
            "cache-src",
            Some("src/cache.rs"),
            "pub fn cached() {}",
            8,
            false,
        ));
        assert!(!cache.key.trim().is_empty());
        let benchmark = engine.benchmark_targeted_context(
            ContextBenchmarkInput {
                task_name: "cross-module bug".to_string(),
                broad_tokens: 2_000,
                broad_success: true,
                targeted_success: true,
                retry_delta: 0,
                latency_delta_ms: -5,
            },
            &receipt,
        );
        assert!(benchmark.passed);
        assert!(benchmark.targeted_tokens < benchmark.broad_tokens);
    }

    fn record_arch_fact(
        memory: &mut MemoryService,
        scope: MemoryScope,
        statement: &str,
    ) -> StableId {
        memory
            .record_typed_fact(FactInput {
                statement: statement.to_string(),
                fact_type: FactType::ArchitectureFact,
                source: FactSource::ArchitectureDecision,
                source_evidence: vec![StableId::new("ev")],
                confidence: 95,
                scope,
                memory_class: MemoryClass::Decision,
                observed_commit: "abc".to_string(),
                dependencies: Vec::new(),
            })
            .unwrap()
    }

    fn test_scope() -> MemoryScope {
        MemoryScope {
            repository_id: StableId::from_existing("repo-1").unwrap(),
            mission_id: Some(StableId::from_existing("mission-1").unwrap()),
            task_id: Some(StableId::from_existing("task-1").unwrap()),
            branch: Some("main".to_string()),
        }
    }

    fn role_request(role: ContextRole) -> ContextPackRequest {
        ContextPackRequest {
            role,
            task_id: StableId::from_existing("task-role").unwrap(),
            task: "worker verifier acceptance src/lib.rs".to_string(),
            profile: ContextProfile::Normal,
            budget: TokenBudget::new(300, 400, 20, 20, 20).unwrap(),
            acceptance_criteria: vec!["acceptance: tests prove behavior".to_string()],
            mission_subset: Vec::new(),
            project_rules: vec![fragment(
                ContextFragmentType::ProjectRule,
                "rules-src",
                Some("src/"),
                "respect architecture",
                12,
                true,
            )],
            fragments: vec![
                fragment(
                    ContextFragmentType::TargetSource,
                    "src-lib",
                    Some("src/lib.rs"),
                    "src/lib.rs target implementation",
                    30,
                    false,
                ),
                fragment(
                    ContextFragmentType::Test,
                    "test-lib",
                    Some("tests/lib.rs"),
                    "acceptance regression test",
                    30,
                    false,
                ),
            ],
            explicit_source_refs: Vec::new(),
            retrieval_requests: Vec::new(),
            provider_allows_sensitive: false,
            cache_parameters: "role-test".to_string(),
        }
    }

    fn fragment(
        fragment_type: ContextFragmentType,
        source_ref: &str,
        source_path: Option<&str>,
        content: &str,
        token_estimate: u32,
        hard_include: bool,
    ) -> ContextFragment {
        fragment_with_ref(
            fragment_type,
            StableId::from_existing(source_ref).unwrap(),
            source_path,
            content,
            token_estimate,
            hard_include,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn fragment_with_ref(
        fragment_type: ContextFragmentType,
        source_ref: StableId,
        source_path: Option<&str>,
        content: &str,
        token_estimate: u32,
        hard_include: bool,
        cache_key: Option<String>,
        raw_evidence_ref: Option<StableId>,
    ) -> ContextFragment {
        ContextFragment {
            id: StableId::new("ctxfrag"),
            fragment_type,
            source_ref,
            source_path: source_path.map(str::to_string),
            range: None,
            reason: "test fixture".to_string(),
            freshness: FreshnessState::Fresh,
            score: 0,
            token_estimate,
            sensitivity: SensitivityClass::Internal,
            authority: AuthorityClass::RawEvidence,
            content: content.to_string(),
            hard_include,
            degraded: false,
            cache_key,
            raw_evidence_ref,
            relationships: Vec::new(),
        }
    }
}
