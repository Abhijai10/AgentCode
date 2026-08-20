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
    pub source_evidence: Vec<StableId>,
    pub confidence: u8,
    pub valid_from: TimestampMillis,
    pub valid_until: Option<TimestampMillis>,
    pub superseded_by: Option<StableId>,
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
        let statement = statement.into();
        if statement.trim().is_empty() || source_evidence.is_empty() {
            return Err(AcError::validation(
                "MEMORY-INVALID_FACT",
                "memory facts require a statement and source evidence",
            ));
        }
        let id = StableId::new("mem");
        self.facts.insert(
            id.clone(),
            MemoryFact {
                id: id.clone(),
                statement,
                source_evidence,
                confidence,
                valid_from: TimestampMillis::now(),
                valid_until: None,
                superseded_by: None,
            },
        );
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
            .filter(|fact| fact.valid_until.is_none() && fact.statement.contains(query))
            .cloned()
            .collect()
    }
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
}
