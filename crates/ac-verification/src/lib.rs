use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::{EvidenceKind, EvidenceStore, Provenance};
use ac_security::{Capability, CapabilityPolicy, SecurityDecision};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserObservation {
    pub id: StableId,
    pub url: String,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationRunReport {
    pub id: StableId,
    pub plan_name: String,
    pub passed: bool,
    pub evidence_ref: StableId,
    pub completed_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityScanReport {
    pub id: StableId,
    pub scanner: String,
    pub findings: usize,
    pub evidence_ref: StableId,
}

pub struct VerificationEngine {
    policy: CapabilityPolicy,
}

impl VerificationEngine {
    pub fn new(policy: CapabilityPolicy) -> Self {
        Self { policy }
    }

    pub fn capture_browser_observation(
        &self,
        url: impl Into<String>,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<BrowserObservation> {
        if self.policy.evaluate(&[Capability::BrowserAutomation]) != SecurityDecision::Allow {
            return Err(AcError::policy_denied(
                "VERIFY-BROWSER_DENIED",
                "browser automation requires capability approval",
            ));
        }
        let url = url.into();
        let evidence_ref = evidence_store.append(
            EvidenceKind::BrowserScreenshot,
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some("browser".to_string()),
            },
            format!("mem://browser/{}", StableId::new("capture")),
            format!("url:{}", url),
        )?;
        Ok(BrowserObservation {
            id: StableId::new("browser"),
            url,
            evidence_ref,
        })
    }

    pub fn record_validation(
        &self,
        plan_name: impl Into<String>,
        passed: bool,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ValidationRunReport> {
        let plan_name = plan_name.into();
        if plan_name.trim().is_empty() {
            return Err(AcError::validation(
                "VERIFY-INVALID_PLAN",
                "validation plan name is required",
            ));
        }
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some("validation".to_string()),
            },
            format!("mem://validation/{}", StableId::new("run")),
            format!("passed:{}", passed),
        )?;
        Ok(ValidationRunReport {
            id: StableId::new("validation"),
            plan_name,
            passed,
            evidence_ref,
            completed_at: TimestampMillis::now(),
        })
    }

    pub fn record_security_scan(
        &self,
        scanner: impl Into<String>,
        findings: usize,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<SecurityScanReport> {
        if self.policy.evaluate(&[Capability::SecurityScan]) != SecurityDecision::Allow {
            return Err(AcError::policy_denied(
                "VERIFY-SCAN_DENIED",
                "security scan requires capability approval",
            ));
        }
        let scanner = scanner.into();
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some(scanner.clone()),
            },
            format!("mem://security/{}", StableId::new("scan")),
            format!("findings:{}", findings),
        )?;
        Ok(SecurityScanReport {
            id: StableId::new("scan"),
            scanner,
            findings,
            evidence_ref,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_capture_requires_capability() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        let err = engine
            .capture_browser_observation("http://localhost", &mut evidence)
            .unwrap_err();
        assert_eq!(err.code(), "VERIFY-BROWSER_DENIED");
    }
}
