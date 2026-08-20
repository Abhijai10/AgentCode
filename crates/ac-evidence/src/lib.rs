use std::collections::BTreeMap;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceKind {
    CommandOutput,
    FileSnapshot,
    BrowserScreenshot,
    TestReport,
    DerivedContext,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance {
    pub source: String,
    pub commit: Option<String>,
    pub worktree: Option<String>,
    pub tool: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRecord {
    pub id: StableId,
    pub kind: EvidenceKind,
    pub provenance: Provenance,
    pub artifact_uri: String,
    pub content_hash: String,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct EvidenceStore {
    records: BTreeMap<StableId, EvidenceRecord>,
}

impl EvidenceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(
        &mut self,
        kind: EvidenceKind,
        provenance: Provenance,
        artifact_uri: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> AcResult<StableId> {
        let artifact_uri = artifact_uri.into();
        let content_hash = content_hash.into();
        if artifact_uri.trim().is_empty() || content_hash.trim().is_empty() {
            return Err(AcError::validation(
                "EVIDENCE-MISSING_ARTIFACT",
                "artifact uri and content hash are required",
            ));
        }
        let id = StableId::new("ev");
        let record = EvidenceRecord {
            id: id.clone(),
            kind,
            provenance,
            artifact_uri,
            content_hash,
            created_at: TimestampMillis::now(),
        };
        self.records.insert(id.clone(), record);
        Ok(id)
    }

    pub fn get(&self, id: &StableId) -> Option<&EvidenceRecord> {
        self.records.get(id)
    }

    pub fn replace(&mut self, _id: &StableId, _record: EvidenceRecord) -> AcResult<()> {
        Err(AcError::policy_denied(
            "EVIDENCE-APPEND_ONLY",
            "evidence records are append-only",
        ))
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance() -> Provenance {
        Provenance {
            source: "unit-test".to_string(),
            commit: Some("abc".to_string()),
            worktree: None,
            tool: None,
        }
    }

    #[test]
    fn evidence_is_append_only() {
        let mut store = EvidenceStore::new();
        let id = store
            .append(
                EvidenceKind::CommandOutput,
                provenance(),
                "mem://one",
                "hash",
            )
            .unwrap();
        let record = store.get(&id).unwrap().clone();
        let err = store.replace(&id, record).unwrap_err();
        assert_eq!(err.code(), "EVIDENCE-APPEND_ONLY");
        assert_eq!(store.len(), 1);
    }
}
