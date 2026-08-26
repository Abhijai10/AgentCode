use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::{EvidenceKind, EvidenceStore, Provenance};
use ac_sandbox::{
    ExecRequest, IsolationLevel, ProcessRestrictedBackend, SandboxManager, SandboxPolicy,
};
use ac_security::{
    ActiveSecurityReport, AiSecurityReport, Capability, CapabilityPolicy, SecurityDecision,
};
use base64::Engine;
use serde_json::{json, Value};
use tempfile::TempDir;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VerificationLayer {
    Format,
    Lint,
    Typecheck,
    Compile,
    Build,
    Unit,
    Integration,
    Browser,
    Security,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationProfile {
    pub id: StableId,
    pub task_id: StableId,
    pub risk: VerificationRisk,
    pub required_layers: Vec<VerificationLayer>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCapabilities {
    pub cargo: bool,
    pub makefile: bool,
    pub package_json: bool,
    pub python: bool,
    pub go: bool,
    pub browser: bool,
    pub security: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub id: StableId,
    pub layer: VerificationLayer,
    pub argv: Vec<String>,
    pub detected_from: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateStatus {
    Passed,
    Failed,
    Unavailable,
    Skipped,
    Partial,
    BaselineFailure,
    Regression,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MechanicalGateResult {
    pub id: StableId,
    pub layer: VerificationLayer,
    pub status: GateStatus,
    pub command: Option<CommandSpec>,
    pub output_hash: String,
    pub duration_ms: u64,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestIdentity {
    pub id: StableId,
    pub name: String,
    pub path: String,
    pub layer: VerificationLayer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestRegistry {
    pub tests: Vec<TestIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestSelectionManifest {
    pub id: StableId,
    pub selected: Vec<TestIdentity>,
    pub excluded: Vec<TestIdentity>,
    pub confidence: u8,
    pub broadened: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationEvidenceManifest {
    pub id: StableId,
    pub commit: String,
    pub worktree: String,
    pub environment: String,
    pub tool_version: String,
    pub command: String,
    pub raw_artifact: String,
    pub normalized_result: GateStatus,
    pub requirement_refs: Vec<StableId>,
    pub freshness_dependencies: Vec<String>,
    pub evidence_ref: StableId,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementEvidenceLink {
    pub requirement_id: StableId,
    pub evidence_ref: StableId,
    pub evidence_kind: String,
    pub verified: bool,
    pub freshness_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierContext {
    pub id: StableId,
    pub requirement: String,
    pub diff_summary: String,
    pub test_summary: String,
    pub evidence_refs: Vec<StableId>,
    pub worker_narrative_included: bool,
    pub can_edit: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationFinding {
    pub code: String,
    pub severity: FindingSeverity,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingSeverity {
    Info,
    Warning,
    Blocking,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndependentVerifierReport {
    pub id: StableId,
    pub passed: bool,
    pub findings: Vec<VerificationFinding>,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestTamperReport {
    pub id: StableId,
    pub suspicious: bool,
    pub findings: Vec<VerificationFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshnessInvalidation {
    pub evidence_ref: StableId,
    pub stale: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrationVerificationReport {
    pub id: StableId,
    pub passed: bool,
    pub ran_broad_tests: bool,
    pub evidence_ref: StableId,
    pub findings: Vec<VerificationFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalAuditInput {
    pub original_goal: String,
    pub requirements: Vec<String>,
    pub verified_requirement_ids: Vec<StableId>,
    pub evidence_refs: Vec<StableId>,
    pub worker_completion_text: String,
    pub unresolved_limitations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalAuditReport {
    pub id: StableId,
    pub passed: bool,
    pub return_to_repair: bool,
    pub findings: Vec<VerificationFinding>,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionGateDecision {
    pub allowed: bool,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserAdapterMode {
    Playwright,
    ChromiumCdp,
    DeterministicHarness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserProcessState {
    Launching,
    Running,
    Ready,
    Crashed,
    LaunchFailed,
    Unavailable,
    Closing,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserProcessRecord {
    pub id: StableId,
    pub task_id: StableId,
    pub mode: BrowserAdapterMode,
    pub state: BrowserProcessState,
    pub profile_dir: String,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserSessionRecord {
    pub id: StableId,
    pub task_id: StableId,
    pub process_id: StableId,
    pub current_url: Option<String>,
    pub profile: String,
    pub storage_state_ref: Option<String>,
    pub sensitive: bool,
    pub stale_evidence_refs: Vec<StableId>,
    pub created_at: TimestampMillis,
    pub updated_at: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewportProfile {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowserAction {
    Navigate { url: String },
    OpenHtmlForTest { url: String, html: String },
    Click { selector: String },
    Type { selector: String, text: String },
    Select { selector: String, value: String },
    Scroll { y: i32 },
    Wait { millis: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserActionResult {
    pub session_id: StableId,
    pub action: String,
    pub ok: bool,
    pub url: String,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomSnapshot {
    pub session_id: StableId,
    pub visible_text: String,
    pub controls: Vec<String>,
    pub accessibility_tree: Vec<String>,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserDiagnostics {
    pub session_id: StableId,
    pub console_errors: Vec<String>,
    pub page_errors: Vec<String>,
    pub network_failures: Vec<String>,
    pub http_status: u16,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScreenshotEvidence {
    pub id: StableId,
    pub session_id: StableId,
    pub task_id: StableId,
    pub commit: String,
    pub viewport: ViewportProfile,
    pub url: String,
    pub artifact_uri: String,
    pub sensitive: bool,
    pub evidence_ref: StableId,
    pub captured_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevServerRecord {
    pub id: StableId,
    pub task_id: StableId,
    pub command: Vec<String>,
    pub port: u16,
    pub ready_url: String,
    pub process_alive: bool,
    pub http_ready: bool,
    pub route_loadable: bool,
    pub retained: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisualQaReport {
    pub id: StableId,
    pub screenshot_ref: StableId,
    pub passed: bool,
    pub findings: Vec<VerificationFinding>,
    pub adapter: String,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignVisualEvaluation {
    pub id: StableId,
    pub artifact_version_id: StableId,
    pub passed: bool,
    pub findings: Vec<String>,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignResponsiveReport {
    pub id: StableId,
    pub artifact_version_id: StableId,
    pub viewports: Vec<String>,
    pub passed: bool,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignAccessibilityReport {
    pub id: StableId,
    pub artifact_version_id: StableId,
    pub checks: Vec<String>,
    pub passed: bool,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignFunctionalReport {
    pub id: StableId,
    pub artifact_version_id: StableId,
    pub flows: Vec<String>,
    pub passed: bool,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PageState {
    url: String,
    html: String,
    fields: BTreeMap<String, String>,
    clicked: Vec<String>,
    scroll_y: i32,
    viewport: ViewportProfile,
}

pub struct BrowserRuntime {
    policy: CapabilityPolicy,
    mode: BrowserAdapterMode,
    processes: BTreeMap<StableId, BrowserProcessRecord>,
    sessions: BTreeMap<StableId, BrowserSessionRecord>,
    pages: BTreeMap<StableId, PageState>,
    real_processes: BTreeMap<StableId, RealBrowserProcess>,
    real_pages: BTreeMap<StableId, RealBrowserPage>,
}

struct RealBrowserProcess {
    child: Child,
    _profile_dir: TempDir,
    port: u16,
}

struct RealBrowserPage {
    client: CdpClient,
    url: String,
    viewport: ViewportProfile,
}

struct CdpClient {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: u64,
    console_errors: Vec<String>,
    page_errors: Vec<String>,
    network_failures: Vec<String>,
    http_status: u16,
}

const MAX_BROWSER_EVENT_EVIDENCE: usize = 256;

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

    /// Converts a current, normalized security scan into verification evidence.
    /// A policy that requires scanners cannot pass using an unavailable result.
    pub fn record_managed_security_scan(
        &self,
        report: &ac_security::SecurityScanReport,
        require_all_configured_scanners: bool,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<SecurityScanReport> {
        if require_all_configured_scanners && !report.missing_adapters.is_empty() {
            return Err(AcError::new(
                "VERIFY-SECURITY_SCANNER_UNAVAILABLE",
                format!(
                    "required security scanner unavailable: {}",
                    report.missing_adapters.join(", ")
                ),
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            ));
        }
        self.record_security_scan(
            "managed-security",
            report
                .findings
                .iter()
                .filter(|finding| finding.status == ac_security::FindingStatus::Confirmed)
                .count(),
            evidence_store,
        )
    }

    pub fn record_active_security_report(
        &self,
        report: &ActiveSecurityReport,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<SecurityScanReport> {
        if self.policy.evaluate(&[Capability::SecurityScan]) != SecurityDecision::Allow {
            return Err(AcError::policy_denied(
                "VERIFY-ACTIVE_SECURITY_DENIED",
                "active security evidence requires security scan capability approval",
            ));
        }
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: Some(report.commit.clone()),
                worktree: None,
                tool: Some("advanced-security".to_string()),
            },
            format!("mem://security/active/{}", report.id),
            format!(
                "findings:{};stops:{};cleanup:{}",
                report.findings.len(),
                report.stop_reasons.len(),
                report.cleanup.teardown_verified
            ),
        )?;
        Ok(SecurityScanReport {
            id: report.id.clone(),
            scanner: "advanced-security".to_string(),
            findings: report.findings.len(),
            evidence_ref,
        })
    }

    pub fn record_ai_security_report(
        &self,
        report: &AiSecurityReport,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<SecurityScanReport> {
        if self.policy.evaluate(&[Capability::SecurityScan]) != SecurityDecision::Allow {
            return Err(AcError::policy_denied(
                "VERIFY-AI_SECURITY_DENIED",
                "AI security evidence requires security scan capability approval",
            ));
        }
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: Some(report.commit.clone()),
                worktree: None,
                tool: Some("ai-security".to_string()),
            },
            format!("mem://security/ai/{}", report.id),
            format!(
                "surfaces:{};cases:{};findings:{}",
                report.surfaces.len(),
                report.attack_cases.len(),
                report.findings.len()
            ),
        )?;
        Ok(SecurityScanReport {
            id: report.id.clone(),
            scanner: "ai-security".to_string(),
            findings: report.findings.len(),
            evidence_ref,
        })
    }

    pub fn record_design_visual_evaluation(
        &self,
        artifact_version_id: &StableId,
        passed: bool,
        findings: Vec<String>,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DesignVisualEvaluation> {
        if findings.iter().any(|finding| finding.trim().is_empty()) {
            return Err(AcError::validation(
                "VERIFY-DESIGN_VISUAL_EMPTY_FINDING",
                "visual findings cannot be empty",
            ));
        }
        let artifact = artifact_version_id.clone();
        let summary = format!(
            "artifact:{artifact};passed:{passed};findings:{}",
            findings.len()
        );
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some("design-visual-critic".to_string()),
            },
            format!("mem://design/visual/{}", StableId::new("dvisual")),
            local_hash(&summary),
        )?;
        Ok(DesignVisualEvaluation {
            id: StableId::new("dvisual"),
            artifact_version_id: artifact,
            passed,
            findings,
            evidence_ref,
        })
    }

    pub fn record_design_responsive_report(
        &self,
        artifact_version_id: &StableId,
        viewports: Vec<String>,
        passed: bool,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DesignResponsiveReport> {
        if viewports.is_empty() {
            return Err(AcError::validation(
                "VERIFY-DESIGN_RESPONSIVE_NO_VIEWPORTS",
                "responsive QA requires at least one viewport",
            ));
        }
        let artifact = artifact_version_id.clone();
        let summary = format!(
            "artifact:{artifact};passed:{passed};viewports:{}",
            viewports.join(",")
        );
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some("design-responsive-qa".to_string()),
            },
            format!("mem://design/responsive/{}", StableId::new("dresponsive")),
            local_hash(&summary),
        )?;
        Ok(DesignResponsiveReport {
            id: StableId::new("dresponsive"),
            artifact_version_id: artifact,
            viewports,
            passed,
            evidence_ref,
        })
    }

    pub fn record_design_accessibility_report(
        &self,
        artifact_version_id: &StableId,
        passed: bool,
        checks: Vec<String>,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DesignAccessibilityReport> {
        if checks.is_empty() {
            return Err(AcError::validation(
                "VERIFY-DESIGN_A11Y_NO_CHECKS",
                "accessibility QA requires concrete checks",
            ));
        }
        let artifact = artifact_version_id.clone();
        let summary = format!(
            "artifact:{artifact};passed:{passed};checks:{}",
            checks.join(",")
        );
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some("design-accessibility-qa".to_string()),
            },
            format!("mem://design/a11y/{}", StableId::new("da11y")),
            local_hash(&summary),
        )?;
        Ok(DesignAccessibilityReport {
            id: StableId::new("da11y"),
            artifact_version_id: artifact,
            checks,
            passed,
            evidence_ref,
        })
    }

    pub fn record_design_functional_report(
        &self,
        artifact_version_id: &StableId,
        passed: bool,
        flows: Vec<String>,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DesignFunctionalReport> {
        if flows.is_empty() {
            return Err(AcError::validation(
                "VERIFY-DESIGN_FUNCTIONAL_NO_FLOWS",
                "functional design QA requires at least one flow",
            ));
        }
        let artifact = artifact_version_id.clone();
        let summary = format!(
            "artifact:{artifact};passed:{passed};flows:{}",
            flows.join(",")
        );
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some("design-functional-qa".to_string()),
            },
            format!("mem://design/functional/{}", StableId::new("dflow")),
            local_hash(&summary),
        )?;
        Ok(DesignFunctionalReport {
            id: StableId::new("dflow"),
            artifact_version_id: artifact,
            flows,
            passed,
            evidence_ref,
        })
    }

    pub fn derive_profile(
        &self,
        task_id: StableId,
        risk: VerificationRisk,
        capabilities: &ProjectCapabilities,
    ) -> VerificationProfile {
        let mut layers = BTreeSet::new();
        layers.insert(VerificationLayer::Format);
        if capabilities.cargo || capabilities.package_json || capabilities.python || capabilities.go
        {
            layers.insert(VerificationLayer::Lint);
            layers.insert(VerificationLayer::Typecheck);
            layers.insert(VerificationLayer::Unit);
        }
        if matches!(risk, VerificationRisk::High | VerificationRisk::Critical) {
            layers.insert(VerificationLayer::Build);
            layers.insert(VerificationLayer::Integration);
        }
        if capabilities.browser {
            layers.insert(VerificationLayer::Browser);
        }
        if capabilities.security || matches!(risk, VerificationRisk::Critical) {
            layers.insert(VerificationLayer::Security);
        }
        VerificationProfile {
            id: StableId::new("verifyprofile"),
            task_id,
            risk,
            required_layers: layers.into_iter().collect(),
            created_at: TimestampMillis::now(),
        }
    }

    pub fn detect_commands(&self, root: &Path, profile: &VerificationProfile) -> Vec<CommandSpec> {
        let capabilities = detect_project_capabilities(root);
        profile
            .required_layers
            .iter()
            .filter_map(|layer| command_for_layer(root, &capabilities, *layer))
            .collect()
    }

    pub fn normalize_gate_result(
        &self,
        layer: VerificationLayer,
        command: Option<CommandSpec>,
        exit_code: Option<i32>,
        raw_output: &str,
        baseline_failed: bool,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<MechanicalGateResult> {
        let status = match (command.is_some(), exit_code, raw_output.contains("skipped")) {
            (false, _, _) => GateStatus::Unavailable,
            (true, Some(0), true) => GateStatus::Skipped,
            (true, Some(0), false) => GateStatus::Passed,
            (true, Some(_), _) if baseline_failed => GateStatus::BaselineFailure,
            (true, Some(_), _) => GateStatus::Regression,
            (true, None, _) => GateStatus::Partial,
        };
        let evidence_ref = evidence_store.append_tool_output(
            Provenance {
                source: "verification-engine".to_string(),
                commit: None,
                worktree: None,
                tool: Some(format!("{:?}", layer)),
            },
            format!("mem://verification/gate/{}", StableId::new("gate")),
            raw_output,
            &[],
        )?;
        Ok(MechanicalGateResult {
            id: StableId::new("gate"),
            layer,
            status,
            command,
            output_hash: local_hash(raw_output),
            duration_ms: 0,
            evidence_ref,
        })
    }

    pub fn discover_tests(&self, root: &Path) -> AcResult<TestRegistry> {
        let mut tests = Vec::new();
        collect_tests(root, root, &mut tests)?;
        Ok(TestRegistry { tests })
    }

    pub fn select_tests(
        &self,
        registry: &TestRegistry,
        changed_paths: &[String],
    ) -> TestSelectionManifest {
        let mut selected = Vec::new();
        let mut excluded = Vec::new();
        for test in &registry.tests {
            if changed_paths
                .iter()
                .any(|path| related_test(path, &test.path))
            {
                selected.push(test.clone());
            } else {
                excluded.push(test.clone());
            }
        }
        let confidence = if selected.is_empty() { 35 } else { 85 };
        let broadened = confidence < 60;
        if broadened {
            selected = registry.tests.clone();
            excluded.clear();
        }
        TestSelectionManifest {
            id: StableId::new("testselect"),
            selected,
            excluded,
            confidence,
            broadened,
            reasons: vec![if broadened {
                "low confidence selection broadened to registry".to_string()
            } else {
                "path proximity selected targeted tests".to_string()
            }],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_evidence_manifest(
        &self,
        commit: impl Into<String>,
        worktree: impl Into<String>,
        command: impl Into<String>,
        normalized_result: GateStatus,
        requirement_refs: Vec<StableId>,
        freshness_dependencies: Vec<String>,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<VerificationEvidenceManifest> {
        let commit = commit.into();
        let worktree = worktree.into();
        let command = command.into();
        if commit.trim().is_empty() || command.trim().is_empty() {
            return Err(AcError::validation(
                "VERIFY-EVIDENCE_MANIFEST_INVALID",
                "verification evidence requires commit and command identity",
            ));
        }
        let raw_artifact = format!("commit:{commit};worktree:{worktree};command:{command}");
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: Some(commit.clone()),
                worktree: Some(worktree.clone()),
                tool: Some(command.clone()),
            },
            format!("mem://verification/manifest/{}", StableId::new("vmanifest")),
            local_hash(&raw_artifact),
        )?;
        Ok(VerificationEvidenceManifest {
            id: StableId::new("vmanifest"),
            commit,
            worktree,
            environment: environment_fingerprint(),
            tool_version: "agentcode-verification-v1".to_string(),
            command,
            raw_artifact,
            normalized_result,
            requirement_refs,
            freshness_dependencies,
            evidence_ref,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn link_requirement_evidence(
        &self,
        requirement_id: StableId,
        manifest: &VerificationEvidenceManifest,
    ) -> RequirementEvidenceLink {
        RequirementEvidenceLink {
            requirement_id,
            evidence_ref: manifest.evidence_ref.clone(),
            evidence_kind: "verification".to_string(),
            verified: manifest.normalized_result == GateStatus::Passed,
            freshness_key: manifest.freshness_dependencies.join(","),
        }
    }

    pub fn build_verifier_context(
        &self,
        requirement: impl Into<String>,
        diff_summary: impl Into<String>,
        test_summary: impl Into<String>,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<VerifierContext> {
        let requirement = requirement.into();
        if requirement.trim().is_empty() {
            return Err(AcError::validation(
                "VERIFY-CONTEXT_REQUIREMENT",
                "verifier context requires the actual requirement",
            ));
        }
        Ok(VerifierContext {
            id: StableId::new("vctx"),
            requirement,
            diff_summary: diff_summary.into(),
            test_summary: test_summary.into(),
            evidence_refs,
            worker_narrative_included: false,
            can_edit: false,
        })
    }

    pub fn run_independent_verifier(
        &self,
        context: &VerifierContext,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<IndependentVerifierReport> {
        if context.can_edit {
            return Err(AcError::policy_denied(
                "VERIFY-VERIFIER_EDIT_DENIED",
                "independent verifier is read-only by default",
            ));
        }
        let mut findings = Vec::new();
        findings.extend(adversarial_findings(&context.diff_summary));
        findings.extend(adversarial_findings(&context.test_summary));
        let passed = findings
            .iter()
            .all(|finding| finding.severity != FindingSeverity::Blocking);
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "independent-verifier".to_string(),
                commit: None,
                worktree: None,
                tool: Some("verifier".to_string()),
            },
            format!("mem://verification/verifier/{}", StableId::new("verifier")),
            format!("passed:{passed};findings:{}", findings.len()),
        )?;
        Ok(IndependentVerifierReport {
            id: StableId::new("verifier"),
            passed,
            findings,
            evidence_ref,
        })
    }

    pub fn detect_test_tampering(&self, base: &str, changed: &str) -> TestTamperReport {
        let mut findings = Vec::new();
        if changed.contains("#[ignore]") || changed.contains(".skip(") {
            findings.push(blocking("VERIFY-TEST_SKIPPED", "test skip introduced"));
        }
        if count_assertions(changed) < count_assertions(base)
            || changed.contains("assert!(true)")
            || changed.contains("assert_eq!(1, 1)")
        {
            findings.push(blocking(
                "VERIFY-WEAK_ASSERTION",
                "test assertion appears weakened",
            ));
        }
        TestTamperReport {
            id: StableId::new("tamper"),
            suspicious: !findings.is_empty(),
            findings,
        }
    }

    pub fn invalidate_stale_evidence(
        &self,
        manifest: &VerificationEvidenceManifest,
        changed_paths: &[String],
    ) -> FreshnessInvalidation {
        let stale = manifest.freshness_dependencies.iter().any(|dep| {
            changed_paths
                .iter()
                .any(|path| path == dep || path.starts_with(dep))
        });
        FreshnessInvalidation {
            evidence_ref: manifest.evidence_ref.clone(),
            stale,
            reason: if stale {
                "source dependency changed".to_string()
            } else {
                "no freshness dependency changed".to_string()
            },
        }
    }

    pub fn verify_integration(
        &self,
        manifests: &[VerificationEvidenceManifest],
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<IntegrationVerificationReport> {
        let ran_broad_tests = manifests.iter().any(|manifest| {
            manifest.command.contains("test")
                || manifest.command.contains("build")
                || manifest.command.contains("check")
        });
        let mut findings = Vec::new();
        if !ran_broad_tests {
            findings.push(blocking(
                "VERIFY-INTEGRATION_MISSING",
                "integration verification requires broad test/build/check evidence",
            ));
        }
        if manifests
            .iter()
            .any(|manifest| manifest.normalized_result != GateStatus::Passed)
        {
            findings.push(blocking(
                "VERIFY-INTEGRATION_FAILED",
                "integration manifest contains non-passing result",
            ));
        }
        let passed = findings.is_empty();
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "verification-engine".to_string(),
                commit: manifests.first().map(|manifest| manifest.commit.clone()),
                worktree: manifests.first().map(|manifest| manifest.worktree.clone()),
                tool: Some("integration-verification".to_string()),
            },
            format!(
                "mem://verification/integration/{}",
                StableId::new("integration")
            ),
            format!("passed:{passed};manifests:{}", manifests.len()),
        )?;
        Ok(IntegrationVerificationReport {
            id: StableId::new("integration"),
            passed,
            ran_broad_tests,
            evidence_ref,
            findings,
        })
    }

    pub fn final_audit(
        &self,
        input: FinalAuditInput,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<FinalAuditReport> {
        if input.original_goal.trim().is_empty() || input.requirements.is_empty() {
            return Err(AcError::validation(
                "VERIFY-FINAL_AUDIT_INPUT",
                "final audit requires original goal and requirements",
            ));
        }
        let mut findings = Vec::new();
        if input.verified_requirement_ids.len() < input.requirements.len() {
            findings.push(blocking(
                "VERIFY-FINAL_AUDIT_MISSING_REQUIREMENT",
                "not every requirement has accepted evidence",
            ));
        }
        if input.evidence_refs.is_empty() {
            findings.push(blocking(
                "VERIFY-FINAL_AUDIT_NO_EVIDENCE",
                "final audit requires evidence refs",
            ));
        }
        if !input.unresolved_limitations.is_empty() {
            findings.push(VerificationFinding {
                code: "VERIFY-FINAL_AUDIT_LIMITATION".to_string(),
                severity: FindingSeverity::Warning,
                message: input.unresolved_limitations.join("; "),
            });
        }
        let passed = findings
            .iter()
            .all(|finding| finding.severity != FindingSeverity::Blocking);
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "final-audit".to_string(),
                commit: None,
                worktree: None,
                tool: Some("final-audit".to_string()),
            },
            format!("mem://verification/final-audit/{}", StableId::new("audit")),
            format!(
                "passed:{passed};worker_text_len:{};findings:{}",
                input.worker_completion_text.len(),
                findings.len()
            ),
        )?;
        Ok(FinalAuditReport {
            id: StableId::new("audit"),
            passed,
            return_to_repair: !passed,
            findings,
            evidence_ref,
        })
    }

    pub fn completion_gate(
        &self,
        audit: &FinalAuditReport,
        worker_completion_text: &str,
    ) -> CompletionGateDecision {
        if !audit.passed {
            return CompletionGateDecision {
                allowed: false,
                reason: "final audit did not pass".to_string(),
            };
        }
        if worker_completion_text.trim().is_empty() {
            return CompletionGateDecision {
                allowed: false,
                reason: "worker completion text is not evidence".to_string(),
            };
        }
        CompletionGateDecision {
            allowed: true,
            reason: "kernel may complete with final audit evidence".to_string(),
        }
    }
}

impl BrowserRuntime {
    pub fn new(policy: CapabilityPolicy) -> Self {
        Self {
            policy,
            mode: BrowserAdapterMode::ChromiumCdp,
            processes: BTreeMap::new(),
            sessions: BTreeMap::new(),
            pages: BTreeMap::new(),
            real_processes: BTreeMap::new(),
            real_pages: BTreeMap::new(),
        }
    }

    pub fn deterministic_harness_for_tests(policy: CapabilityPolicy) -> Self {
        Self {
            policy,
            mode: BrowserAdapterMode::DeterministicHarness,
            processes: BTreeMap::new(),
            sessions: BTreeMap::new(),
            pages: BTreeMap::new(),
            real_processes: BTreeMap::new(),
            real_pages: BTreeMap::new(),
        }
    }

    pub fn launch(&mut self, task_id: StableId) -> AcResult<BrowserProcessRecord> {
        self.ensure_browser_allowed()?;
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            return self.launch_chromium_cdp(task_id);
        }
        let process = BrowserProcessRecord {
            id: StableId::new("browserproc"),
            profile_dir: format!("isolated-profile/{}", task_id),
            task_id,
            mode: self.mode,
            state: BrowserProcessState::Running,
            created_at: TimestampMillis::now(),
        };
        self.processes.insert(process.id.clone(), process.clone());
        Ok(process)
    }

    pub fn create_session(
        &mut self,
        task_id: StableId,
        process_id: StableId,
    ) -> AcResult<BrowserSessionRecord> {
        self.ensure_browser_allowed()?;
        let process = self.processes.get(&process_id).ok_or_else(|| {
            AcError::validation(
                "BROWSER-PROCESS_UNKNOWN",
                "browser process is not registered",
            )
        })?;
        if process.task_id != task_id || process.state != BrowserProcessState::Running {
            return Err(AcError::conflict(
                "BROWSER-SESSION_OWNER",
                "browser session must be tied to a running task-owned process",
            ));
        }
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            let real_process = self.real_processes.get(&process_id).ok_or_else(|| {
                AcError::validation(
                    "BROWSER-PROCESS_UNKNOWN",
                    "real browser process is not registered",
                )
            })?;
            let page = RealBrowserPage::create(real_process.port)?;
            self.real_pages.insert(process_id.clone(), page);
        }
        let session = BrowserSessionRecord {
            id: StableId::new("browsersession"),
            task_id,
            process_id,
            current_url: None,
            profile: "isolated-task-profile".to_string(),
            storage_state_ref: Some("classified-storage-ref".to_string()),
            sensitive: true,
            stale_evidence_refs: Vec::new(),
            created_at: TimestampMillis::now(),
            updated_at: TimestampMillis::now(),
        };
        self.sessions.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    pub fn act(
        &mut self,
        session_id: &StableId,
        action: BrowserAction,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<BrowserActionResult> {
        self.ensure_browser_allowed()?;
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            return self.act_chromium_cdp(session_id, action, evidence_store);
        }
        if !self.sessions.contains_key(session_id) {
            return Err(AcError::validation(
                "BROWSER-SESSION_UNKNOWN",
                "browser session is not registered",
            ));
        }
        let page = self.pages.entry(session_id.clone()).or_insert(PageState {
            url: "about:blank".to_string(),
            html: String::new(),
            fields: BTreeMap::new(),
            clicked: Vec::new(),
            scroll_y: 0,
            viewport: default_viewports()[2],
        });
        let action_name = match &action {
            BrowserAction::Navigate { url } => {
                page.url = url.clone();
                "navigate"
            }
            BrowserAction::OpenHtmlForTest { url, html } => {
                page.url = url.clone();
                page.html = html.clone();
                "open-html-for-test"
            }
            BrowserAction::Click { selector } => {
                require_selector(&page.html, selector)?;
                page.clicked.push(selector.clone());
                if selector.contains("submit") || selector.contains("button") {
                    page.url = route_after_submit(&page.url);
                }
                "click"
            }
            BrowserAction::Type { selector, text } => {
                require_selector(&page.html, selector)?;
                page.fields.insert(selector.clone(), text.clone());
                "type"
            }
            BrowserAction::Select { selector, value } => {
                require_selector(&page.html, selector)?;
                page.fields.insert(selector.clone(), value.clone());
                "select"
            }
            BrowserAction::Scroll { y } => {
                page.scroll_y = *y;
                "scroll"
            }
            BrowserAction::Wait { .. } => "wait",
        };
        let url = page.url.clone();
        {
            let session = self.session_mut(session_id)?;
            session.current_url = Some(url.clone());
            session.updated_at = TimestampMillis::now();
        }
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: None,
                worktree: None,
                tool: Some("browser-action".to_string()),
            },
            format!("mem://browser/action/{}", StableId::new("baction")),
            format!("session:{};action:{};url:{}", session_id, action_name, url),
        )?;
        Ok(BrowserActionResult {
            session_id: session_id.clone(),
            action: action_name.to_string(),
            ok: true,
            url,
            evidence_ref,
        })
    }

    pub fn inspect_dom(
        &mut self,
        session_id: &StableId,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DomSnapshot> {
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            return self.inspect_dom_chromium_cdp(session_id, evidence_store);
        }
        let page = self.page(session_id)?;
        let visible_text = visible_text(&page.html);
        let controls = controls(&page.html);
        let accessibility_tree = controls
            .iter()
            .map(|control| format!("control:{control}"))
            .chain(
                visible_text
                    .split_whitespace()
                    .take(16)
                    .map(|word| format!("text:{word}")),
            )
            .collect::<Vec<_>>();
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: None,
                worktree: None,
                tool: Some("dom-inspect".to_string()),
            },
            format!("mem://browser/dom/{}", StableId::new("dom")),
            local_hash(&visible_text),
        )?;
        Ok(DomSnapshot {
            session_id: session_id.clone(),
            visible_text,
            controls,
            accessibility_tree,
            evidence_ref,
        })
    }

    pub fn diagnostics(
        &mut self,
        session_id: &StableId,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<BrowserDiagnostics> {
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            return self.diagnostics_chromium_cdp(session_id, evidence_store);
        }
        let page = self.page(session_id)?;
        let console_errors = contains_any(&page.html, &["console.error", "throw new Error"])
            .then(|| "console error detected".to_string())
            .into_iter()
            .collect::<Vec<_>>();
        let page_errors = contains_any(&page.html, &["<script>throw", "window.onerror"])
            .then(|| "page error detected".to_string())
            .into_iter()
            .collect::<Vec<_>>();
        let network_failures = contains_any(&page.html, &["http://fail", "404.js", "missing.png"])
            .then(|| "network failure detected".to_string())
            .into_iter()
            .collect::<Vec<_>>();
        let http_status = if page.url.contains("404") { 404 } else { 200 };
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: None,
                worktree: None,
                tool: Some("browser-diagnostics".to_string()),
            },
            format!("mem://browser/diagnostics/{}", StableId::new("bdiag")),
            format!(
                "console:{};page:{};network:{};status:{}",
                console_errors.len(),
                page_errors.len(),
                network_failures.len(),
                http_status
            ),
        )?;
        Ok(BrowserDiagnostics {
            session_id: session_id.clone(),
            console_errors,
            page_errors,
            network_failures,
            http_status,
            evidence_ref,
        })
    }

    pub fn capture_screenshot(
        &mut self,
        session_id: &StableId,
        task_id: StableId,
        commit: impl Into<String>,
        viewport: ViewportProfile,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ScreenshotEvidence> {
        let commit = commit.into();
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            return self.capture_screenshot_chromium_cdp(
                session_id,
                task_id,
                commit,
                viewport,
                evidence_store,
            );
        }
        let page = self.pages.get_mut(session_id).ok_or_else(|| {
            AcError::validation("BROWSER-PAGE_UNKNOWN", "browser page is not open")
        })?;
        page.viewport = viewport;
        let artifact = format!(
            "screenshot:{}:{}x{}:{}",
            page.url, viewport.width, viewport.height, page.html
        );
        let evidence_ref = evidence_store.append(
            EvidenceKind::BrowserScreenshot,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: Some(commit.clone()),
                worktree: None,
                tool: Some("screenshot".to_string()),
            },
            format!("mem://browser/screenshot/{}", StableId::new("shot")),
            local_hash(&artifact),
        )?;
        Ok(ScreenshotEvidence {
            id: StableId::new("shot"),
            session_id: session_id.clone(),
            task_id,
            commit,
            viewport,
            url: page.url.clone(),
            artifact_uri: artifact,
            sensitive: false,
            evidence_ref,
            captured_at: TimestampMillis::now(),
        })
    }

    pub fn manage_dev_server(
        &self,
        task_id: StableId,
        command: Vec<String>,
        port: u16,
        ready_url: impl Into<String>,
    ) -> AcResult<DevServerRecord> {
        if command.is_empty() || port == 0 {
            return Err(AcError::validation(
                "BROWSER-DEV_SERVER_INVALID",
                "dev server requires command and port",
            ));
        }
        let ready_url = ready_url.into();
        Ok(DevServerRecord {
            id: StableId::new("devserver"),
            task_id,
            command,
            port,
            process_alive: true,
            http_ready: ready_url.starts_with("http://127.0.0.1")
                || ready_url.starts_with("http://localhost"),
            route_loadable: !ready_url.ends_with("/404"),
            ready_url,
            retained: true,
        })
    }

    pub fn visual_qa(
        &self,
        screenshot: &ScreenshotEvidence,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<VisualQaReport> {
        let mut findings = Vec::new();
        if screenshot.artifact_uri.starts_with("screenshot:") {
            findings.push(blocking(
                "BROWSER-VISUAL_FAKE_ARTIFACT",
                "visual QA requires a real screenshot artifact",
            ));
        } else if fs::metadata(&screenshot.artifact_uri)
            .map(|metadata| metadata.len() == 0)
            .unwrap_or(true)
        {
            findings.push(blocking(
                "BROWSER-VISUAL_ARTIFACT_MISSING",
                "screenshot artifact is missing or empty",
            ));
        }
        let passed = findings.is_empty();
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: Some(screenshot.commit.clone()),
                worktree: None,
                tool: Some("visual-qa".to_string()),
            },
            format!("mem://browser/visual-qa/{}", StableId::new("visual")),
            format!("passed:{passed};screenshot:{}", screenshot.evidence_ref),
        )?;
        Ok(VisualQaReport {
            id: StableId::new("visual"),
            screenshot_ref: screenshot.evidence_ref.clone(),
            passed,
            findings,
            adapter: "browser-screenshot-file-check".to_string(),
            evidence_ref,
        })
    }

    pub fn mark_crashed(&mut self, process_id: &StableId) -> AcResult<()> {
        let process = self.processes.get_mut(process_id).ok_or_else(|| {
            AcError::validation(
                "BROWSER-PROCESS_UNKNOWN",
                "browser process is not registered",
            )
        })?;
        process.state = BrowserProcessState::Crashed;
        if let Some(mut real_process) = self.real_processes.remove(process_id) {
            let _ = real_process.child.kill();
            let _ = real_process.child.wait();
        }
        Ok(())
    }

    pub fn recover_crashed_session(
        &mut self,
        session_id: &StableId,
        stale_evidence: Vec<StableId>,
    ) -> AcResult<BrowserSessionRecord> {
        let old = self.sessions.get(session_id).cloned().ok_or_else(|| {
            AcError::validation(
                "BROWSER-SESSION_UNKNOWN",
                "browser session is not registered",
            )
        })?;
        let process = self.launch(old.task_id.clone())?;
        let mut recovered = self.create_session(old.task_id, process.id)?;
        recovered.current_url = old.current_url;
        recovered.stale_evidence_refs = stale_evidence;
        self.sessions
            .insert(recovered.id.clone(), recovered.clone());
        Ok(recovered)
    }

    pub fn default_viewports(&self) -> Vec<ViewportProfile> {
        default_viewports().to_vec()
    }

    fn page(&self, session_id: &StableId) -> AcResult<&PageState> {
        self.pages
            .get(session_id)
            .ok_or_else(|| AcError::validation("BROWSER-PAGE_UNKNOWN", "browser page is not open"))
    }

    fn session_mut(&mut self, session_id: &StableId) -> AcResult<&mut BrowserSessionRecord> {
        self.sessions.get_mut(session_id).ok_or_else(|| {
            AcError::validation(
                "BROWSER-SESSION_UNKNOWN",
                "browser session is not registered",
            )
        })
    }

    fn launch_chromium_cdp(&mut self, task_id: StableId) -> AcResult<BrowserProcessRecord> {
        let executable = discover_chromium_executable().ok_or_else(|| {
            AcError::new(
                "BROWSER-UNAVAILABLE",
                "no Chrome/Chromium executable is configured or available",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            )
        })?;
        let profile_dir = tempfile::Builder::new()
            .prefix("agentcode-browser-profile-")
            .tempdir()
            .map_err(|err| {
                AcError::new(
                    "BROWSER-PROFILE_CREATE",
                    err.to_string(),
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::NotRetryable,
                )
            })?;
        let browser_args = vec![
            "--headless=new".to_string(),
            "--remote-debugging-port=0".to_string(),
            format!("--user-data-dir={}", profile_dir.path().display()),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            "--disable-background-networking".to_string(),
            "--disable-sync".to_string(),
            "--disable-extensions".to_string(),
            "--disable-popup-blocking".to_string(),
            "about:blank".to_string(),
        ];
        let plan = prepare_browser_spawn(&executable, &browser_args, profile_dir.path())?;
        let mut child = Command::new(plan.backend_argv.first().ok_or_else(|| {
            AcError::validation("BROWSER-SANDBOX_PLAN", "sandbox plan has no executable")
        })?)
        .args(plan.backend_argv.iter().skip(1))
        .current_dir(plan.cwd)
        .env_clear()
        .envs(plan.allowed_env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            AcError::new(
                "BROWSER-LAUNCH_FAILED",
                err.to_string(),
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            )
        })?;
        let port = match wait_for_devtools_port(profile_dir.path(), &mut child) {
            Ok(port) => port,
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(err);
            }
        };
        let process = BrowserProcessRecord {
            id: StableId::new("browserproc"),
            profile_dir: profile_dir.path().display().to_string(),
            task_id,
            mode: BrowserAdapterMode::ChromiumCdp,
            state: BrowserProcessState::Running,
            created_at: TimestampMillis::now(),
        };
        self.real_processes.insert(
            process.id.clone(),
            RealBrowserProcess {
                child,
                _profile_dir: profile_dir,
                port,
            },
        );
        self.processes.insert(process.id.clone(), process.clone());
        Ok(process)
    }

    fn real_page_mut(&mut self, session_id: &StableId) -> AcResult<&mut RealBrowserPage> {
        let process_id = self
            .sessions
            .get(session_id)
            .ok_or_else(|| {
                AcError::validation(
                    "BROWSER-SESSION_UNKNOWN",
                    "browser session is not registered",
                )
            })?
            .process_id
            .clone();
        self.real_pages.get_mut(&process_id).ok_or_else(|| {
            AcError::validation("BROWSER-PAGE_UNKNOWN", "real browser page is not open")
        })
    }

    fn act_chromium_cdp(
        &mut self,
        session_id: &StableId,
        action: BrowserAction,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<BrowserActionResult> {
        let action_name = {
            let page = self.real_page_mut(session_id)?;
            match &action {
                BrowserAction::Navigate { url } => {
                    page.navigate(url)?;
                    "navigate"
                }
                BrowserAction::OpenHtmlForTest { .. } => {
                    return Err(AcError::policy_denied(
                        "BROWSER-HTML_HARNESS_DISABLED",
                        "caller-provided HTML is not accepted by the production browser backend",
                    ));
                }
                BrowserAction::Click { selector } => {
                    page.click(selector)?;
                    "click"
                }
                BrowserAction::Type { selector, text } => {
                    page.type_text(selector, text)?;
                    "type"
                }
                BrowserAction::Select { selector, value } => {
                    page.select(selector, value)?;
                    "select"
                }
                BrowserAction::Scroll { y } => {
                    page.scroll(*y)?;
                    "scroll"
                }
                BrowserAction::Wait { millis } => {
                    page.wait(*millis)?;
                    "wait"
                }
            }
        };
        let url = self.real_page_mut(session_id)?.url.clone();
        {
            let session = self.session_mut(session_id)?;
            session.current_url = Some(url.clone());
            session.updated_at = TimestampMillis::now();
        }
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: None,
                worktree: None,
                tool: Some("browser-action-cdp".to_string()),
            },
            format!("mem://browser/action/{}", StableId::new("baction")),
            format!("session:{};action:{};url:{}", session_id, action_name, url),
        )?;
        Ok(BrowserActionResult {
            session_id: session_id.clone(),
            action: action_name.to_string(),
            ok: true,
            url,
            evidence_ref,
        })
    }

    fn inspect_dom_chromium_cdp(
        &mut self,
        session_id: &StableId,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DomSnapshot> {
        let page = self.real_page_mut(session_id)?;
        let visible_text = page.visible_text()?;
        let controls = page.controls()?;
        let accessibility_tree = page.accessibility_tree()?;
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: None,
                worktree: None,
                tool: Some("dom-inspect-cdp".to_string()),
            },
            format!("mem://browser/dom/{}", StableId::new("dom")),
            local_hash(&visible_text),
        )?;
        Ok(DomSnapshot {
            session_id: session_id.clone(),
            visible_text,
            controls,
            accessibility_tree,
            evidence_ref,
        })
    }

    fn diagnostics_chromium_cdp(
        &mut self,
        session_id: &StableId,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<BrowserDiagnostics> {
        let page = self.real_page_mut(session_id)?;
        page.client.drain_events(Duration::from_millis(150))?;
        let console_errors = page.client.console_errors.clone();
        let page_errors = page.client.page_errors.clone();
        let network_failures = page.client.network_failures.clone();
        let http_status = page.client.http_status;
        let evidence_ref = evidence_store.append(
            EvidenceKind::TestReport,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: None,
                worktree: None,
                tool: Some("browser-diagnostics-cdp".to_string()),
            },
            format!("mem://browser/diagnostics/{}", StableId::new("bdiag")),
            format!(
                "console:{};page:{};network:{};status:{}",
                console_errors.len(),
                page_errors.len(),
                network_failures.len(),
                http_status
            ),
        )?;
        Ok(BrowserDiagnostics {
            session_id: session_id.clone(),
            console_errors,
            page_errors,
            network_failures,
            http_status,
            evidence_ref,
        })
    }

    fn capture_screenshot_chromium_cdp(
        &mut self,
        session_id: &StableId,
        task_id: StableId,
        commit: String,
        viewport: ViewportProfile,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ScreenshotEvidence> {
        let page = self.real_page_mut(session_id)?;
        let artifact_uri = page.capture_screenshot(viewport)?;
        let url = page.url.clone();
        let screenshot_bytes = fs::read(&artifact_uri)
            .map_err(|err| AcError::validation("BROWSER-SCREENSHOT_READ", err.to_string()))?;
        let evidence_ref = evidence_store.append(
            EvidenceKind::BrowserScreenshot,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: Some(commit.clone()),
                worktree: None,
                tool: Some("screenshot-cdp".to_string()),
            },
            artifact_uri.clone(),
            local_hash(&base64::engine::general_purpose::STANDARD.encode(&screenshot_bytes)),
        )?;
        Ok(ScreenshotEvidence {
            id: StableId::new("shot"),
            session_id: session_id.clone(),
            task_id,
            commit,
            viewport,
            url,
            artifact_uri,
            sensitive: false,
            evidence_ref,
            captured_at: TimestampMillis::now(),
        })
    }

    fn ensure_browser_allowed(&self) -> AcResult<()> {
        if self.policy.evaluate(&[Capability::BrowserAutomation]) != SecurityDecision::Allow {
            return Err(AcError::policy_denied(
                "BROWSER-CAPABILITY_DENIED",
                "browser automation requires capability approval",
            ));
        }
        Ok(())
    }
}

impl Drop for BrowserRuntime {
    fn drop(&mut self) {
        for process in self.processes.values_mut() {
            if matches!(
                process.state,
                BrowserProcessState::Running | BrowserProcessState::Ready
            ) {
                process.state = BrowserProcessState::Closing;
            }
        }
        for (_, mut process) in std::mem::take(&mut self.real_processes) {
            let _ = process.child.kill();
            let _ = process.child.wait();
        }
        for process in self.processes.values_mut() {
            if process.state == BrowserProcessState::Closing {
                process.state = BrowserProcessState::Closed;
            }
        }
    }
}

impl RealBrowserPage {
    fn create(port: u16) -> AcResult<Self> {
        let target = http_request_json("PUT", port, "/json/new?about:blank")
            .or_else(|_| http_request_json("GET", port, "/json/new?about:blank"))?;
        let ws_url = target
            .get("webSocketDebuggerUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AcError::validation(
                    "BROWSER-CDP_TARGET",
                    "Chrome did not return a page websocket URL",
                )
            })?;
        let mut client = CdpClient::connect(ws_url)?;
        client.call("Page.enable", json!({}))?;
        client.call("Runtime.enable", json!({}))?;
        client.call("DOM.enable", json!({}))?;
        client.call("Network.enable", json!({}))?;
        client.call("Accessibility.enable", json!({}))?;
        Ok(Self {
            client,
            url: "about:blank".to_string(),
            viewport: default_viewports()[2],
        })
    }

    fn navigate(&mut self, url: &str) -> AcResult<()> {
        self.client
            .call("Page.navigate", json!({ "url": url.to_string() }))?;
        self.client.wait_for_load(Some(url))?;
        self.url = self.current_url()?;
        Ok(())
    }

    fn click(&mut self, selector: &str) -> AcResult<()> {
        let selector = serde_json::to_string(selector)
            .map_err(|err| AcError::validation("BROWSER-SELECTOR_ENCODE", err.to_string()))?;
        let clicked = self.client.evaluate_bool(&format!(
            r#"(() => {{
                const el = document.querySelector({selector});
                if (!el) return false;
                el.scrollIntoView({{ block: "center", inline: "center" }});
                el.click();
                return true;
            }})()"#
        ))?;
        if !clicked {
            return Err(AcError::validation(
                "BROWSER-SELECTOR_NOT_FOUND",
                "selector not found in real browser DOM",
            ));
        }
        self.client.drain_events(Duration::from_millis(250))?;
        self.url = self.current_url()?;
        Ok(())
    }

    fn type_text(&mut self, selector: &str, text: &str) -> AcResult<()> {
        let selector = serde_json::to_string(selector)
            .map_err(|err| AcError::validation("BROWSER-SELECTOR_ENCODE", err.to_string()))?;
        let text = serde_json::to_string(text)
            .map_err(|err| AcError::validation("BROWSER-TEXT_ENCODE", err.to_string()))?;
        let typed = self.client.evaluate_bool(&format!(
            r#"(() => {{
                const el = document.querySelector({selector});
                if (!el) return false;
                el.focus();
                el.value = {text};
                el.dispatchEvent(new Event("input", {{ bubbles: true }}));
                el.dispatchEvent(new Event("change", {{ bubbles: true }}));
                return true;
            }})()"#
        ))?;
        if !typed {
            return Err(AcError::validation(
                "BROWSER-SELECTOR_NOT_FOUND",
                "selector not found in real browser DOM",
            ));
        }
        Ok(())
    }

    fn select(&mut self, selector: &str, value: &str) -> AcResult<()> {
        let selector = serde_json::to_string(selector)
            .map_err(|err| AcError::validation("BROWSER-SELECTOR_ENCODE", err.to_string()))?;
        let value = serde_json::to_string(value)
            .map_err(|err| AcError::validation("BROWSER-VALUE_ENCODE", err.to_string()))?;
        let selected = self.client.evaluate_bool(&format!(
            r#"(() => {{
                const el = document.querySelector({selector});
                if (!el) return false;
                el.value = {value};
                el.dispatchEvent(new Event("input", {{ bubbles: true }}));
                el.dispatchEvent(new Event("change", {{ bubbles: true }}));
                return true;
            }})()"#
        ))?;
        if !selected {
            return Err(AcError::validation(
                "BROWSER-SELECTOR_NOT_FOUND",
                "selector not found in real browser DOM",
            ));
        }
        Ok(())
    }

    fn scroll(&mut self, y: i32) -> AcResult<()> {
        self.client.evaluate_bool(&format!(
            "(() => {{ window.scrollTo(0, {y}); return true; }})()"
        ))?;
        Ok(())
    }

    fn wait(&mut self, millis: u64) -> AcResult<()> {
        let bounded = millis.min(10_000);
        self.client.drain_events(Duration::from_millis(bounded))?;
        self.url = self.current_url()?;
        Ok(())
    }

    fn visible_text(&mut self) -> AcResult<String> {
        self.client.evaluate_string(
            r#"(() => document.body ? document.body.innerText.replace(/\s+/g, " ").trim() : "")()"#,
        )
    }

    fn controls(&mut self) -> AcResult<Vec<String>> {
        self.client.evaluate_string_vec(
            r#"(() => Array.from(document.querySelectorAll("button,input,select,textarea,a"))
                .slice(0, 80)
                .map((el) => {
                    const role = el.getAttribute("role") || el.tagName.toLowerCase();
                    const name = el.getAttribute("aria-label") || el.name || el.id || el.innerText || el.value || "";
                    return `${role}:${String(name).replace(/\s+/g, " ").trim()}`;
                }))()"#,
        )
    }

    fn accessibility_tree(&mut self) -> AcResult<Vec<String>> {
        let value = self
            .client
            .call("Accessibility.getFullAXTree", json!({ "depth": 4 }))?;
        let nodes = value
            .get("nodes")
            .and_then(Value::as_array)
            .map(|nodes| {
                nodes
                    .iter()
                    .filter_map(|node| {
                        let role = node
                            .get("role")
                            .and_then(|role| role.get("value"))
                            .and_then(Value::as_str)?;
                        let name = node
                            .get("name")
                            .and_then(|name| name.get("value"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        Some(format!("{role}:{}", name.trim()))
                    })
                    .take(80)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(nodes)
    }

    fn capture_screenshot(&mut self, viewport: ViewportProfile) -> AcResult<String> {
        self.viewport = viewport;
        self.client.call(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "width": viewport.width,
                "height": viewport.height,
                "deviceScaleFactor": 1,
                "mobile": viewport.width < 600
            }),
        )?;
        self.client.drain_events(Duration::from_millis(100))?;
        let value = self.client.call(
            "Page.captureScreenshot",
            json!({ "format": "png", "fromSurface": true }),
        )?;
        let data = value.get("data").and_then(Value::as_str).ok_or_else(|| {
            AcError::validation(
                "BROWSER-SCREENSHOT_EMPTY",
                "Chrome returned no screenshot data",
            )
        })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|err| AcError::validation("BROWSER-SCREENSHOT_DECODE", err.to_string()))?;
        let dir = std::env::temp_dir().join("agentcode-browser-artifacts");
        fs::create_dir_all(&dir)
            .map_err(|err| AcError::validation("BROWSER-SCREENSHOT_DIR", err.to_string()))?;
        let path = dir.join(format!("{}.png", StableId::new("browser-shot")));
        fs::write(&path, bytes)
            .map_err(|err| AcError::validation("BROWSER-SCREENSHOT_WRITE", err.to_string()))?;
        Ok(path.display().to_string())
    }

    fn current_url(&mut self) -> AcResult<String> {
        self.client.evaluate_string("(() => location.href)()")
    }
}

impl CdpClient {
    fn connect(ws_url: &str) -> AcResult<Self> {
        let (mut socket, _) = connect(ws_url).map_err(|err| {
            AcError::new(
                "BROWSER-CDP_CONNECT",
                err.to_string(),
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            )
        })?;
        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .map_err(|err| AcError::validation("BROWSER-CDP_TIMEOUT", err.to_string()))?;
        }
        Ok(Self {
            socket,
            next_id: 1,
            console_errors: Vec::new(),
            page_errors: Vec::new(),
            network_failures: Vec::new(),
            http_status: 0,
        })
    }

    fn call(&mut self, method: &str, params: Value) -> AcResult<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let payload = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .map_err(|err| AcError::validation("BROWSER-CDP_SEND", err.to_string()))?;
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            match self.socket.read() {
                Ok(message) => {
                    if let Some(response) = self.observe_message(message, id)? {
                        if let Some(error) = response.get("error") {
                            return Err(AcError::validation(
                                "BROWSER-CDP_ERROR",
                                error.to_string(),
                            ));
                        }
                        return Ok(response.get("result").cloned().unwrap_or(Value::Null));
                    }
                }
                Err(tungstenite::Error::Io(err))
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(err) => {
                    return Err(AcError::validation("BROWSER-CDP_READ", err.to_string()));
                }
            }
        }
        Err(AcError::new(
            "BROWSER-CDP_TIMEOUT",
            format!("timed out waiting for {method}"),
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::NotRetryable,
        ))
    }

    fn drain_events(&mut self, duration: Duration) -> AcResult<()> {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            match self.socket.read() {
                Ok(message) => {
                    self.observe_message(message, u64::MAX)?;
                }
                Err(tungstenite::Error::Io(err))
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(tungstenite::Error::ConnectionClosed) => break,
                Err(err) => {
                    return Err(AcError::validation("BROWSER-CDP_READ", err.to_string()));
                }
            }
        }
        Ok(())
    }

    fn wait_for_load(&mut self, expected_url: Option<&str>) -> AcResult<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            self.drain_events(Duration::from_millis(100))?;
            let ready = self.evaluate_string("(() => document.readyState)()")?;
            let url_matches = if let Some(expected_url) = expected_url {
                self.evaluate_string("(() => location.href)()")?
                    .starts_with(expected_url)
            } else {
                true
            };
            if (ready == "complete" || ready == "interactive") && url_matches {
                return Ok(());
            }
        }
        Err(AcError::new(
            "BROWSER-NAVIGATION_TIMEOUT",
            "navigation did not reach an interactive document state",
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::NotRetryable,
        ))
    }

    fn evaluate_string(&mut self, expression: &str) -> AcResult<String> {
        let value = self.evaluate_value(expression)?;
        Ok(value.as_str().unwrap_or_default().to_string())
    }

    fn evaluate_bool(&mut self, expression: &str) -> AcResult<bool> {
        let value = self.evaluate_value(expression)?;
        Ok(value.as_bool().unwrap_or(false))
    }

    fn evaluate_string_vec(&mut self, expression: &str) -> AcResult<Vec<String>> {
        let value = self.evaluate_value(expression)?;
        Ok(value
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default())
    }

    fn evaluate_value(&mut self, expression: &str) -> AcResult<Value> {
        let result = self.call(
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true
            }),
        )?;
        Ok(result
            .get("result")
            .and_then(|result| result.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    fn observe_message(&mut self, message: Message, expected_id: u64) -> AcResult<Option<Value>> {
        let text = match message {
            Message::Text(text) => text.to_string(),
            Message::Binary(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Message::Ping(payload) => {
                let _ = self.socket.send(Message::Pong(payload));
                return Ok(None);
            }
            Message::Close(_) => {
                return Err(AcError::new(
                    "BROWSER-CDP_DISCONNECTED",
                    "Chrome DevTools websocket closed",
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::NotRetryable,
                ));
            }
            _ => return Ok(None),
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|err| AcError::validation("BROWSER-CDP_JSON", err.to_string()))?;
        if value.get("id").and_then(Value::as_u64) == Some(expected_id) {
            return Ok(Some(value));
        }
        if let Some(method) = value.get("method").and_then(Value::as_str) {
            self.observe_event(method, value.get("params").unwrap_or(&Value::Null));
        }
        Ok(None)
    }

    fn observe_event(&mut self, method: &str, params: &Value) {
        match method {
            "Runtime.consoleAPICalled" => {
                if matches!(params.get("type").and_then(Value::as_str), Some("error")) {
                    let text = params
                        .get("args")
                        .and_then(Value::as_array)
                        .map(|args| {
                            args.iter()
                                .filter_map(|arg| {
                                    arg.get("value")
                                        .and_then(Value::as_str)
                                        .or_else(|| arg.get("description").and_then(Value::as_str))
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .unwrap_or_else(|| "console error".to_string());
                    push_bounded(&mut self.console_errors, redact_url(&text));
                }
            }
            "Runtime.exceptionThrown" => {
                let text = params
                    .get("exceptionDetails")
                    .and_then(|details| details.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or("page exception");
                push_bounded(&mut self.page_errors, redact_url(text));
            }
            "Network.responseReceived" => {
                if let Some(status) = params
                    .get("response")
                    .and_then(|response| response.get("status"))
                    .and_then(Value::as_u64)
                {
                    self.http_status = status as u16;
                    if status >= 400 {
                        let url = params
                            .get("response")
                            .and_then(|response| response.get("url"))
                            .and_then(Value::as_str)
                            .unwrap_or("network response");
                        push_bounded(
                            &mut self.network_failures,
                            format!("http {status}: {}", redact_url(url)),
                        );
                    }
                }
            }
            "Network.loadingFailed" => {
                let text = params
                    .get("errorText")
                    .and_then(Value::as_str)
                    .unwrap_or("network loading failed");
                push_bounded(&mut self.network_failures, redact_url(text));
            }
            _ => {}
        }
    }
}

fn push_bounded(items: &mut Vec<String>, value: String) {
    if items.len() >= MAX_BROWSER_EVENT_EVIDENCE {
        items.remove(0);
    }
    items.push(value);
}

fn discover_chromium_executable() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("AGENTCODE_CHROME_EXECUTABLE") {
        let path = PathBuf::from(value);
        return path.is_file().then_some(path);
    }
    [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|path| path.is_file())
}

fn prepare_browser_spawn(
    executable: &Path,
    args: &[String],
    profile_dir: &Path,
) -> AcResult<ac_sandbox::SandboxedExecutionPlan> {
    let executable = executable.display().to_string();
    let mut argv = vec![executable.clone()];
    argv.extend(args.iter().cloned());
    let mut capability_policy = CapabilityPolicy::new()
        .allow(Capability::ProcessExec(executable))
        .allow(Capability::Network("*".to_string()));
    capability_policy = capability_policy.allow(Capability::BrowserAutomation);
    let policy = SandboxPolicy {
        workspace_roots: vec![profile_dir.to_path_buf()],
        capability_policy,
        network_default_allow: true,
        max_timeout_ms: 60_000,
        required_isolation: IsolationLevel::ProcessRestricted,
        max_output_bytes: 1024 * 1024,
    };
    SandboxManager::with_backend(policy, Box::new(ProcessRestrictedBackend)).prepare_execution(
        ExecRequest {
            argv,
            cwd: profile_dir.to_path_buf(),
            env: BTreeMap::new(),
            network: true,
            timeout_ms: 60_000,
        },
    )
}

fn wait_for_devtools_port(profile_dir: &Path, child: &mut Child) -> AcResult<u16> {
    let port_file = profile_dir.join("DevToolsActivePort");
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(|err| {
            AcError::new(
                "BROWSER-LAUNCH_STATUS",
                err.to_string(),
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            )
        })? {
            return Err(AcError::new(
                "BROWSER-LAUNCH_FAILED",
                format!("Chrome exited before DevTools became ready: {status}"),
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            ));
        }
        if let Ok(contents) = fs::read_to_string(&port_file) {
            if let Some(port) = contents.lines().next().and_then(|line| line.parse().ok()) {
                return Ok(port);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(AcError::new(
        "BROWSER-LAUNCH_TIMEOUT",
        "Chrome did not publish DevToolsActivePort",
        ac_common::ErrorKind::Unavailable,
        ac_common::Retryability::NotRetryable,
    ))
}

fn http_request_json(method: &str, port: u16, path: &str) -> AcResult<Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|err| {
        AcError::new(
            "BROWSER-CDP_HTTP_CONNECT",
            err.to_string(),
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::NotRetryable,
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .map_err(|err| AcError::validation("BROWSER-CDP_HTTP_TIMEOUT", err.to_string()))?;
    let request =
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|err| AcError::validation("BROWSER-CDP_HTTP_WRITE", err.to_string()))?;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                bytes.extend_from_slice(&buffer[..n]);
                if bytes.windows(5).any(|window| window == b"\r\n0\r\n") {
                    break;
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) && !bytes.is_empty() =>
            {
                break;
            }
            Err(err) => {
                return Err(AcError::validation(
                    "BROWSER-CDP_HTTP_READ",
                    err.to_string(),
                ));
            }
        }
    }
    let response = String::from_utf8_lossy(&bytes);
    let (headers, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
        AcError::validation(
            "BROWSER-CDP_HTTP_RESPONSE",
            "invalid HTTP response from Chrome",
        )
    })?;
    if !headers.starts_with("HTTP/1.1 200") {
        return Err(AcError::validation(
            "BROWSER-CDP_HTTP_STATUS",
            headers.lines().next().unwrap_or("HTTP error").to_string(),
        ));
    }
    let body = if headers.lines().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    }) {
        decode_chunked_body(body)?
    } else {
        body.to_string()
    };
    serde_json::from_str(&body)
        .map_err(|err| AcError::validation("BROWSER-CDP_HTTP_JSON", err.to_string()))
}

fn decode_chunked_body(body: &str) -> AcResult<String> {
    let mut rest = body;
    let mut decoded = String::new();
    while let Some((size_line, after_size)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim(), 16)
            .map_err(|err| AcError::validation("BROWSER-CDP_HTTP_CHUNK", err.to_string()))?;
        if size == 0 {
            break;
        }
        if after_size.len() < size {
            return Err(AcError::validation(
                "BROWSER-CDP_HTTP_CHUNK",
                "truncated chunked response from Chrome",
            ));
        }
        decoded.push_str(&after_size[..size]);
        rest = after_size.get(size + 2..).unwrap_or_default();
    }
    Ok(decoded)
}

fn redact_url(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            if let Some((base, query)) = part.split_once('?') {
                if query.contains("token")
                    || query.contains("secret")
                    || query.contains("key")
                    || query.contains("auth")
                {
                    return format!("{base}?[REDACTED]");
                }
            }
            part.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_viewports() -> [ViewportProfile; 3] {
    [
        ViewportProfile {
            name: "mobile",
            width: 390,
            height: 844,
        },
        ViewportProfile {
            name: "tablet",
            width: 820,
            height: 1180,
        },
        ViewportProfile {
            name: "desktop",
            width: 1440,
            height: 900,
        },
    ]
}

fn require_selector(html: &str, selector: &str) -> AcResult<()> {
    let needle = selector.trim_start_matches('#').trim_start_matches('.');
    if html.contains(&format!("id=\"{needle}\""))
        || html.contains(&format!("class=\"{needle}\""))
        || html.contains(&format!("name=\"{needle}\""))
        || html.contains(needle)
    {
        Ok(())
    } else {
        Err(AcError::validation(
            "BROWSER-SELECTOR_NOT_FOUND",
            format!("selector not found: {selector}"),
        ))
    }
}

fn route_after_submit(url: &str) -> String {
    if let Some((base, _)) = url.rsplit_once('/') {
        format!("{base}/dashboard")
    } else {
        "/dashboard".to_string()
    }
}

fn visible_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn controls(html: &str) -> Vec<String> {
    html.split('<')
        .filter(|part| {
            part.starts_with("button")
                || part.starts_with("input")
                || part.starts_with("select")
                || part.starts_with("a ")
        })
        .map(|part| part.split('>').next().unwrap_or(part).to_string())
        .collect()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

pub fn detect_project_capabilities(root: &Path) -> ProjectCapabilities {
    ProjectCapabilities {
        cargo: root.join("Cargo.toml").exists(),
        makefile: root.join("Makefile").exists(),
        package_json: root.join("package.json").exists(),
        python: has_python_test_capability(root),
        go: root.join("go.mod").exists(),
        browser: root.join("playwright.config.ts").exists()
            || root.join("playwright.config.js").exists(),
        security: root.join("deny.toml").exists()
            || root.join(".cargo").join("audit.toml").exists(),
    }
}

fn has_python_test_capability(root: &Path) -> bool {
    root.join("pytest.ini").exists()
        || root.join("tox.ini").exists()
        || root.join("setup.cfg").exists()
        || root.join("pyproject.toml").exists()
        || root.join("requirements.txt").exists()
}

fn command_for_layer(
    root: &Path,
    capabilities: &ProjectCapabilities,
    layer: VerificationLayer,
) -> Option<CommandSpec> {
    let (argv, detected_from) = if capabilities.cargo {
        match layer {
            VerificationLayer::Format => (vec!["cargo", "fmt", "--all"], "Cargo.toml"),
            VerificationLayer::Lint => (
                vec![
                    "cargo",
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
                "Cargo.toml",
            ),
            VerificationLayer::Typecheck => (
                vec!["cargo", "check", "--workspace", "--all-targets"],
                "Cargo.toml",
            ),
            VerificationLayer::Compile | VerificationLayer::Build => {
                (vec!["cargo", "build", "--workspace"], "Cargo.toml")
            }
            VerificationLayer::Unit | VerificationLayer::Integration => {
                (vec!["cargo", "test", "--workspace"], "Cargo.toml")
            }
            VerificationLayer::Security if capabilities.security => {
                (vec!["cargo", "audit"], "deny.toml")
            }
            _ => return None,
        }
    } else if capabilities.makefile
        && matches!(layer, VerificationLayer::Build | VerificationLayer::Unit)
    {
        (vec!["make", "test"], "Makefile")
    } else if capabilities.package_json {
        let package_manager = node_package_manager(root);
        let package_json = fs::read_to_string(root.join("package.json")).unwrap_or_default();
        let has_script = |script: &str| package_json_has_script(&package_json, script);
        match layer {
            VerificationLayer::Format if has_script("format") => (
                node_run_command(package_manager, "format", &["--check"]),
                "package.json",
            ),
            VerificationLayer::Lint if has_script("lint") => (
                node_run_command(package_manager, "lint", &[]),
                "package.json",
            ),
            VerificationLayer::Typecheck if has_script("typecheck") => (
                node_run_command(package_manager, "typecheck", &[]),
                "package.json",
            ),
            VerificationLayer::Unit | VerificationLayer::Integration if has_script("test") => {
                (node_test_command(package_manager), "package.json")
            }
            VerificationLayer::Build if has_script("build") => (
                node_run_command(package_manager, "build", &[]),
                "package.json",
            ),
            _ => return None,
        }
    } else if capabilities.python {
        match layer {
            VerificationLayer::Unit | VerificationLayer::Integration => {
                (vec!["pytest"], python_detected_from(root))
            }
            _ => return None,
        }
    } else if capabilities.go {
        match layer {
            VerificationLayer::Unit | VerificationLayer::Integration => {
                (vec!["go", "test", "./..."], "go.mod")
            }
            _ => return None,
        }
    } else {
        return None;
    };
    Some(CommandSpec {
        id: StableId::new("vcmd"),
        layer,
        argv: argv.iter().map(ToString::to_string).collect(),
        detected_from: root.join(detected_from).display().to_string(),
    })
}

fn node_package_manager(root: &Path) -> &'static str {
    if root.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if root.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm"
    }
}

fn node_test_command(package_manager: &str) -> Vec<&'static str> {
    match package_manager {
        "pnpm" => vec!["pnpm", "test"],
        "yarn" => vec!["yarn", "test"],
        _ => vec!["npm", "test"],
    }
}

fn node_run_command(
    package_manager: &str,
    script: &'static str,
    extra: &[&'static str],
) -> Vec<&'static str> {
    let mut argv = match package_manager {
        "pnpm" => vec!["pnpm", "run", script],
        "yarn" => vec!["yarn", "run", script],
        _ => vec!["npm", "run", script],
    };
    if !extra.is_empty() {
        argv.push("--");
        argv.extend(extra.iter().copied());
    }
    argv
}

fn package_json_has_script(package_json: &str, script: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(package_json) else {
        return false;
    };
    value
        .get("scripts")
        .and_then(Value::as_object)
        .is_some_and(|scripts| scripts.contains_key(script))
}

fn python_detected_from(root: &Path) -> &'static str {
    for marker in [
        "pytest.ini",
        "pyproject.toml",
        "tox.ini",
        "setup.cfg",
        "requirements.txt",
    ] {
        if root.join(marker).exists() {
            return marker;
        }
    }
    "pyproject.toml"
}

fn collect_tests(root: &Path, current: &Path, tests: &mut Vec<TestIdentity>) -> AcResult<()> {
    for entry in fs::read_dir(current)
        .map_err(|error| AcError::validation("VERIFY-TEST_DISCOVERY_FAILED", error.to_string()))?
    {
        let entry = entry.map_err(|error| {
            AcError::validation("VERIFY-TEST_DISCOVERY_FAILED", error.to_string())
        })?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_tests(root, &path, tests)?;
        } else if is_test_file(&path) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            tests.push(TestIdentity {
                id: StableId::new("test"),
                name: relative.clone(),
                path: relative,
                layer: VerificationLayer::Unit,
            });
        }
    }
    Ok(())
}

fn is_test_file(path: &Path) -> bool {
    let path = path.display().to_string();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with(".test.ts")
        || path.ends_with(".spec.ts")
}

fn related_test(source_path: &str, test_path: &str) -> bool {
    let source_stem = PathBuf::from(source_path)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    test_path.contains(&source_stem)
}

fn adversarial_findings(text: &str) -> Vec<VerificationFinding> {
    let lower = text.to_ascii_lowercase();
    let checks = [
        (
            "VERIFY-FALSE_DONE_PLACEHOLDER",
            ["todo", "placeholder", "stub"].as_slice(),
        ),
        (
            "VERIFY-FALSE_DONE_MOCK",
            ["mock in production", "hardcoded success"].as_slice(),
        ),
        (
            "VERIFY-MISSING_WIRING",
            ["unregistered route", "not registered", "unwired"].as_slice(),
        ),
        (
            "VERIFY-TEST_SKIPPED",
            ["#[ignore]", ".skip(", "skipped test"].as_slice(),
        ),
        (
            "VERIFY-WEAK_ASSERTION",
            ["assert!(true)", "assert_eq!(1, 1)", "always true"].as_slice(),
        ),
    ];
    checks
        .iter()
        .filter(|(_, needles)| needles.iter().any(|needle| lower.contains(needle)))
        .map(|(code, _)| blocking(code, "adversarial verifier found false-done risk"))
        .collect()
}

fn count_assertions(text: &str) -> usize {
    text.matches("assert").count() + text.matches("expect(").count()
}

fn blocking(code: &str, message: &str) -> VerificationFinding {
    VerificationFinding {
        code: code.to_string(),
        severity: FindingSeverity::Blocking,
        message: message.to_string(),
    }
}

fn environment_fingerprint() -> String {
    format!(
        "os:{};arch:{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn local_hash(content: &str) -> String {
    let hash = content
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_security::{
        ActiveAuthorization, ActiveEnvironment, ActiveSecurityAction, ActiveSecurityInput,
        ActiveValidationFixture, AiHarnessKind, AiSecurityInput, BaselineSecurityOrchestrator,
        SecurityPolicy,
    };

    #[test]
    fn browser_capture_requires_capability() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        let err = engine
            .capture_browser_observation("http://localhost", &mut evidence)
            .unwrap_err();
        assert_eq!(err.code(), "VERIFY-BROWSER_DENIED");
    }

    #[test]
    fn design_qa_reports_record_append_only_evidence() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        let version = StableId::new("dversion");
        let visual = engine
            .record_design_visual_evaluation(
                &version,
                false,
                vec!["generic cards hide hierarchy".to_string()],
                &mut evidence,
            )
            .unwrap();
        let responsive = engine
            .record_design_responsive_report(
                &version,
                vec!["mobile".to_string(), "desktop".to_string()],
                true,
                &mut evidence,
            )
            .unwrap();
        let accessibility = engine
            .record_design_accessibility_report(
                &version,
                true,
                vec!["labels".to_string(), "focus".to_string()],
                &mut evidence,
            )
            .unwrap();
        let functional = engine
            .record_design_functional_report(
                &version,
                true,
                vec!["primary flow".to_string()],
                &mut evidence,
            )
            .unwrap();
        assert!(!visual.passed);
        assert_eq!(responsive.viewports.len(), 2);
        assert_eq!(accessibility.checks.len(), 2);
        assert_eq!(functional.flows, vec!["primary flow".to_string()]);
        assert_eq!(evidence.len(), 4);
    }

    #[test]
    fn phase14_profile_commands_gates_and_targeted_tests_work() {
        let root = std::env::temp_dir().join(format!("agentcode-verify-{}", StableId::new("tmp")));
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname=\"demo\"\n").unwrap();
        fs::write(
            root.join("tests/auth_test.rs"),
            "#[test]\nfn auth_works() {}\n",
        )
        .unwrap();
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let profile = engine.derive_profile(
            StableId::new("task"),
            VerificationRisk::High,
            &detect_project_capabilities(&root),
        );
        assert!(profile.required_layers.contains(&VerificationLayer::Format));
        assert!(profile
            .required_layers
            .contains(&VerificationLayer::Integration));
        let commands = engine.detect_commands(&root, &profile);
        assert!(commands.iter().any(|command| command.argv[0] == "cargo"));

        let registry = engine.discover_tests(&root).unwrap();
        let selection = engine.select_tests(&registry, &["src/auth.rs".to_string()]);
        assert_eq!(selection.selected.len(), 1);
        assert!(!selection.broadened);

        let mut evidence = EvidenceStore::new();
        let gate = engine
            .normalize_gate_result(
                VerificationLayer::Unit,
                commands
                    .iter()
                    .find(|command| command.layer == VerificationLayer::Unit)
                    .cloned(),
                Some(0),
                "status:0\nok",
                false,
                &mut evidence,
            )
            .unwrap();
        assert_eq!(gate.status, GateStatus::Passed);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn project_aware_test_command_selection_supports_rust_node_python_and_go() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let profile = VerificationProfile {
            id: StableId::new("verifyprofile"),
            task_id: StableId::new("task"),
            risk: VerificationRisk::Medium,
            required_layers: vec![VerificationLayer::Unit],
            created_at: TimestampMillis::now(),
        };

        let rust = test_root("verify-rust");
        fs::write(rust.join("Cargo.toml"), "[package]\nname=\"demo\"\n").unwrap();
        assert_eq!(
            engine.detect_commands(&rust, &profile)[0].argv,
            vec!["cargo", "test", "--workspace"]
        );

        let node = test_root("verify-node");
        fs::write(
            node.join("package.json"),
            "{\"scripts\":{\"test\":\"node test.js\"}}",
        )
        .unwrap();
        fs::write(node.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();
        assert_eq!(
            engine.detect_commands(&node, &profile)[0].argv,
            vec!["pnpm", "test"]
        );

        let python = test_root("verify-python");
        fs::write(python.join("pytest.ini"), "[pytest]\n").unwrap();
        assert_eq!(
            engine.detect_commands(&python, &profile)[0].argv,
            vec!["pytest"]
        );

        let go = test_root("verify-go");
        fs::write(go.join("go.mod"), "module example.com/demo\n").unwrap();
        assert_eq!(
            engine.detect_commands(&go, &profile)[0].argv,
            vec!["go", "test", "./..."]
        );

        let none = test_root("verify-none");
        assert!(engine.detect_commands(&none, &profile).is_empty());

        for root in [rust, node, python, go, none] {
            let _ = fs::remove_dir_all(root);
        }
    }

    fn test_root(prefix: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("agentcode-{prefix}-{}", StableId::new("tmp")));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn phase14_evidence_requirement_freshness_and_integration_work() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        let requirement = StableId::new("req");
        let manifest = engine
            .record_evidence_manifest(
                "commit-a",
                "wt-a",
                "cargo test --workspace",
                GateStatus::Passed,
                vec![requirement.clone()],
                vec!["src/lib.rs".to_string()],
                &mut evidence,
            )
            .unwrap();
        let link = engine.link_requirement_evidence(requirement, &manifest);
        assert!(link.verified);
        assert!(
            !engine
                .invalidate_stale_evidence(&manifest, &["README.md".to_string()])
                .stale
        );
        assert!(
            engine
                .invalidate_stale_evidence(&manifest, &["src/lib.rs".to_string()])
                .stale
        );
        let integration = engine
            .verify_integration(&[manifest], &mut evidence)
            .unwrap();
        assert!(integration.passed);
        assert!(integration.ran_broad_tests);
    }

    #[test]
    fn phase18_phase19_security_reports_are_evidence_gated_by_capability() {
        let orchestrator = BaselineSecurityOrchestrator::new(SecurityPolicy::baseline());
        let active_input = ActiveSecurityInput {
            repository_id: StableId::new("repo"),
            commit: "verify-p18".to_string(),
            authorization: ActiveAuthorization {
                id: StableId::new("authz"),
                target: "http://fixture.local".to_string(),
                environment: ActiveEnvironment::AuthorizedLab,
                allowed_targets: vec!["http://fixture.local".to_string()],
                cloud_accounts: Vec::new(),
                credential_ref: None,
                rate_limit_per_minute: 10,
                concurrency_limit: 1,
                forbidden_actions: vec!["destructive-production-change".to_string()],
                expires_at: TimestampMillis::from_millis(
                    TimestampMillis::now().as_millis() + 60_000,
                ),
                cleanup_required: true,
            },
            requested_actions: vec![ActiveSecurityAction::DastSpider],
            fixture: Some(ActiveValidationFixture {
                id: StableId::new("fixture"),
                vulnerable_route: "http://fixture.local/admin".to_string(),
                synthetic_account: "user-a".to_string(),
                canary_record: "canary".to_string(),
            }),
            redirect_observations: Vec::new(),
            cloud_resources: Vec::new(),
        };
        let active_report = orchestrator.run_active_security(&active_input).unwrap();
        let ai_report = orchestrator
            .run_ai_security(&AiSecurityInput {
                repository_id: StableId::new("repo"),
                commit: "verify-p19".to_string(),
                files: vec![(
                    "src/agent.rs".to_string(),
                    "openai user_prompt rag tool_call mcp agent SECRET=synthetic".to_string(),
                )],
                selected_harnesses: vec![AiHarnessKind::Promptfoo],
            })
            .unwrap();

        let denied = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        assert_eq!(
            denied
                .record_active_security_report(&active_report, &mut evidence)
                .unwrap_err()
                .code(),
            "VERIFY-ACTIVE_SECURITY_DENIED"
        );

        let allowed =
            VerificationEngine::new(CapabilityPolicy::new().allow(Capability::SecurityScan));
        let active_evidence = allowed
            .record_active_security_report(&active_report, &mut evidence)
            .unwrap();
        assert_eq!(active_evidence.scanner, "advanced-security");
        let ai_evidence = allowed
            .record_ai_security_report(&ai_report, &mut evidence)
            .unwrap();
        assert_eq!(ai_evidence.scanner, "ai-security");
        assert!(ai_evidence.findings > 0);
    }

    #[test]
    fn phase14_independent_verifier_rejects_false_done_and_cannot_edit() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        let context = engine
            .build_verifier_context(
                "wire backend route",
                "backend service not registered; placeholder implementation",
                "assert!(true)",
                Vec::new(),
            )
            .unwrap();
        assert!(!context.can_edit);
        let report = engine
            .run_independent_verifier(&context, &mut evidence)
            .unwrap();
        assert!(!report.passed);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "VERIFY-MISSING_WIRING"));

        let mut edit_context = context;
        edit_context.can_edit = true;
        assert_eq!(
            engine
                .run_independent_verifier(&edit_context, &mut evidence)
                .unwrap_err()
                .code(),
            "VERIFY-VERIFIER_EDIT_DENIED"
        );
    }

    #[test]
    fn phase14_test_tampering_detects_weakened_assertions_and_skips() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let base = "#[test]\nfn works() { assert_eq!(answer(), 42); }\n";
        let changed = "#[test]\n#[ignore]\nfn works() { assert!(true); }\n";
        let report = engine.detect_test_tampering(base, changed);
        assert!(report.suspicious);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "VERIFY-TEST_SKIPPED"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "VERIFY-WEAK_ASSERTION"));
    }

    #[test]
    fn phase14_final_audit_and_completion_gate_block_worker_text_bypass() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        let incomplete = engine
            .final_audit(
                FinalAuditInput {
                    original_goal: "implement auth".to_string(),
                    requirements: vec!["route wired".to_string(), "tests prove it".to_string()],
                    verified_requirement_ids: vec![StableId::new("req")],
                    evidence_refs: vec![StableId::new("ev")],
                    worker_completion_text: "done, trust me".to_string(),
                    unresolved_limitations: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert!(!incomplete.passed);
        assert!(incomplete.return_to_repair);
        assert!(
            !engine
                .completion_gate(&incomplete, "done, trust me")
                .allowed
        );

        let complete = engine
            .final_audit(
                FinalAuditInput {
                    original_goal: "implement auth".to_string(),
                    requirements: vec!["route wired".to_string()],
                    verified_requirement_ids: vec![StableId::new("req")],
                    evidence_refs: vec![StableId::new("ev")],
                    worker_completion_text: "verified with evidence".to_string(),
                    unresolved_limitations: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert!(complete.passed);
        assert!(
            engine
                .completion_gate(&complete, "verified with evidence")
                .allowed
        );
        assert!(!engine.completion_gate(&complete, "").allowed);
    }

    #[test]
    fn mandatory_acceptance_coverage_blocks_until_all_required_items_are_verified() {
        let engine = VerificationEngine::new(CapabilityPolicy::new());
        let mut evidence = EvidenceStore::new();
        let evidence_ref = evidence
            .append(
                EvidenceKind::TestReport,
                Provenance {
                    source: "acceptance-test".to_string(),
                    commit: None,
                    worktree: None,
                    tool: Some("verification".to_string()),
                },
                "mem://acceptance/proof",
                "verified",
            )
            .unwrap();
        let required_a = StableId::from_existing("criterion-a").unwrap();
        let required_b = StableId::from_existing("criterion-b").unwrap();
        let only_a = engine
            .final_audit(
                FinalAuditInput {
                    original_goal: "ship covered requirements".to_string(),
                    requirements: vec!["A".to_string(), "B".to_string()],
                    verified_requirement_ids: vec![required_a.clone()],
                    evidence_refs: vec![evidence_ref.clone()],
                    worker_completion_text: "all requirements are satisfied".to_string(),
                    unresolved_limitations: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert!(
            !engine
                .completion_gate(&only_a, "all requirements are satisfied")
                .allowed
        );

        let required_covered_optional_uncovered = engine
            .final_audit(
                FinalAuditInput {
                    original_goal: "ship covered requirements".to_string(),
                    requirements: vec!["A".to_string(), "B".to_string()],
                    verified_requirement_ids: vec![required_a, required_b],
                    evidence_refs: vec![evidence_ref],
                    worker_completion_text: "verified by deterministic evidence".to_string(),
                    unresolved_limitations: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert!(
            engine
                .completion_gate(
                    &required_covered_optional_uncovered,
                    "verified by deterministic evidence"
                )
                .allowed
        );
    }

    #[test]
    fn browser_event_evidence_buffers_are_bounded_to_recent_events() {
        let mut events = Vec::new();
        for index in 0..(MAX_BROWSER_EVENT_EVIDENCE + 10) {
            push_bounded(&mut events, format!("event-{index}"));
        }
        assert_eq!(events.len(), MAX_BROWSER_EVENT_EVIDENCE);
        assert_eq!(events.first().unwrap(), "event-10");
        assert_eq!(
            events.last().unwrap(),
            &format!("event-{}", MAX_BROWSER_EVENT_EVIDENCE + 9)
        );
    }

    #[test]
    fn phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work() {
        let mut runtime = BrowserRuntime::deterministic_harness_for_tests(
            CapabilityPolicy::new().allow(Capability::BrowserAutomation),
        );
        let task_id = StableId::new("task");
        let process = runtime.launch(task_id.clone()).unwrap();
        assert_eq!(process.state, BrowserProcessState::Running);
        assert!(process.profile_dir.contains("isolated-profile"));
        let session = runtime
            .create_session(task_id.clone(), process.id.clone())
            .unwrap();
        assert_eq!(session.task_id, task_id);
        assert!(session.sensitive);
        let mut evidence = EvidenceStore::new();
        let html = r#"
            <main>
              <h1>Login</h1>
              <input id="email" name="email" />
              <input id="password" name="password" />
              <button id="submit">Sign in</button>
              <script>console.error("boom")</script>
              <img src="missing.png" />
            </main>
        "#;
        runtime
            .act(
                &session.id,
                BrowserAction::OpenHtmlForTest {
                    url: "http://127.0.0.1:3000/login".to_string(),
                    html: html.to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Type {
                    selector: "#email".to_string(),
                    text: "demo@example.test".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Type {
                    selector: "#password".to_string(),
                    text: "secret".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        let click = runtime
            .act(
                &session.id,
                BrowserAction::Click {
                    selector: "#submit".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        assert!(click.url.ends_with("/dashboard"));
        let dom = runtime.inspect_dom(&session.id, &mut evidence).unwrap();
        assert!(dom.visible_text.contains("Login"));
        assert!(dom
            .controls
            .iter()
            .any(|control| control.contains("button")));
        let diagnostics = runtime.diagnostics(&session.id, &mut evidence).unwrap();
        assert_eq!(diagnostics.console_errors.len(), 1);
        assert_eq!(diagnostics.network_failures.len(), 1);
        let shot = runtime
            .capture_screenshot(
                &session.id,
                task_id,
                "commit-p15",
                runtime.default_viewports()[0],
                &mut evidence,
            )
            .unwrap();
        assert_eq!(shot.viewport.name, "mobile");
        let visual = runtime.visual_qa(&shot, &mut evidence).unwrap();
        assert!(!visual.passed);
        assert_eq!(visual.screenshot_ref, shot.evidence_ref);
    }

    #[test]
    fn batch4_browser_runtime_drives_real_chrome_dom_and_evidence() {
        if discover_chromium_executable().is_none() {
            let mut runtime =
                BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
            assert_eq!(
                runtime.launch(StableId::new("task")).unwrap_err().code(),
                "BROWSER-UNAVAILABLE"
            );
            return;
        }
        let Ok(listener) = std::net::TcpListener::bind("127.0.0.1:0") else {
            return;
        };
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buffer = [0_u8; 2048];
                        let n = stream.read(&mut buffer).unwrap_or(0);
                        let request = String::from_utf8_lossy(&buffer[..n]);
                        let path = request
                            .lines()
                            .next()
                            .and_then(|line| line.split_whitespace().nth(1))
                            .unwrap_or("/");
                        let (status, body, content_type) = if path.starts_with("/missing") {
                            (
                                "404 Not Found",
                                "<!doctype html><html><body>missing</body></html>",
                                "text/html",
                            )
                        } else {
                            (
                                "200 OK",
                                r#"<!doctype html>
                                <html>
                                  <head><title>AgentCode Browser Test</title></head>
                                  <body>
                                    <main>
                                      <h1 id="title">Login</h1>
                                      <input id="email" name="email" aria-label="Email" />
                                      <select id="role" aria-label="Role">
                                        <option value="user">User</option>
                                        <option value="admin">Admin</option>
                                      </select>
                                      <button id="submit" onclick="document.getElementById('title').innerText='Dashboard'; history.pushState({}, '', '/dashboard');">Sign in</button>
                                      <script>console.error("batch4 real console error")</script>
                                      <img src="/missing.png" />
                                    </main>
                                  </body>
                                </html>"#,
                                "text/html",
                            )
                        };
                        let response = format!(
                            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    Err(_) => break,
                }
            }
        });

        let mut runtime =
            BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
        let mut evidence = EvidenceStore::new();
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        assert_eq!(process.mode, BrowserAdapterMode::ChromiumCdp);
        assert_eq!(process.state, BrowserProcessState::Running);
        assert!(process.profile_dir.contains("agentcode-browser-profile"));
        let session = runtime
            .create_session(task.clone(), process.id.clone())
            .unwrap();
        let url = format!("http://{addr}/login");
        runtime
            .act(&session.id, BrowserAction::Navigate { url }, &mut evidence)
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Wait { millis: 250 },
                &mut evidence,
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Type {
                    selector: "#email".to_string(),
                    text: "demo@example.test".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Select {
                    selector: "#role".to_string(),
                    value: "admin".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        let click = runtime
            .act(
                &session.id,
                BrowserAction::Click {
                    selector: "#submit".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        assert!(click.url.ends_with("/dashboard"));
        let dom = runtime.inspect_dom(&session.id, &mut evidence).unwrap();
        assert!(dom.visible_text.contains("Dashboard"));
        assert!(dom.controls.iter().any(|control| control.contains("Email")));
        assert!(dom
            .accessibility_tree
            .iter()
            .any(|node| node.contains("Sign in") || node.contains("Email")));
        let diagnostics = runtime.diagnostics(&session.id, &mut evidence).unwrap();
        assert!(diagnostics
            .console_errors
            .iter()
            .any(|error| error.contains("batch4 real console error")));
        let shot = runtime
            .capture_screenshot(
                &session.id,
                task,
                "commit-batch4",
                runtime.default_viewports()[0],
                &mut evidence,
            )
            .unwrap();
        assert_eq!(shot.viewport.name, "mobile");
        assert!(fs::metadata(&shot.artifact_uri).unwrap().len() > 0);
        let visual = runtime.visual_qa(&shot, &mut evidence).unwrap();
        assert!(visual.passed);
        let _ = runtime.act(
            &session.id,
            BrowserAction::Navigate {
                url: format!("http://{addr}/missing.html"),
            },
            &mut evidence,
        );
        let diagnostics = runtime.diagnostics(&session.id, &mut evidence).unwrap();
        assert!(diagnostics
            .network_failures
            .iter()
            .any(|failure| { failure.contains("missing.html") || failure.contains("net::ERR") }));
        runtime.mark_crashed(&process.id).unwrap();
    }

    #[test]
    fn phase15_browser_capability_denial_and_selector_failure_are_explicit() {
        let mut denied = BrowserRuntime::new(CapabilityPolicy::new());
        assert_eq!(
            denied.launch(StableId::new("task")).unwrap_err().code(),
            "BROWSER-CAPABILITY_DENIED"
        );
        let mut runtime = BrowserRuntime::deterministic_harness_for_tests(
            CapabilityPolicy::new().allow(Capability::BrowserAutomation),
        );
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime.create_session(task, process.id).unwrap();
        let mut evidence = EvidenceStore::new();
        runtime
            .act(
                &session.id,
                BrowserAction::OpenHtmlForTest {
                    url: "http://localhost/login".to_string(),
                    html: "<button id=\"ok\">OK</button>".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(
            runtime
                .act(
                    &session.id,
                    BrowserAction::Click {
                        selector: "#missing".to_string()
                    },
                    &mut evidence
                )
                .unwrap_err()
                .code(),
            "BROWSER-SELECTOR_NOT_FOUND"
        );
    }

    #[test]
    fn phase15_dev_server_responsive_profiles_and_crash_recovery_work() {
        let mut runtime = BrowserRuntime::deterministic_harness_for_tests(
            CapabilityPolicy::new().allow(Capability::BrowserAutomation),
        );
        let task = StableId::new("task");
        let dev_server = runtime
            .manage_dev_server(
                task.clone(),
                vec!["npm".to_string(), "run".to_string(), "dev".to_string()],
                3000,
                "http://127.0.0.1:3000/login",
            )
            .unwrap();
        assert!(dev_server.process_alive);
        assert!(dev_server.http_ready);
        assert!(dev_server.route_loadable);
        assert_eq!(
            runtime
                .manage_dev_server(task.clone(), Vec::new(), 3000, "http://127.0.0.1:3000")
                .unwrap_err()
                .code(),
            "BROWSER-DEV_SERVER_INVALID"
        );
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime
            .create_session(task.clone(), process.id.clone())
            .unwrap();
        let mut evidence = EvidenceStore::new();
        runtime
            .act(
                &session.id,
                BrowserAction::OpenHtmlForTest {
                    url: dev_server.ready_url,
                    html: "<h1>Ready</h1>".to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        let shots = runtime
            .default_viewports()
            .into_iter()
            .map(|viewport| {
                runtime
                    .capture_screenshot(
                        &session.id,
                        task.clone(),
                        "commit-p15",
                        viewport,
                        &mut evidence,
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(shots.len(), 3);
        runtime.mark_crashed(&process.id).unwrap();
        let recovered = runtime
            .recover_crashed_session(&session.id, vec![shots[0].evidence_ref.clone()])
            .unwrap();
        assert_ne!(recovered.id, session.id);
        assert_eq!(
            recovered.current_url,
            Some("http://127.0.0.1:3000/login".to_string())
        );
        assert_eq!(recovered.stale_evidence_refs.len(), 1);
    }
}
