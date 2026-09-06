#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyInventoryRecord {
    pub id: StableId,
    pub name: String,
    pub version: String,
    pub license: String,
    pub source: String,
    pub checksum: String,
    pub security_status: String,
    pub vulnerability_refs: Vec<String>,
    pub release_blocking: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyChainRecord {
    pub id: StableId,
    pub component: String,
    pub source: String,
    pub version: String,
    pub checksum: String,
    pub reproducible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretReviewInput {
    pub canary_secret: String,
    pub evidence: String,
    pub report: String,
    pub logs: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityHardeningReport {
    pub id: StableId,
    pub findings: Vec<String>,
    pub mitigations: Vec<String>,
    pub unresolved_risks: Vec<String>,
    pub accepted_limitations: Vec<String>,
    pub release_blocked: bool,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct SecurityHardeningReview {
    dependencies: Vec<DependencyInventoryRecord>,
    supply_chain: Vec<SupplyChainRecord>,
    findings: Vec<String>,
    mitigations: Vec<String>,
    unresolved_risks: Vec<String>,
    accepted_limitations: Vec<String>,
}

impl SecurityHardeningReview {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_dependency(
        &mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        license: impl Into<String>,
        source: impl Into<String>,
        checksum: impl Into<String>,
    ) -> AcResult<DependencyInventoryRecord> {
        let record = DependencyInventoryRecord {
            id: StableId::new("dep"),
            name: non_empty(name.into(), "SECURITY-DEPENDENCY_NAME")?,
            version: non_empty(version.into(), "SECURITY-DEPENDENCY_VERSION")?,
            license: non_empty(license.into(), "SECURITY-DEPENDENCY_LICENSE")?,
            source: non_empty(source.into(), "SECURITY-DEPENDENCY_SOURCE")?,
            checksum: non_empty(checksum.into(), "SECURITY-DEPENDENCY_CHECKSUM")?,
            security_status: "reviewed".to_string(),
            vulnerability_refs: Vec::new(),
            release_blocking: false,
        };
        if record.license == "UNKNOWN" || record.license == "INCOMPATIBLE" {
            self.unresolved_risks
                .push(format!("{} has unresolved license {}", record.name, record.license));
        }
        self.dependencies.push(record.clone());
        Ok(record)
    }

    pub fn record_supply_chain(
        &mut self,
        component: impl Into<String>,
        source: impl Into<String>,
        version: impl Into<String>,
        checksum: impl Into<String>,
    ) -> AcResult<SupplyChainRecord> {
        let record = SupplyChainRecord {
            id: StableId::new("supply"),
            component: non_empty(component.into(), "SECURITY-SUPPLY_COMPONENT")?,
            source: non_empty(source.into(), "SECURITY-SUPPLY_SOURCE")?,
            version: non_empty(version.into(), "SECURITY-SUPPLY_VERSION")?,
            checksum: non_empty(checksum.into(), "SECURITY-SUPPLY_CHECKSUM")?,
            reproducible: false,
        };
        self.supply_chain.push(record.clone());
        Ok(record)
    }

    pub fn validate_secret_review(&mut self, input: SecretReviewInput) -> AcResult<()> {
        if input.canary_secret.is_empty() {
            return Err(AcError::validation(
                "SECURITY-SECRET_CANARY_EMPTY",
                "secret review requires a canary",
            ));
        }
        let leaked = [&input.evidence, &input.report, &input.logs]
            .iter()
            .any(|text| text.contains(&input.canary_secret));
        if leaked {
            self.unresolved_risks
                .push("secret canary appeared in release-visible output".to_string());
            return Err(AcError::policy_denied(
                "SECURITY-SECRET_LEAK",
                "secret canary must not appear in evidence, reports or logs",
            ));
        }
        self.mitigations
            .push("secret canary redaction verified across evidence, reports and logs".to_string());
        Ok(())
    }

    pub fn run_boundary_campaign(
        &mut self,
        workspace_escape_blocked: bool,
        prompt_injection_blocked: bool,
        malicious_skill_blocked: bool,
        malicious_mcp_blocked: bool,
        tool_bypass_blocked: bool,
    ) -> AcResult<()> {
        let checks = [
            ("workspace escape", workspace_escape_blocked),
            ("prompt injection", prompt_injection_blocked),
            ("malicious skill", malicious_skill_blocked),
            ("malicious MCP", malicious_mcp_blocked),
            ("tool permission bypass", tool_bypass_blocked),
        ];
        for (name, passed) in checks {
            if passed {
                self.mitigations.push(format!("{name} regression passed"));
            } else {
                self.unresolved_risks
                    .push(format!("{name} boundary failed release hardening"));
            }
        }
        if self.unresolved_risks.is_empty() {
            Ok(())
        } else {
            Err(AcError::policy_denied(
                "SECURITY-HARDENING_BLOCKED",
                "release hardening has unresolved boundary failures",
            ))
        }
    }

    pub fn generate_sbom(&mut self) -> AcResult<String> {
        if self.dependencies.is_empty() {
            return Err(AcError::validation(
                "SECURITY-SBOM_EMPTY",
                "SBOM requires at least one dependency",
            ));
        }
        let mut lines = vec!["name,version,license,source,checksum".to_string()];
        for dep in &self.dependencies {
            lines.push(format!(
                "{},{},{},{},{}",
                dep.name, dep.version, dep.license, dep.source, dep.checksum
            ));
        }
        self.mitigations.push("SBOM generated".to_string());
        Ok(lines.join("\n"))
    }

    pub fn report(&self) -> SecurityHardeningReport {
        let release_blocked = !self.unresolved_risks.is_empty()
            || self
                .dependencies
                .iter()
                .any(|dep| dep.release_blocking || dep.license == "UNKNOWN");
        SecurityHardeningReport {
            id: StableId::new("hardening"),
            findings: self.findings.clone(),
            mitigations: self.mitigations.clone(),
            unresolved_risks: self.unresolved_risks.clone(),
            accepted_limitations: self.accepted_limitations.clone(),
            release_blocked,
            created_at: TimestampMillis::now(),
        }
    }

    pub fn dependencies(&self) -> &[DependencyInventoryRecord] {
        &self.dependencies
    }

    pub fn supply_chain(&self) -> &[SupplyChainRecord] {
        &self.supply_chain
    }
}

fn non_empty(value: String, code: &'static str) -> AcResult<String> {
    if value.trim().is_empty() {
        Err(AcError::validation(code, "release security field is required"))
    } else {
        Ok(value)
    }
}
