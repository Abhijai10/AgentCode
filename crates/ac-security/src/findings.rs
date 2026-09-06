pub struct SecurityFinding {
    pub id: StableId,
    pub scanner: String,
    pub severity: String,
    pub fingerprint: String,
    pub evidence_ref: StableId,
    pub created_at: TimestampMillis,
}


#[derive(Default)]
pub struct SecurityFindingStore {
    findings: BTreeMap<String, SecurityFinding>,
}

impl SecurityFindingStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest(
        &mut self,
        scanner: impl Into<String>,
        severity: impl Into<String>,
        fingerprint: impl Into<String>,
        evidence_ref: StableId,
    ) -> AcResult<StableId> {
        let fingerprint = fingerprint.into();
        if fingerprint.trim().is_empty() {
            return Err(AcError::validation(
                "SECURITY-MISSING_FINGERPRINT",
                "finding fingerprint is required",
            ));
        }
        let id = StableId::new("finding");
        self.findings.insert(
            fingerprint.clone(),
            SecurityFinding {
                id: id.clone(),
                scanner: scanner.into(),
                severity: severity.into(),
                fingerprint,
                evidence_ref,
                created_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn len(&self) -> usize {
        self.findings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}
