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
    pub raw_content: Option<String>,
    pub model_summary: Option<String>,
    pub sensitive: bool,
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
            raw_content: None,
            model_summary: None,
            sensitive: false,
            created_at: TimestampMillis::now(),
        };
        self.records.insert(id.clone(), record);
        Ok(id)
    }

    pub fn append_tool_output(
        &mut self,
        provenance: Provenance,
        artifact_uri: impl Into<String>,
        raw_content: impl Into<String>,
        secrets: &[String],
    ) -> AcResult<StableId> {
        let raw_content = raw_content.into();
        let redacted = redact(&raw_content, secrets);
        let id = self.append(
            EvidenceKind::CommandOutput,
            provenance,
            artifact_uri,
            content_hash(&raw_content),
        )?;
        let record = self.records.get_mut(&id).expect("new evidence exists");
        record.raw_content = Some(raw_content);
        record.model_summary = Some(bounded_summary(&redacted));
        record.sensitive = !secrets.is_empty();
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

fn content_hash(content: &str) -> String {
    // A deterministic FNV-1a fingerprint is sufficient for local evidence tamper checks.
    let hash = content
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("fnv1a64:{hash:016x}")
}

fn redact(value: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(value.to_string(), |redacted, secret| {
            redacted.replace(secret, "[REDACTED]")
        })
}

fn bounded_summary(value: &str) -> String {
    const LIMIT: usize = 4096;
    if value.len() <= LIMIT {
        value.to_string()
    } else {
        format!("{}\n[output truncated]", &value[..LIMIT])
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

    #[test]
    fn tool_output_keeps_raw_evidence_but_redacts_model_summary() {
        let mut store = EvidenceStore::new();
        let id = store
            .append_tool_output(
                provenance(),
                "mem://tool/output",
                "token=canary-secret\nerror: failed",
                &["canary-secret".to_string()],
            )
            .unwrap();
        let record = store.get(&id).unwrap();
        assert!(record
            .raw_content
            .as_ref()
            .unwrap()
            .contains("canary-secret"));
        assert!(!record
            .model_summary
            .as_ref()
            .unwrap()
            .contains("canary-secret"));
        assert!(record
            .model_summary
            .as_ref()
            .unwrap()
            .contains("error: failed"));
    }
}
