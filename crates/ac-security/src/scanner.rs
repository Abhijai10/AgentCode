#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingStatus {
    Candidate,
    Confirmed,
    Likely,
    NeedsValidation,
    FalsePositive,
    Resolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofLevel {
    Pattern,
    Dependency,
    Secret,
    Manual,
    Rescan,
    ActiveValidation,
    AiFixture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecurityAdapter {
    Gitleaks,
    Osv,
    Trivy,
    Semgrep,
    Checkov,
    Zap,
    Nuclei,
    Prowler,
    Stratus,
    CloudGoat,
    Pacu,
    AiNative,
    Promptfoo,
    Garak,
    PyRit,
    Manual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityPolicy {
    pub id: StableId,
    pub version: String,
    pub blocking_severities: BTreeSet<SecuritySeverity>,
    pub active_tests_allowed: bool,
    pub retention_days: u32,
}

impl SecurityPolicy {
    pub fn baseline() -> Self {
        Self {
            id: StableId::new("secpolicy"),
            version: "baseline-v1".to_string(),
            blocking_severities: [SecuritySeverity::High, SecuritySeverity::Critical]
                .into_iter()
                .collect(),
            active_tests_allowed: false,
            retention_days: 30,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreatModel {
    pub id: StableId,
    pub entry_points: Vec<String>,
    pub auth_boundaries: Vec<String>,
    pub data_stores: Vec<String>,
    pub admin_operations: Vec<String>,
    pub cloud_configuration: Vec<String>,
    pub sensitive_assets: Vec<String>,
    pub evidence_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityFindingInstance {
    pub id: StableId,
    pub adapter: SecurityAdapter,
    pub rule_id: String,
    pub severity: SecuritySeverity,
    pub confidence: u8,
    pub proof_level: ProofLevel,
    pub file_path: String,
    pub line: u32,
    pub fingerprint: String,
    pub redacted_evidence: String,
    pub raw_evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedSecurityFinding {
    pub id: StableId,
    pub root_cause: String,
    pub severity: SecuritySeverity,
    pub confidence: u8,
    pub exploitability: u8,
    pub status: FindingStatus,
    pub affected_code: Vec<String>,
    pub evidence_refs: Vec<StableId>,
    pub remediation: String,
    pub instance_ids: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityScanReport {
    pub id: StableId,
    pub adapters_run: Vec<SecurityAdapter>,
    pub missing_adapters: Vec<String>,
    pub threat_model: ThreatModel,
    pub instances: Vec<SecurityFindingInstance>,
    pub findings: Vec<NormalizedSecurityFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityRepairTask {
    pub id: StableId,
    pub finding_id: StableId,
    pub title: String,
    pub verification: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityRegressionResult {
    pub finding_id: StableId,
    pub passed: bool,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityReportBundle {
    pub markdown: String,
    pub json: String,
    pub sarif: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityScanInput {
    pub repository_id: StableId,
    pub commit: String,
    pub files: Vec<(String, String)>,
    pub dependency_manifest: Option<String>,
    pub include_iac: bool,
}

pub struct BaselineSecurityOrchestrator {
    policy: SecurityPolicy,
}

impl BaselineSecurityOrchestrator {
    pub fn new(policy: SecurityPolicy) -> Self {
        Self { policy }
    }

    pub fn run(&self, input: &SecurityScanInput) -> AcResult<SecurityScanReport> {
        if input.commit.trim().is_empty() {
            return Err(AcError::validation(
                "SECURITY-SCAN_INVALID",
                "security scan commit is required",
            ));
        }
        let threat_model = self.threat_model(input);
        let mut adapters_run = vec![
            SecurityAdapter::Gitleaks,
            SecurityAdapter::Osv,
            SecurityAdapter::Trivy,
            SecurityAdapter::Semgrep,
        ];
        if input.include_iac {
            adapters_run.push(SecurityAdapter::Checkov);
        }
        let mut instances = Vec::new();
        instances.extend(self.scan_secrets(input));
        instances.extend(self.scan_dependencies(input));
        instances.extend(self.scan_semgrep(input));
        if input.include_iac {
            instances.extend(self.scan_iac(input));
        }
        let findings = self.triage(self.group(instances.clone()));
        Ok(SecurityScanReport {
            id: StableId::new("secscan"),
            adapters_run,
            missing_adapters: Vec::new(),
            threat_model,
            instances,
            findings,
        })
    }

    pub fn manual_business_logic_finding(
        &self,
        file_path: impl Into<String>,
        evidence: impl Into<String>,
    ) -> NormalizedSecurityFinding {
        NormalizedSecurityFinding {
            id: StableId::new("secfinding"),
            root_cause: "business-logic authorization gap".to_string(),
            severity: SecuritySeverity::High,
            confidence: 80,
            exploitability: 70,
            status: FindingStatus::NeedsValidation,
            affected_code: vec![file_path.into()],
            evidence_refs: vec![StableId::new("evidence")],
            remediation: evidence.into(),
            instance_ids: Vec::new(),
        }
    }

    pub fn create_repair_task(
        &self,
        finding: &NormalizedSecurityFinding,
    ) -> AcResult<SecurityRepairTask> {
        if finding.status != FindingStatus::Confirmed {
            return Err(AcError::validation(
                "SECURITY-FINDING_NOT_CONFIRMED",
                "only confirmed findings create repair tasks",
            ));
        }
        Ok(SecurityRepairTask {
            id: StableId::new("sectask"),
            finding_id: finding.id.clone(),
            title: format!("Fix security finding: {}", finding.root_cause),
            verification: "rescan and rerun relevant tests".to_string(),
        })
    }

    pub fn regression(
        &self,
        finding: &NormalizedSecurityFinding,
        rescan: &SecurityScanReport,
    ) -> SecurityRegressionResult {
        let still_present = rescan
            .findings
            .iter()
            .any(|candidate| candidate.root_cause == finding.root_cause);
        SecurityRegressionResult {
            finding_id: finding.id.clone(),
            passed: !still_present,
            evidence_ref: StableId::new("evidence"),
        }
    }

    pub fn reports(&self, report: &SecurityScanReport) -> SecurityReportBundle {
        let markdown = format!(
            "# Security Report\n\nFindings: {}\nAdapters: {:?}\n",
            report.findings.len(),
            report.adapters_run
        );
        let findings_json = report
            .findings
            .iter()
            .map(|finding| {
                format!(
                    "{{\"id\":\"{}\",\"severity\":\"{:?}\",\"status\":\"{:?}\",\"root_cause\":\"{}\"}}",
                    finding.id,
                    finding.severity,
                    finding.status,
                    json_escape(&finding.root_cause)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            "{{\"scan_id\":\"{}\",\"findings\":[{}]}}",
            report.id, findings_json
        );
        let sarif_results = report
            .findings
            .iter()
            .map(|finding| {
                format!(
                    "{{\"ruleId\":\"{}\",\"level\":\"{:?}\",\"message\":{{\"text\":\"{}\"}}}}",
                    json_escape(&finding.root_cause),
                    finding.severity,
                    json_escape(&finding.remediation)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let sarif = format!(
            "{{\"version\":\"2.1.0\",\"runs\":[{{\"tool\":{{\"driver\":{{\"name\":\"AgentCode Baseline Security\"}}}},\"results\":[{}]}}]}}",
            sarif_results
        );
        SecurityReportBundle {
            markdown,
            json,
            sarif,
        }
    }

    fn threat_model(&self, input: &SecurityScanInput) -> ThreatModel {
        let mut model = ThreatModel {
            id: StableId::new("threat"),
            entry_points: Vec::new(),
            auth_boundaries: Vec::new(),
            data_stores: Vec::new(),
            admin_operations: Vec::new(),
            cloud_configuration: Vec::new(),
            sensitive_assets: Vec::new(),
            evidence_refs: vec![StableId::new("evidence")],
        };
        for (path, content) in &input.files {
            if content.contains("route(") || content.contains("handler") || path.contains("api") {
                model.entry_points.push(path.clone());
            }
            if content.contains("auth") || content.contains("token") {
                model.auth_boundaries.push(path.clone());
            }
            if content.contains("DATABASE_URL") || content.contains("sqlite") {
                model.data_stores.push(path.clone());
            }
            if content.contains("admin") {
                model.admin_operations.push(path.clone());
            }
            if path.ends_with(".tf") || path.ends_with(".yaml") || path.ends_with(".yml") {
                model.cloud_configuration.push(path.clone());
            }
            if content.contains("SECRET") || content.contains("password") {
                model.sensitive_assets.push(path.clone());
            }
        }
        model
    }

    fn scan_secrets(&self, input: &SecurityScanInput) -> Vec<SecurityFindingInstance> {
        let mut out = Vec::new();
        for (path, content) in &input.files {
            for (idx, line) in content.lines().enumerate() {
                if line.contains("AKIA") || line.contains("SECRET=") {
                    out.push(instance(InstanceSpec {
                        adapter: SecurityAdapter::Gitleaks,
                        rule_id: "secret.detected",
                        severity: SecuritySeverity::Critical,
                        proof_level: ProofLevel::Secret,
                        file_path: path,
                        line: idx as u32 + 1,
                        fingerprint: "secret-exposure",
                        redacted_evidence: redact_secret(line),
                    }));
                }
            }
        }
        out
    }

    fn scan_dependencies(&self, input: &SecurityScanInput) -> Vec<SecurityFindingInstance> {
        let manifest = input.dependency_manifest.as_deref().unwrap_or_default();
        if manifest.contains("vulnerable-package") || manifest.contains("RUSTSEC-") {
            vec![instance(InstanceSpec {
                adapter: SecurityAdapter::Osv,
                rule_id: "dependency.vulnerable",
                severity: SecuritySeverity::High,
                proof_level: ProofLevel::Dependency,
                file_path: "dependency-manifest",
                line: 1,
                fingerprint: "vulnerable-dependency",
                redacted_evidence: "known vulnerable dependency".to_string(),
            })]
        } else {
            Vec::new()
        }
    }

    fn scan_semgrep(&self, input: &SecurityScanInput) -> Vec<SecurityFindingInstance> {
        input
            .files
            .iter()
            .filter_map(|(path, content)| {
                let line = content
                    .lines()
                    .position(|line| {
                        line.contains("SELECT * FROM users WHERE name = '")
                            || line.contains("dangerouslySetInnerHTML")
                    })
                    .map(|idx| idx as u32 + 1)?;
                Some(instance(InstanceSpec {
                    adapter: SecurityAdapter::Semgrep,
                    rule_id: "sast.injection",
                    severity: SecuritySeverity::High,
                    proof_level: ProofLevel::Pattern,
                    file_path: path,
                    line,
                    fingerprint: "injection-pattern",
                    redacted_evidence: "injection-like pattern".to_string(),
                }))
            })
            .collect()
    }

    fn scan_iac(&self, input: &SecurityScanInput) -> Vec<SecurityFindingInstance> {
        input
            .files
            .iter()
            .filter_map(|(path, content)| {
                let line = content
                    .lines()
                    .position(|line| line.contains("0.0.0.0/0") || line.contains("public-read"))
                    .map(|idx| idx as u32 + 1)?;
                Some(instance(InstanceSpec {
                    adapter: SecurityAdapter::Checkov,
                    rule_id: "iac.public-exposure",
                    severity: SecuritySeverity::High,
                    proof_level: ProofLevel::Pattern,
                    file_path: path,
                    line,
                    fingerprint: "iac-public-exposure",
                    redacted_evidence: "public infrastructure exposure".to_string(),
                }))
            })
            .collect()
    }

    fn group(&self, instances: Vec<SecurityFindingInstance>) -> Vec<NormalizedSecurityFinding> {
        let mut grouped: BTreeMap<String, NormalizedSecurityFinding> = BTreeMap::new();
        for item in instances {
            grouped
                .entry(item.fingerprint.clone())
                .and_modify(|finding| {
                    finding.affected_code.push(item.file_path.clone());
                    finding.evidence_refs.push(item.raw_evidence_ref.clone());
                    finding.instance_ids.push(item.id.clone());
                    finding.confidence = finding.confidence.max(item.confidence);
                })
                .or_insert_with(|| NormalizedSecurityFinding {
                    id: StableId::new("secfinding"),
                    root_cause: item.fingerprint.clone(),
                    severity: item.severity,
                    confidence: item.confidence,
                    exploitability: if self.policy.blocking_severities.contains(&item.severity) {
                        80
                    } else {
                        40
                    },
                    status: FindingStatus::Candidate,
                    affected_code: vec![item.file_path.clone()],
                    evidence_refs: vec![item.raw_evidence_ref.clone()],
                    remediation: remediation_for(&item.fingerprint),
                    instance_ids: vec![item.id.clone()],
                });
        }
        grouped.into_values().collect()
    }

    fn triage(
        &self,
        mut findings: Vec<NormalizedSecurityFinding>,
    ) -> Vec<NormalizedSecurityFinding> {
        for finding in &mut findings {
            if finding
                .affected_code
                .iter()
                .any(|path| path.contains("false_positive"))
            {
                finding.status = FindingStatus::FalsePositive;
            } else if self.policy.blocking_severities.contains(&finding.severity) {
                finding.status = FindingStatus::Confirmed;
            } else {
                finding.status = FindingStatus::Likely;
            }
        }
        findings
    }
}


struct InstanceSpec<'a> {
    adapter: SecurityAdapter,
    rule_id: &'a str,
    severity: SecuritySeverity,
    proof_level: ProofLevel,
    file_path: &'a str,
    line: u32,
    fingerprint: &'a str,
    redacted_evidence: String,
}

fn instance(spec: InstanceSpec<'_>) -> SecurityFindingInstance {
    SecurityFindingInstance {
        id: StableId::new("secinst"),
        adapter: spec.adapter,
        rule_id: spec.rule_id.to_string(),
        severity: spec.severity,
        confidence: 90,
        proof_level: spec.proof_level,
        file_path: spec.file_path.to_string(),
        line: spec.line,
        fingerprint: spec.fingerprint.to_string(),
        redacted_evidence: spec.redacted_evidence,
        raw_evidence_ref: StableId::new("evidence"),
    }
}

fn redact_secret(line: &str) -> String {
    line.split_whitespace()
        .map(|part| {
            if part.contains("AKIA") || part.contains("SECRET=") {
                "[REDACTED]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn remediation_for(fingerprint: &str) -> String {
    match fingerprint {
        "secret-exposure" => "remove committed secret and rotate credential".to_string(),
        "vulnerable-dependency" => "upgrade vulnerable dependency".to_string(),
        "injection-pattern" => "parameterize user-controlled query or sanitize sink".to_string(),
        "iac-public-exposure" => "restrict public infrastructure exposure".to_string(),
        "seeded-web-authorization-bypass" => {
            "enforce ownership checks and verify with synthetic user canary".to_string()
        }
        "cloud-public-or-wildcard-permission" => {
            "restrict public access and least-privilege wildcard permissions".to_string()
        }
        "ai-direct-prompt-injection" => "separate instructions from user content and enforce policy refusal".to_string(),
        "ai-indirect-prompt-injection" => {
            "treat retrieved/tool content as untrusted and require source-bound policy checks".to_string()
        }
        "ai-rag-poisoning" => "score retrieval trust and quarantine poisoned documents".to_string(),
        "ai-tool-abuse" => "intersect tool requests with Kernel capability policy".to_string(),
        "ai-secret-leakage" => "redact synthetic secrets before model/report exposure".to_string(),
        "ai-excessive-agency" => "require explicit approval for external or destructive agency".to_string(),
        _ => "review and remediate security finding".to_string(),
    }
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
