use std::fs;
use std::path::{Path, PathBuf};

use ac_evidence::{EvidenceStore, Provenance};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SecuritySeverity { Low, Medium, High, Critical }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingStatus { Candidate, Confirmed, Likely, NeedsValidation, FalsePositive, Resolved }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofLevel { Pattern, Dependency, Secret, Manual, Rescan, ActiveValidation, AiFixture, ExternalTool }

/// Named adapters are reserved for output from that external executable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SecurityAdapter {
    Gitleaks, Osv, Trivy, Semgrep, Checkov, Zap,
    BuiltInSecretHeuristic, BuiltInSuspiciousSqlHeuristic, BuiltInIacHeuristic, BuiltInActiveDastHeuristic, BuiltInCloudPostureHeuristic, BuiltInDependencyHeuristic,
    Nuclei, Prowler, Stratus, CloudGoat, Pacu, AiNative, Promptfoo, Garak, PyRit, Manual,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScannerAvailability { Available, Unavailable, Misconfigured, Failed }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScannerNetworkPolicy { Deny, Allow }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScannerConfiguration { pub adapter: SecurityAdapter, pub enabled: bool, pub required: bool, pub executable: String, pub timeout_ms: u64, pub network: ScannerNetworkPolicy, pub rules_path: Option<String>, pub data_dir: Option<PathBuf> }
impl ScannerConfiguration {
    pub fn external(adapter: SecurityAdapter) -> Self {
        let executable = discover_scanner_executable(adapter).unwrap_or_else(|| default_scanner_executable(adapter).to_string());
        Self { adapter, enabled: true, required: false, executable, timeout_ms: 60_000, network: ScannerNetworkPolicy::Deny, rules_path: None, data_dir: None }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScannerDataStatus {
    Ready { path: PathBuf },
    NeedsData { path: PathBuf, reason: String },
    Unsupported,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScannerDataOperation {
    PrepareScannerData,
    VerificationScan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScannerDataManager {
    root: PathBuf,
}
impl ScannerDataManager {
    pub fn new(root: PathBuf) -> Self { Self { root } }
    pub fn default_runtime() -> Self { Self::new(default_scanner_data_root()) }
    pub fn managed_path(&self, adapter: SecurityAdapter) -> PathBuf { self.root.join(scanner_data_name(adapter)) }
    pub fn status(&self, adapter: SecurityAdapter) -> ScannerDataStatus {
        match adapter {
            SecurityAdapter::Trivy => {
                let path = self.managed_path(adapter);
                if path.join("db/trivy.db").is_file() && path.join("db/metadata.json").is_file() {
                    ScannerDataStatus::Ready { path }
                } else {
                    ScannerDataStatus::NeedsData { path, reason: "Trivy database is not prepared in the managed cache".to_string() }
                }
            }
            SecurityAdapter::Osv => {
                let path = self.managed_path(adapter);
                if path.exists() {
                    ScannerDataStatus::Ready { path }
                } else {
                    ScannerDataStatus::NeedsData { path, reason: "OSV offline database is not prepared".to_string() }
                }
            }
            _ => ScannerDataStatus::Unsupported,
        }
    }
    pub fn verification_config(&self, adapter: SecurityAdapter) -> ScannerConfiguration {
        let mut config = ScannerConfiguration::external(adapter);
        config.data_dir = Some(self.managed_path(adapter));
        config.network = ScannerNetworkPolicy::Deny;
        config
    }
    pub fn prepare_request(&self, adapter: SecurityAdapter, workspace_root: &Path) -> AcResult<ScannerProcessRequest> {
        let mut config = ScannerConfiguration::external(adapter);
        config.data_dir = Some(self.managed_path(adapter));
        scanner_data_prepare_request(&config, workspace_root)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScannerCapabilities { pub scan_kinds: Vec<String>, pub requires_target_authorization: bool }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScannerProcessRequest { pub adapter: SecurityAdapter, pub executable: String, pub argv: Vec<String>, pub cwd: PathBuf, pub timeout_ms: u64, pub network: bool, pub cleanup_paths: Vec<PathBuf>, pub report_path: Option<PathBuf>, pub env: BTreeMap<String, String> }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScannerProcessResult { pub exit_code: Option<i32>, pub stdout: String, pub stderr: String, pub stdout_truncated: bool, pub stderr_truncated: bool }
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScannerFailure { Unavailable(String), Misconfigured(String), Timeout, Cancelled, ExecutionFailed(String), OutputMalformed(String), NetworkDenied, TargetUnauthorized }
pub trait SecurityScannerExecutor { fn execute(&self, request: ScannerProcessRequest) -> Result<ScannerProcessResult, ScannerFailure>; }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScannerExecution { pub adapter: SecurityAdapter, pub availability: ScannerAvailability, pub version: Option<String>, pub raw_evidence_ref: Option<StableId>, pub source_commit: String, pub failure: Option<ScannerFailure> }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityPolicy { pub id: StableId, pub version: String, pub blocking_severities: BTreeSet<SecuritySeverity>, pub active_tests_allowed: bool, pub retention_days: u32 }
impl SecurityPolicy { pub fn baseline() -> Self { Self { id: StableId::new("secpolicy"), version: "baseline-v1".to_string(), blocking_severities: [SecuritySeverity::High, SecuritySeverity::Critical].into_iter().collect(), active_tests_allowed: false, retention_days: 30 } } }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreatModel { pub id: StableId, pub entry_points: Vec<String>, pub auth_boundaries: Vec<String>, pub data_stores: Vec<String>, pub admin_operations: Vec<String>, pub cloud_configuration: Vec<String>, pub sensitive_assets: Vec<String>, pub evidence_refs: Vec<StableId> }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityFindingInstance { pub id: StableId, pub adapter: SecurityAdapter, pub rule_id: String, pub severity: SecuritySeverity, pub confidence: u8, pub proof_level: ProofLevel, pub file_path: String, pub line: u32, pub fingerprint: String, pub redacted_evidence: String, pub raw_evidence_ref: StableId }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedSecurityFinding { pub id: StableId, pub root_cause: String, pub severity: SecuritySeverity, pub confidence: u8, pub exploitability: u8, pub status: FindingStatus, pub affected_code: Vec<String>, pub evidence_refs: Vec<StableId>, pub remediation: String, pub instance_ids: Vec<StableId> }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityScanReport { pub id: StableId, pub adapters_run: Vec<SecurityAdapter>, pub missing_adapters: Vec<String>, pub threat_model: ThreatModel, pub instances: Vec<SecurityFindingInstance>, pub findings: Vec<NormalizedSecurityFinding>, pub executions: Vec<ScannerExecution> }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityRepairTask { pub id: StableId, pub finding_id: StableId, pub title: String, pub verification: String }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityRegressionResult { pub finding_id: StableId, pub passed: bool, pub evidence_ref: StableId }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityReportBundle { pub markdown: String, pub json: String, pub sarif: String }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityScanInput { pub repository_id: StableId, pub commit: String, pub files: Vec<(String, String)>, pub dependency_manifest: Option<String>, pub include_iac: bool }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedSecurityScanInput { pub repository_id: StableId, pub commit: String, pub workspace_root: PathBuf, pub configurations: Vec<ScannerConfiguration>, pub target_url: Option<String>, pub target_authorized: bool }

pub struct BaselineSecurityOrchestrator { policy: SecurityPolicy }
impl BaselineSecurityOrchestrator {
    pub fn new(policy: SecurityPolicy) -> Self { Self { policy } }

    /// Lightweight editor-content signals. They never claim an external scanner identity.
    pub fn run(&self, input: &SecurityScanInput) -> AcResult<SecurityScanReport> {
        if input.commit.trim().is_empty() { return Err(AcError::validation("SECURITY-SCAN_INVALID", "security scan commit is required")); }
        let threat_model = self.threat_model(input);
        let mut instances = self.secret_heuristic(input);
        instances.extend(self.sql_heuristic(input));
        if input.include_iac { instances.extend(self.iac_heuristic(input)); }
        if let Some(manifest) = input.dependency_manifest.as_deref() { instances.extend(self.dependency_heuristic(manifest)); }
        let findings = self.triage(self.group(instances.clone()));
        Ok(SecurityScanReport { id: StableId::new("secscan"), adapters_run: instances.iter().map(|i| i.adapter).collect(), missing_adapters: Vec::new(), threat_model, instances, findings, executions: Vec::new() })
    }

    /// Runs external scanners through a caller-supplied governed executor. A required
    /// unavailable scanner is fail-closed; optional scanners are reported honestly.
    pub fn run_managed(&self, input: &ManagedSecurityScanInput, executor: &dyn SecurityScannerExecutor, evidence: &mut EvidenceStore) -> AcResult<SecurityScanReport> {
        if input.commit.trim().is_empty() || !input.workspace_root.is_dir() { return Err(AcError::validation("SECURITY-SCAN_INVALID", "security scan needs commit and workspace")); }
        let mut report = SecurityScanReport { id: StableId::new("secscan"), adapters_run: Vec::new(), missing_adapters: Vec::new(), threat_model: empty_threat_model(), instances: Vec::new(), findings: Vec::new(), executions: Vec::new() };
        for config in input.configurations.iter().filter(|c| c.enabled) {
            // Per-scanner configuration problems (a missing executable, a
            // semgrep without local rules, an unauthorized ZAP target) are
            // recorded honestly against THAT scanner — they never abort the
            // whole sweep.  Only a REQUIRED misconfigured scanner fails the
            // run (via record_failure).
            if let Err(error) = validate_config(config, input) {
                self.record_failure(&mut report, config, input, ScannerFailure::Misconfigured(error.to_string()))?;
                continue;
            }
            let version = match executor.execute(version_request(config, &input.workspace_root)) { Ok(out) if out.exit_code == Some(0) => scanner_version(config.adapter, &out.stdout), Ok(out) => { self.record_failure(&mut report, config, input, ScannerFailure::Unavailable(redact_output(&out.stderr)))?; continue; }, Err(failure) => { self.record_failure(&mut report, config, input, failure)?; continue; } };
            let scan = match scan_request(config, input) {
                Ok(scan) => scan,
                Err(error) => {
                    self.record_failure(&mut report, config, input, ScannerFailure::Misconfigured(error.to_string()))?;
                    continue;
                }
            };
            let output = match executor.execute(scan) { Ok(out) if scan_exit_is_result(config.adapter, out.exit_code) => out, Ok(out) => { self.record_failure(&mut report, config, input, ScannerFailure::ExecutionFailed(redact_output(&out.stderr)))?; continue; }, Err(failure) => { self.record_failure(&mut report, config, input, failure)?; continue; } };
            if output.stdout_truncated || output.stderr_truncated { return Err(AcError::new("SECURITY-SCANNER_OUTPUT_TRUNCATED", "scanner output exceeded governed limit", ErrorKind::Unavailable, Retryability::NotRetryable)); }
            let raw = redact_output(&output.stdout);
            let evidence_ref = evidence.append_tool_output(Provenance { source: "security-orchestrator".to_string(), commit: Some(input.commit.clone()), worktree: Some(input.workspace_root.display().to_string()), tool: Some(adapter_name(config.adapter).to_string()) }, format!("mem://security/{}/{}", adapter_name(config.adapter), StableId::new("raw")), raw.clone(), &[])?;
            report.instances.extend(parse_external_output(config.adapter, &raw, evidence_ref.clone())?);
            report.adapters_run.push(config.adapter);
            report.executions.push(ScannerExecution { adapter: config.adapter, availability: ScannerAvailability::Available, version: Some(version), raw_evidence_ref: Some(evidence_ref), source_commit: input.commit.clone(), failure: None });
        }
        report.findings = self.triage(self.group(report.instances.clone()));
        Ok(report)
    }

    /// Merges a managed external-scanner report into a baseline heuristic
    /// report: combined instances are re-grouped and re-triaged so external
    /// findings flow through the same normalization pipeline, adapter
    /// coverage and availability are unioned, and the baseline threat model
    /// (built from file content) is preserved.
    pub fn merge_reports(&self, baseline: SecurityScanReport, managed: SecurityScanReport) -> SecurityScanReport {
        let threat_model = baseline.threat_model.clone();
        let mut adapters_run = baseline.adapters_run.clone();
        for adapter in managed.adapters_run.iter() {
            if !adapters_run.contains(adapter) {
                adapters_run.push(*adapter);
            }
        }
        let mut instances = baseline.instances.clone();
        instances.extend(managed.instances.iter().cloned());
        let findings = self.triage(self.group(instances.clone()));
        SecurityScanReport {
            id: baseline.id,
            adapters_run,
            missing_adapters: managed.missing_adapters.clone(),
            threat_model,
            instances,
            findings,
            executions: managed.executions.clone(),
        }
    }
    fn record_failure(&self, report: &mut SecurityScanReport, config: &ScannerConfiguration, input: &ManagedSecurityScanInput, failure: ScannerFailure) -> AcResult<SecurityScanReport> { let availability = match failure { ScannerFailure::Unavailable(_) => ScannerAvailability::Unavailable, ScannerFailure::Misconfigured(_) => ScannerAvailability::Misconfigured, _ => ScannerAvailability::Failed }; let reason = format!("{}:{:?}", adapter_name(config.adapter), failure); report.missing_adapters.push(reason.clone()); report.executions.push(ScannerExecution { adapter: config.adapter, availability, version: None, raw_evidence_ref: None, source_commit: input.commit.clone(), failure: Some(failure) }); if config.required { return Err(AcError::new("SECURITY-SCANNER_REQUIRED_UNAVAILABLE", reason, ErrorKind::Unavailable, Retryability::NotRetryable)); } Ok(report.clone()) }
    pub fn manual_business_logic_finding(&self, file_path: impl Into<String>, evidence: impl Into<String>) -> NormalizedSecurityFinding { NormalizedSecurityFinding { id: StableId::new("secfinding"), root_cause: "business-logic authorization gap".to_string(), severity: SecuritySeverity::High, confidence: 80, exploitability: 70, status: FindingStatus::NeedsValidation, affected_code: vec![file_path.into()], evidence_refs: vec![StableId::new("evidence")], remediation: evidence.into(), instance_ids: Vec::new() } }
    pub fn create_repair_task(&self, finding: &NormalizedSecurityFinding) -> AcResult<SecurityRepairTask> { if finding.status != FindingStatus::Confirmed { return Err(AcError::validation("SECURITY-FINDING_NOT_CONFIRMED", "only confirmed findings create repair tasks")); } Ok(SecurityRepairTask { id: StableId::new("sectask"), finding_id: finding.id.clone(), title: format!("Fix security finding: {}", finding.root_cause), verification: "rescan and rerun relevant tests".to_string() }) }
    pub fn regression(&self, finding: &NormalizedSecurityFinding, rescan: &SecurityScanReport) -> SecurityRegressionResult { SecurityRegressionResult { finding_id: finding.id.clone(), passed: !rescan.findings.iter().any(|f| f.root_cause == finding.root_cause), evidence_ref: StableId::new("evidence") } }
    pub fn reports(&self, report: &SecurityScanReport) -> SecurityReportBundle { let json = format!("{{\"scan_id\":\"{}\",\"findings\":{}}}", report.id, report.findings.len()); let sarif = format!("{{\"version\":\"2.1.0\",\"runs\":[{{\"tool\":{{\"driver\":{{\"name\":\"AgentCode Security\"}}}},\"results\":[{}]}}]}}", report.instances.iter().map(|i| format!("{{\"ruleId\":\"{}\",\"level\":\"{:?}\",\"message\":{{\"text\":\"{}\"}}}}", json_escape(&i.rule_id), i.severity, json_escape(&i.redacted_evidence))).collect::<Vec<_>>().join(",")); SecurityReportBundle { markdown: format!("# Security Report\n\nFindings: {}\nAdapters: {:?}\n", report.findings.len(), report.adapters_run), json, sarif } }
    #[allow(clippy::possible_missing_else)]
    fn threat_model(&self, input: &SecurityScanInput) -> ThreatModel { let mut out = empty_threat_model(); for (path, content) in &input.files { if content.contains("route(") || content.contains("handler") || path.contains("api") { out.entry_points.push(path.clone()); } if content.contains("auth") || content.contains("token") { out.auth_boundaries.push(path.clone()); } if content.contains("DATABASE_URL") || content.contains("sqlite") { out.data_stores.push(path.clone()); } if content.contains("admin") { out.admin_operations.push(path.clone()); } if path.ends_with(".tf") || path.ends_with(".yaml") || path.ends_with(".yml") { out.cloud_configuration.push(path.clone()); } if content.contains("SECRET") || content.contains("password") { out.sensitive_assets.push(path.clone()); } } out }
    fn secret_heuristic(&self, input: &SecurityScanInput) -> Vec<SecurityFindingInstance> { input.files.iter().flat_map(|(path, content)| content.lines().enumerate().filter(|(_, line)| line.contains("AKIA") || line.contains("SECRET=")).map(move |(n, line)| external_instance(SecurityAdapter::BuiltInSecretHeuristic, "builtin.secret-pattern", SecuritySeverity::Critical, ProofLevel::Pattern, path, n as u32 + 1, "builtin-secret-pattern", redact_output(line), StableId::new("evidence")))).collect() }
    fn sql_heuristic(&self, input: &SecurityScanInput) -> Vec<SecurityFindingInstance> { input.files.iter().filter_map(|(path, content)| content.lines().position(|line| line.contains("SELECT * FROM users WHERE name = '") || line.contains("dangerouslySetInnerHTML")).map(|n| external_instance(SecurityAdapter::BuiltInSuspiciousSqlHeuristic, "builtin.suspicious-sink", SecuritySeverity::High, ProofLevel::Pattern, path, n as u32 + 1, "builtin-suspicious-sink", "suspicious source sink pattern".to_string(), StableId::new("evidence")))).collect() }
    fn iac_heuristic(&self, input: &SecurityScanInput) -> Vec<SecurityFindingInstance> { input.files.iter().filter_map(|(path, content)| content.lines().position(|line| line.contains("0.0.0.0/0") || line.contains("public-read")).map(|n| external_instance(SecurityAdapter::BuiltInIacHeuristic, "builtin.iac-public-exposure", SecuritySeverity::High, ProofLevel::Pattern, path, n as u32 + 1, "builtin-iac-public-exposure", "public infrastructure exposure heuristic".to_string(), StableId::new("evidence")))).collect() }
    /// Real dependency-manifest analysis (G5): parses the project's actual
    /// lockfile/manifest content for declared dependency versions and flags
    /// KNOWN-VULNERABLE versions from the bundled advisory list.  A
    /// version with no advisory is never flagged.  Version strings never
    /// enter evidence beyond the name+version pair.
    fn dependency_heuristic(&self, manifest: &str) -> Vec<SecurityFindingInstance> {
        let mut out = Vec::new();
        for (name, version) in parse_manifest_dependencies(manifest) {
            if let Some(advisory) = known_vulnerable_dependency(&name, &version) {
                out.push(external_instance(
                    SecurityAdapter::BuiltInDependencyHeuristic,
                    "builtin.dependency-advisory",
                    advisory.severity,
                    ProofLevel::Pattern,
                    &advisory.evidence_file,
                    1,
                    &format!("builtin-dependency-{}-{}", name, version),
                    format!(
                        "{} {} matches a known-vulnerable range: {} ({})| remediation: {}",
                        name, version, advisory.summary, advisory.advisory_id, advisory.remediation
                    ),
                    StableId::new("evidence"),
                ));
            }
        }
        out
    }
    fn group(&self, items: Vec<SecurityFindingInstance>) -> Vec<NormalizedSecurityFinding> { let mut groups = BTreeMap::new(); for item in items { let key = format!("{}:{}:{}", item.file_path, item.rule_id, item.fingerprint); groups.entry(key).and_modify(|f: &mut NormalizedSecurityFinding| { f.affected_code.push(item.file_path.clone()); f.evidence_refs.push(item.raw_evidence_ref.clone()); f.instance_ids.push(item.id.clone()); f.confidence = f.confidence.max(item.confidence); f.severity = f.severity.max(item.severity); }).or_insert_with(|| NormalizedSecurityFinding { id: StableId::new("secfinding"), root_cause: item.fingerprint.clone(), severity: item.severity, confidence: item.confidence, exploitability: if self.policy.blocking_severities.contains(&item.severity) { 80 } else { 40 }, status: FindingStatus::Candidate, affected_code: vec![item.file_path.clone()], evidence_refs: vec![item.raw_evidence_ref.clone()], remediation: if item.fingerprint.starts_with("builtin-dependency") { // The advisory remediation is carried in the instance evidence after the "| remediation: " marker.
 item.redacted_evidence.split("| remediation: ").nth(1).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| remediation_for(&item.fingerprint)) } else { remediation_for(&item.fingerprint) }, instance_ids: vec![item.id.clone()] }); } groups.into_values().collect() }
    fn triage(&self, mut findings: Vec<NormalizedSecurityFinding>) -> Vec<NormalizedSecurityFinding> { for f in &mut findings { f.status = if self.policy.blocking_severities.contains(&f.severity) { FindingStatus::NeedsValidation } else { FindingStatus::Candidate }; } findings }
}

#[allow(clippy::possible_missing_else)]
fn validate_config(config: &ScannerConfiguration, input: &ManagedSecurityScanInput) -> AcResult<()> { if !is_external(config.adapter) || config.executable.trim().is_empty() || config.timeout_ms == 0 { return Err(AcError::validation("SECURITY-SCANNER_MISCONFIGURED", "scanner executable and timeout are required")); } if config.adapter == SecurityAdapter::Zap && (!input.target_authorized || !input.target_url.as_deref().is_some_and(|url| url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost"))) { return Err(AcError::policy_denied("SECURITY-ZAP_TARGET_UNAUTHORIZED", "ZAP requires an explicitly authorized localhost target")); } Ok(()) }
fn is_external(adapter: SecurityAdapter) -> bool { matches!(adapter, SecurityAdapter::Gitleaks | SecurityAdapter::Osv | SecurityAdapter::Trivy | SecurityAdapter::Semgrep | SecurityAdapter::Checkov | SecurityAdapter::Zap) }
fn version_request(config: &ScannerConfiguration, cwd: &Path) -> ScannerProcessRequest {
    let version_arg = if config.adapter == SecurityAdapter::Zap { "-version" } else { "--version" };
    ScannerProcessRequest { adapter: config.adapter, executable: config.executable.clone(), argv: vec![config.executable.clone(), version_arg.to_string()], cwd: cwd.to_path_buf(), timeout_ms: config.timeout_ms.min(10_000), network: false, cleanup_paths: Vec::new(), report_path: None, env: BTreeMap::new() }
}
fn scan_request(c: &ScannerConfiguration, input: &ManagedSecurityScanInput) -> AcResult<ScannerProcessRequest> {
    let root = input.workspace_root.display().to_string();
    let mut cleanup_paths = Vec::new();
    let mut report_path = None;
    let mut env = BTreeMap::new();
    let argv = match c.adapter {
        SecurityAdapter::Gitleaks => {
            // /dev/stdout is not a writable report target on macOS hosts
            // (gitleaks fails with "Report path is not writable"), so the
            // report goes to a governed temp file the executor reads back
            // and removes afterwards.
            let report = std::env::temp_dir().join(format!("agentcode-gitleaks-{}.json", StableId::new("gl")));
            cleanup_paths.push(report.clone());
            report_path = Some(report.clone());
            vec![c.executable.clone(), "detect".to_string(), "--source".to_string(), root, "--report-format".to_string(), "json".to_string(), "--report-path".to_string(), report.display().to_string(), "--no-banner".to_string(), "--exit-code".to_string(), "0".to_string()]
        }
        SecurityAdapter::Osv => {
            let data = scanner_data_dir(c, "osv");
            if !data.exists() {
                return Err(scanner_data_unavailable("OSV offline database is not prepared"));
            }
            env.insert("OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY".to_string(), data.join("osv").display().to_string());
            env.insert("OSV_SCALIBR_LOCAL_DB_CACHE_DIRECTORY".to_string(), data.join("scalibr").display().to_string());
            vec![c.executable.clone(), "scan".to_string(), "source".to_string(), "--offline".to_string(), "--offline-vulnerabilities".to_string(), "--format".to_string(), "json".to_string(), root]
        },
        SecurityAdapter::Trivy => {
            let data = scanner_data_dir(c, "trivy");
            if !data.join("db/trivy.db").is_file() || !data.join("db/metadata.json").is_file() {
                return Err(scanner_data_unavailable("Trivy database is not prepared in the managed cache"));
            }
            vec![c.executable.clone(), "--cache-dir".to_string(), data.display().to_string(), "fs".to_string(), "--format".to_string(), "json".to_string(), "--offline-scan".to_string(), "--skip-db-update".to_string(), "--skip-java-db-update".to_string(), root]
        },
        SecurityAdapter::Semgrep => {
            let rules = c.rules_path.clone().ok_or_else(|| AcError::policy_denied("SECURITY-SEMGREP_RULES_REQUIRED", "Semgrep requires a configured local rules path"))?;
            vec![c.executable.clone(), "scan".to_string(), "--json".to_string(), "--config".to_string(), rules, root]
        },
        SecurityAdapter::Checkov => vec![c.executable.clone(), "-d".to_string(), root, "-o".to_string(), "json".to_string()],
        SecurityAdapter::Zap => {
            let zap_home = std::env::temp_dir().join(format!("agentcode-zap-home-{}", StableId::new("zap")));
            fs::create_dir_all(&zap_home).map_err(|error| AcError::new("SECURITY-ZAP_HOME_CREATE", error.to_string(), ErrorKind::Unavailable, Retryability::NotRetryable))?;
            cleanup_paths.push(zap_home.clone());
            vec![c.executable.clone(), "-cmd".to_string(), "-silent".to_string(), "-notel".to_string(), "-dir".to_string(), zap_home.display().to_string(), "-zapit".to_string(), input.target_url.clone().unwrap_or_default()]
        },
        _ => return Err(AcError::validation("SECURITY-SCANNER_MISCONFIGURED", "unsupported scanner")),
    };
    Ok(ScannerProcessRequest { adapter: c.adapter, executable: c.executable.clone(), argv, cwd: input.workspace_root.clone(), timeout_ms: c.timeout_ms, network: c.network == ScannerNetworkPolicy::Allow, cleanup_paths, report_path, env })
}
fn scanner_data_prepare_request(c: &ScannerConfiguration, workspace_root: &Path) -> AcResult<ScannerProcessRequest> {
    let data = scanner_data_dir(c, scanner_data_name(c.adapter));
    let mut env = BTreeMap::new();
    let argv = match c.adapter {
        SecurityAdapter::Trivy => vec![
            c.executable.clone(),
            "--cache-dir".to_string(),
            data.display().to_string(),
            "image".to_string(),
            "--download-db-only".to_string(),
        ],
        SecurityAdapter::Osv => {
            fs::create_dir_all(&data).map_err(|error| AcError::new("SECURITY-SCANNER_DATA_PREPARE_FAILED", error.to_string(), ErrorKind::Unavailable, Retryability::NotRetryable))?;
            env.insert("OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY".to_string(), data.join("osv").display().to_string());
            env.insert("OSV_SCALIBR_LOCAL_DB_CACHE_DIRECTORY".to_string(), data.join("scalibr").display().to_string());
            vec![
                c.executable.clone(),
                "scan".to_string(),
                "source".to_string(),
                "--offline-vulnerabilities".to_string(),
                "--download-offline-databases".to_string(),
                "--format".to_string(),
                "json".to_string(),
                workspace_root.display().to_string(),
            ]
        }
        _ => return Err(AcError::validation("SECURITY-SCANNER_DATA_UNSUPPORTED", "scanner data preparation is only supported for Trivy and OSV")),
    };
    Ok(ScannerProcessRequest {
        adapter: c.adapter,
        executable: c.executable.clone(),
        argv,
        cwd: workspace_root.to_path_buf(),
        timeout_ms: c.timeout_ms.max(180_000),
        network: true,
        cleanup_paths: Vec::new(),
        report_path: None,
        env,
    })
}
fn scanner_data_dir(config: &ScannerConfiguration, scanner: &str) -> PathBuf {
    config
        .data_dir
        .clone()
        .unwrap_or_else(|| default_scanner_data_root().join(scanner))
}
fn scanner_data_name(adapter: SecurityAdapter) -> &'static str {
    match adapter {
        SecurityAdapter::Osv => "osv-scanner",
        SecurityAdapter::Trivy => "trivy",
        _ => adapter_name(adapter),
    }
}
fn default_scanner_data_root() -> PathBuf {
    std::env::var_os("AGENTCODE_SCANNER_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("AGENTCODE_RUNTIME_DIR").map(|dir| PathBuf::from(dir).join("scanner-data")))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support/AgentCode/runtime/scanner-data")))
        .unwrap_or_else(|| std::env::temp_dir().join("agentcode-scanner-data"))
}
fn scanner_data_unavailable(message: &'static str) -> AcError {
    AcError::new("SECURITY-SCANNER_DATA_UNAVAILABLE", message, ErrorKind::Unavailable, Retryability::NotRetryable)
}
fn default_scanner_executable(adapter: SecurityAdapter) -> &'static str {
    match adapter {
        SecurityAdapter::Gitleaks => "gitleaks",
        SecurityAdapter::Osv => "osv-scanner",
        SecurityAdapter::Trivy => "trivy",
        SecurityAdapter::Semgrep => "semgrep",
        SecurityAdapter::Checkov => "checkov",
        SecurityAdapter::Zap => "zap.sh",
        _ => "",
    }
}
fn discover_scanner_executable(adapter: SecurityAdapter) -> Option<String> {
    let override_key = match adapter {
        SecurityAdapter::Gitleaks => "AGENTCODE_GITLEAKS_EXECUTABLE",
        SecurityAdapter::Osv => "AGENTCODE_OSV_SCANNER_EXECUTABLE",
        SecurityAdapter::Trivy => "AGENTCODE_TRIVY_EXECUTABLE",
        SecurityAdapter::Semgrep => "AGENTCODE_SEMGREP_EXECUTABLE",
        SecurityAdapter::Checkov => "AGENTCODE_CHECKOV_EXECUTABLE",
        SecurityAdapter::Zap => "AGENTCODE_ZAP_EXECUTABLE",
        _ => "",
    };
    if !override_key.is_empty() {
        if let Ok(value) = std::env::var(override_key) {
            let path = PathBuf::from(value);
            if path.is_file() {
                return Some(path.display().to_string());
            }
        }
    }
    let name = default_scanner_executable(adapter);
    let mut candidates = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .collect::<Vec<_>>();
    if adapter == SecurityAdapter::Zap {
        candidates.push(PathBuf::from("/Applications/ZAP.app/Contents/Java/zap.sh"));
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.display().to_string())
}
fn scan_exit_is_result(adapter: SecurityAdapter, code: Option<i32>) -> bool { code == Some(0) || matches!(adapter, SecurityAdapter::Gitleaks | SecurityAdapter::Osv | SecurityAdapter::Semgrep | SecurityAdapter::Checkov) && code == Some(1) }
fn parse_external_output(adapter: SecurityAdapter, raw: &str, evidence: StableId) -> AcResult<Vec<SecurityFindingInstance>> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    if adapter == SecurityAdapter::Zap {
        return Ok(normalize_zapit_output(raw, evidence));
    }
    let value: Value = serde_json::from_str(raw).map_err(|e| AcError::new("SECURITY-SCANNER_OUTPUT_MALFORMED", e.to_string(), ErrorKind::Validation, Retryability::NotRetryable))?;
    if adapter == SecurityAdapter::Osv {
        return Ok(normalize_osv_output(&value, evidence));
    }
    let items = match adapter { SecurityAdapter::Gitleaks => value.as_array().cloned().unwrap_or_default(), SecurityAdapter::Semgrep => at(&value, &["results"]), SecurityAdapter::Trivy => value.get("Results").and_then(Value::as_array).into_iter().flatten().flat_map(|r| ["Vulnerabilities", "Misconfigurations"].into_iter().flat_map(|k| at(r, &[k]))).collect(), SecurityAdapter::Checkov => at(&value, &["results", "failed_checks"]), SecurityAdapter::Zap => value.get("site").and_then(Value::as_array).into_iter().flatten().flat_map(|s| at(s, &["alerts"])).collect(), _ => Vec::new() };
    Ok(items.iter().map(|v| normalize_item(adapter, v, evidence.clone())).collect())
}
fn normalize_zapit_output(raw: &str, evidence: StableId) -> Vec<SecurityFindingInstance> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            let (severity_text, rest) = line.split_once(':')?;
            let severity = match severity_text {
                "Critical" | "High" | "Medium" | "Low" | "Informational" => {
                    severity(Some(severity_text))
                }
                _ => return None,
            };
            let rule = rest
                .split(':')
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("ZAP alert");
            let fingerprint = format!("zap:{rule}:localhost");
            Some(external_instance(
                SecurityAdapter::Zap,
                rule,
                severity,
                ProofLevel::ExternalTool,
                "localhost",
                1,
                &fingerprint,
                redact_output(line),
                evidence.clone(),
            ))
        })
        .collect()
}
fn normalize_osv_output(value: &Value, evidence: StableId) -> Vec<SecurityFindingInstance> {
    let mut instances = Vec::new();
    for result in at(value, &["results"]) {
        let source = result
            .get("source")
            .and_then(|source| field(source, &["path"]))
            .unwrap_or_else(|| "workspace".to_string());
        for package in at(&result, &["packages"]) {
            let package_name = package
                .get("package")
                .and_then(|package| field(package, &["name"]))
                .unwrap_or_else(|| "unknown-package".to_string());
            let version = field(&package, &["version"]).unwrap_or_else(|| "unknown".to_string());
            for vulnerability in at(&package, &["vulnerabilities"]) {
                let rule = field(&vulnerability, &["id", "modified"]).unwrap_or_else(|| "OSV".to_string());
                let title = field(&vulnerability, &["summary", "details"]).unwrap_or_else(|| "OSV advisory".to_string());
                let severity = vulnerability
                    .get("database_specific")
                    .and_then(|specific| field(specific, &["severity"]))
                    .map(|severity| severity.to_ascii_uppercase())
                    .filter(|severity| severity != "INFORMATIONAL")
                    .map(|severity| self::severity(Some(&severity)))
                    .unwrap_or(SecuritySeverity::Low);
                let fingerprint = format!("osv-scanner:{}:{}:{}", package_name, version, rule);
                instances.push(external_instance(SecurityAdapter::Osv, &rule, severity, ProofLevel::ExternalTool, &source, 1, &fingerprint, redact_output(&title), evidence.clone()));
            }
        }
    }
    instances
}
#[allow(clippy::manual_try_fold)]
fn at(value: &Value, path: &[&str]) -> Vec<Value> { path.iter().fold(Some(value), |v, p| v.and_then(|v| v.get(*p))).and_then(Value::as_array).cloned().unwrap_or_default() }
fn normalize_item(adapter: SecurityAdapter, v: &Value, evidence: StableId) -> SecurityFindingInstance { let rule = field(v, &["RuleID", "check_id", "VulnerabilityID", "alertRef", "pluginId"]).unwrap_or_else(|| adapter_name(adapter).to_string()); let file = field(v, &["File", "path", "file_path", "Target", "resource", "url", "uri"]).unwrap_or_else(|| "workspace".to_string()); let line = number(v, &["StartLine", "line", "line_number"]).unwrap_or(1); let title = field(v, &["Description", "message", "check_name", "Title", "alert", "name"]).unwrap_or_else(|| "external scanner finding".to_string()); let severity = severity(field(v, &["Severity", "severity", "risk", "riskcode"]).as_deref()); let fingerprint = format!("{}:{}:{}:{}", adapter_name(adapter), rule, file, line); external_instance(adapter, &rule, severity, ProofLevel::ExternalTool, &file, line, &fingerprint, redact_output(&title), evidence) }
fn field(v: &Value, names: &[&str]) -> Option<String> { names.iter().find_map(|n| v.get(*n).and_then(Value::as_str).map(ToString::to_string)) }
fn number(v: &Value, names: &[&str]) -> Option<u32> { names.iter().find_map(|n| v.get(*n).and_then(Value::as_u64).map(|x| x as u32)) }
fn severity(value: Option<&str>) -> SecuritySeverity { match value.unwrap_or_default().to_ascii_uppercase().as_str() { "CRITICAL" | "4" => SecuritySeverity::Critical, "HIGH" | "ERROR" | "3" => SecuritySeverity::High, "MEDIUM" | "WARNING" | "2" => SecuritySeverity::Medium, _ => SecuritySeverity::Low } }

/// Extract (name, version) pairs from a real dependency lockfile or
/// manifest.  Recognized formats: Cargo.lock/Cargo.toml (`name = "x"`),
/// package-lock.json (`"x": "1.2.3"`), pnpm/yarn locks (`'x@1.2.3'`),
/// requirements.txt (`x==1.2.3`), go.mod (`mod v1.2.3`).
pub fn parse_manifest_dependencies(manifest: &str) -> Vec<(String, String)> {
    let mut deps = Vec::new();
    let lines: Vec<&str> = manifest.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        // Cargo.lock window: `name = "x"` followed by `version = "y"`.
        if let Some(value) = line.strip_prefix("name = ") {
            let name = value.trim().trim_matches('"');
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')) {
                if let Some(next) = lines.get(index + 1) {
                    if let Some(version) = next.trim().strip_prefix("version = ") {
                        let version = version.trim().trim_matches('"');
                        if valid_pair(name, version) {
                            deps.push((name.to_string(), version.to_string()));
                            continue;
                        }
                    }
                }
            }
        }
        // requirements.txt: name==1.2.3
        if let Some((name, version)) = line.split_once("==") {
            let name = name.trim();
            let version = version.split([';', '>', '<', ' ', ',']).next().unwrap_or("").trim();
            if valid_pair(name, version) {
                deps.push((name.to_string(), version.to_string()));
                continue;
            }
        }
        // JSON pair: "name": "1.2.3" (direct) or "name": { followed by a
        // "version": "1.2.3" line (package-lock nesting).
        if let Some((name, version)) = parse_json_pair(line) {
            if valid_pair(&name, &version) {
                deps.push((name, version));
                continue;
            }
        }
        if let Some(name) = line.strip_suffix("\": {") {
            let name = name.trim_start_matches('"').trim();
            if name.is_empty() { continue; }
            // Look ahead for the "version" key inside this object.
            for next in lines.iter().skip(index + 1).take(4) {
                let next = next.trim().trim_end_matches(',');
                if let Some((key, value)) = next.split_once("\": ") {
                    let key = key.trim_start_matches('"').trim();
                    let version = value.trim().trim_matches('"');
                    if key == "version" && valid_pair(name, version) {
                        deps.push((name.to_string(), version.to_string()));
                        break;
                    }
                }
            }
        }
        // TOML dep: name = "1.2" (skip structural keys)
        if let Some((name, version)) = parse_toml_pair(line) {
            if valid_pair(&name, &version) {
                deps.push((name, version));
                continue;
            }
        }
        // yarn/pnpm: 'name@1.2.3' or 'name@npm:1.2.3'
        if line.starts_with('\'') || line.starts_with('"') {
            let inner = line.trim_matches(['\'', '"']);
            if let Some((name, version)) = inner.rsplit_once('@') {
                let version = version.strip_prefix("npm:").unwrap_or(version);
                let name = name.rsplit_once('@').map(|(n, _)| n).unwrap_or(name);
                if valid_pair(name, version) {
                    deps.push((name.to_string(), version.to_string()));
                    continue;
                }
            }
        }
        // go.mod: module v1.2.3
        if let Some((name, version)) = line.split_once(' ') {
            if version.starts_with('v')
                && name.contains('/')
                && valid_pair(name, version.trim_start_matches('v'))
            {
                deps.push((name.to_string(), version[1..].to_string()));
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn parse_json_pair(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_end_matches(',');
    let (key, value) = line.split_once("\": ")?;
    let name = key.trim_start_matches('"').trim();
    let version = value.trim().trim_matches('"');
    if name.is_empty()
        || version.is_empty()
        // Structural keys are not dependencies.
        || matches!(name, "version" | "name" | "id" | "resolved" | "integrity" | "type")
    {
        return None;
    }
    Some((name.to_string(), version.to_string()))
}

fn parse_toml_pair(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(" = ")?;
    let name = key.trim();
    if name.is_empty()
        || name.contains(' ')
        || name.contains('.')
        || name.contains('/')
        || name.contains(':')
        || matches!(name, "version" | "name" | "edition" | "license" | "path")
    {
        return None;
    }
    let version = value.trim().trim_matches('"');
    if version.is_empty() { return None; }
    Some((name.to_string(), version.to_string()))
}

fn valid_pair(name: &str, version: &str) -> bool {
    !name.is_empty()
        && !version.is_empty()
        && version.len() <= 32
        && version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '/' | '@'))
}

/// A bundled minimal advisory set of widely-known vulnerable dependency
/// versions (long-standing public advisories).  Real depth comes from the
/// external Osv/Trivy adapters when installed; this list keeps the
/// builtin path honest when they are not — no version without an entry
/// is ever flagged.
struct DependencyAdvisory {
    severity: SecuritySeverity,
    summary: &'static str,
    advisory_id: &'static str,
    remediation: &'static str,
    evidence_file: String,
}

fn known_vulnerable_dependency(name: &str, version: &str) -> Option<DependencyAdvisory> {
    let name = name.rsplit('/').next().unwrap_or(name).to_ascii_lowercase();
    let file = format!("manifest:{}@{}", name, version);
    let mk = |severity, summary: &'static str, advisory_id: &'static str, remediation: &'static str, file: String| {
        Some(DependencyAdvisory { severity, summary, advisory_id, remediation, evidence_file: file })
    };
    let major: Option<u32> = version.split('.').next()?.parse().ok();
    let minor: Option<u32> = version.split('.').nth(1).unwrap_or("0").parse().ok();
    let (major, minor) = (major?, minor.unwrap_or(0));
    if name == "lodash" && (major < 4 || (major == 4 && minor < 17) || (major == 4 && minor == 17 && version.split('.').nth(2).unwrap_or("0").parse::<u32>().unwrap_or(0) < 21)) {
        return mk(SecuritySeverity::High, "prototype pollution / template injection", "CVE-2015-8861 / CVE-2021-23337", "upgrade lodash to >=4.17.21", file);
    }
    if name == "request" && (major < 2 || (major == 2 && minor < 68)) {
        return mk(SecuritySeverity::Medium, "credential leak on cross-origin redirects", "GHSA-52mw-8j83-vb5w", "replace request with a maintained HTTP client", file);
    }
    if name == "node-sass" && major < 7 {
        return mk(SecuritySeverity::High, "libsass path traversal", "CVE-2020-24073", "replace node-sass with sass (dart-sass)", file);
    }
    if name == "moment" && (major < 2 || (major == 2 && minor < 29)) {
        return mk(SecuritySeverity::Low, "ReDoS in long date strings", "CVE-2022-31129", "upgrade moment to >=2.29.4", file);
    }
    if name == "validator" && (major < 13 || (major == 13 && minor < 7)) {
        return mk(SecuritySeverity::Medium, "inefficient regular expression DoS", "CVE-2021-3765", "upgrade validator to >=13.7.0", file);
    }
    if name == "elliptic" && (major < 6 || (major == 6 && minor < 5)) {
        return mk(SecuritySeverity::High, "invalid signature malleability", "GHSA-r9p9-mrjm-926w", "upgrade elliptic to >=6.5.4", file);
    }
    None
}

fn adapter_name(a: SecurityAdapter) -> &'static str { match a { SecurityAdapter::Gitleaks => "gitleaks", SecurityAdapter::Osv => "osv-scanner", SecurityAdapter::Trivy => "trivy", SecurityAdapter::Semgrep => "semgrep", SecurityAdapter::Checkov => "checkov", SecurityAdapter::Zap => "zap", SecurityAdapter::BuiltInSecretHeuristic => "builtin-secret-heuristic", SecurityAdapter::BuiltInSuspiciousSqlHeuristic => "builtin-suspicious-sql-heuristic", SecurityAdapter::BuiltInIacHeuristic => "builtin-iac-heuristic", SecurityAdapter::BuiltInActiveDastHeuristic => "builtin-active-dast-heuristic", SecurityAdapter::BuiltInCloudPostureHeuristic => "builtin-cloud-posture-heuristic", SecurityAdapter::BuiltInDependencyHeuristic => "builtin-dependency-heuristic", _ => "agentcode" } }
struct InstanceSpec<'a> { adapter: SecurityAdapter, rule_id: &'a str, severity: SecuritySeverity, proof_level: ProofLevel, file_path: &'a str, line: u32, fingerprint: &'a str, redacted_evidence: String }
fn instance(spec: InstanceSpec<'_>) -> SecurityFindingInstance { external_instance(spec.adapter, spec.rule_id, spec.severity, spec.proof_level, spec.file_path, spec.line, spec.fingerprint, spec.redacted_evidence, StableId::new("evidence")) }
#[allow(clippy::too_many_arguments)]
fn external_instance(adapter: SecurityAdapter, rule: &str, severity: SecuritySeverity, proof: ProofLevel, file: &str, line: u32, fingerprint: &str, evidence: String, raw: StableId) -> SecurityFindingInstance { SecurityFindingInstance { id: StableId::new("secinst"), adapter, rule_id: rule.to_string(), severity, confidence: if proof == ProofLevel::ExternalTool { 95 } else { 60 }, proof_level: proof, file_path: file.to_string(), line, fingerprint: fingerprint.to_string(), redacted_evidence: evidence, raw_evidence_ref: raw } }
fn empty_threat_model() -> ThreatModel { ThreatModel { id: StableId::new("threat"), entry_points: Vec::new(), auth_boundaries: Vec::new(), data_stores: Vec::new(), admin_operations: Vec::new(), cloud_configuration: Vec::new(), sensitive_assets: Vec::new(), evidence_refs: vec![StableId::new("evidence")] } }
fn scanner_version(adapter: SecurityAdapter, s: &str) -> String {
    if adapter == SecurityAdapter::Zap {
        return s
            .lines()
            .rev()
            .map(str::trim)
            .find(|line| line.chars().any(|c| c.is_ascii_digit()) && line.contains('.'))
            .map(redact_output)
            .unwrap_or_else(|| "unknown".to_string());
    }
    redact_output(s.lines().next().unwrap_or("unknown"))
}
fn redact_secret(value: &str) -> String { redact_output(value) }
fn redact_output(value: &str) -> String { value.split_whitespace().map(|part| if part.contains("AKIA") || part.contains("SECRET=") || part.contains("Authorization:") || part.contains("Cookie:") || part.contains("token=") || part.contains("password=") { "[REDACTED]" } else { part }).collect::<Vec<_>>().join(" ") }
fn remediation_for(f: &str) -> String { if f.contains("secret") { "remove committed secret and rotate credential".to_string() } else if f.contains("vulnerab") || f.starts_with("builtin-dependency") { "upgrade vulnerable dependency".to_string() } else { "review and remediate security finding".to_string() } }
fn json_escape(v: &str) -> String { v.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n") }
