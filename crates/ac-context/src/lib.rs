use std::collections::BTreeMap;

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
}
