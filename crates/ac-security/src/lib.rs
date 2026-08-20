use std::collections::{BTreeMap, BTreeSet};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Capability {
    FilesystemRead(String),
    FilesystemWrite(String),
    ProcessExec(String),
    Network(String),
    SecretRead(String),
    BrowserAutomation,
    SecurityScan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustTier {
    BuiltIn,
    Project,
    UserInstalled,
    Untrusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecurityDecision {
    Allow,
    Deny,
    RequireApproval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionRecord {
    pub id: StableId,
    pub source: String,
    pub version: String,
    pub trust_tier: TrustTier,
    pub declared_capabilities: BTreeSet<Capability>,
    pub granted_capabilities: BTreeSet<Capability>,
    pub registered_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityFinding {
    pub id: StableId,
    pub scanner: String,
    pub severity: String,
    pub fingerprint: String,
    pub evidence_ref: StableId,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: BTreeMap<StableId, ExtensionRecord>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        source: impl Into<String>,
        version: impl Into<String>,
        trust_tier: TrustTier,
        declared_capabilities: BTreeSet<Capability>,
    ) -> AcResult<StableId> {
        let source = source.into();
        let version = version.into();
        if source.trim().is_empty() || version.trim().is_empty() {
            return Err(AcError::validation(
                "SECURITY-INVALID_EXTENSION",
                "extension source and version are required",
            ));
        }
        let id = StableId::new("ext");
        self.extensions.insert(
            id.clone(),
            ExtensionRecord {
                id: id.clone(),
                source,
                version,
                trust_tier,
                declared_capabilities,
                granted_capabilities: BTreeSet::new(),
                registered_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn grant(&mut self, id: &StableId, capability: Capability) -> AcResult<()> {
        let extension = self.extensions.get_mut(id).ok_or_else(|| {
            AcError::validation("SECURITY-UNKNOWN_EXTENSION", "extension is not registered")
        })?;
        if !extension.declared_capabilities.contains(&capability) {
            return Err(AcError::policy_denied(
                "SECURITY-UNDECLARED_CAPABILITY",
                "extension cannot receive undeclared capability",
            ));
        }
        extension.granted_capabilities.insert(capability);
        Ok(())
    }

    pub fn get(&self, id: &StableId) -> Option<&ExtensionRecord> {
        self.extensions.get(id)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapabilityPolicy {
    denied: BTreeSet<Capability>,
    allowed: BTreeSet<Capability>,
}

impl CapabilityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deny(mut self, capability: Capability) -> Self {
        self.denied.insert(capability);
        self
    }

    pub fn allow(mut self, capability: Capability) -> Self {
        self.allowed.insert(capability);
        self
    }

    pub fn evaluate(&self, requested: &[Capability]) -> SecurityDecision {
        if requested.iter().any(|cap| self.denied.contains(cap)) {
            return SecurityDecision::Deny;
        }
        if requested.iter().all(|cap| self.allowed.contains(cap)) {
            SecurityDecision::Allow
        } else {
            SecurityDecision::RequireApproval
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denied_capability_overrides_allow() {
        let capability = Capability::Network("*".to_string());
        let policy = CapabilityPolicy::new()
            .allow(capability.clone())
            .deny(capability.clone());
        assert_eq!(policy.evaluate(&[capability]), SecurityDecision::Deny);
    }

    #[test]
    fn extension_cannot_gain_undeclared_capability() {
        let mut registry = ExtensionRegistry::new();
        let id = registry
            .register("project-hook", "1", TrustTier::Project, BTreeSet::new())
            .unwrap();
        let err = registry
            .grant(&id, Capability::SecretRead("token".to_string()))
            .unwrap_err();
        assert_eq!(err.code(), "SECURITY-UNDECLARED_CAPABILITY");
    }
}
