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
    pub required_requirement_ids: Vec<StableId>,
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
    /// Honest degradation marker: Chrome was relaunched with its internal
    /// sandbox disabled because the host OS could not provide the sandbox
    /// profile Chrome needs (e.g. macOS seatbelt unavailable to the browser).
    /// AgentCode's own process-restricted boundary still applies.
    pub internal_sandbox_degraded: bool,
    /// Honest shutdown marker (batch N2): true when Chrome was closed via
    /// the CDP `Browser.close` handshake (or a normal exit observed before
    /// any signal), false when it had to be terminated.  A SIGKILL-only
    /// teardown is what produces the user-visible "Chrome quit
    /// unexpectedly" crash dialogs; this flag makes the shutdown class
    /// inspectable evidence.
    pub graceful_shutdown: bool,
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
    Navigate {
        url: String,
    },
    OpenHtmlForTest {
        url: String,
        html: String,
    },
    Click {
        selector: String,
    },
    /// Click at raw page VIEWPORT coordinates (interactive inbuilt browser).
    ClickAt {
        x: i32,
        y: i32,
    },
    /// Type a single key event into the focused element (interactive
    /// inbuilt browser) — key is a key literal like "a", "Enter", "Tab".
    PressKey {
        key: String,
    },
    /// Wheel scroll by (dx, dy) at viewport coordinates.
    ScrollBy {
        dx: i32,
        dy: i32,
        x: i32,
        y: i32,
    },
    Type {
        selector: String,
        text: String,
    },
    Select {
        selector: String,
        value: String,
    },
    Scroll {
        y: i32,
    },
    Wait {
        millis: u64,
    },
    /// Evaluate a JS expression in the page and return its JSON result
    /// (diagnostics + design QA measurements; never a control-flow path).
    Evaluate {
        script: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserActionResult {
    pub session_id: StableId,
    pub action: String,
    pub ok: bool,
    pub url: String,
    pub evidence_ref: StableId,
    /// Evaluate actions: the JSON value the expression returned (None for
    /// every other action) — consumed by diagnostics and design QA.
    pub value: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomSnapshot {
    pub session_id: StableId,
    pub visible_text: String,
    pub controls: Vec<String>,
    pub accessibility_tree: Vec<String>,
    pub evidence_ref: StableId,
}

/// One interactive element with its rendered size (design QA touch-target
/// check, WCAG 2.5.8).  Sizes come from the real layout engine via CDP.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignTouchTarget {
    pub tag: String,
    pub label: String,
    pub role: String,
    pub width: i64,
    pub height: i64,
    pub visible: bool,
}

/// Real design-QA metrics measured in a live page (Doc 06 §49-50).
/// `needs_manual_review: true` marks every metric the current runtime
/// cannot measure honestly (deterministic harness has no layout engine).
/// Layout-dependent fields are `None` — never a fabricated verdict.
#[derive(Clone, Debug, PartialEq)]
pub struct DesignMetrics {
    pub mode: String,
    pub needs_manual_review: bool,
    pub viewport: ViewportProfile,
    /// Some(true) = horizontal overflow detected; Some(false) = fits;
    /// None = not measurable in this runtime.
    pub horizontal_overflow: Option<bool>,
    pub has_viewport_meta: bool,
    pub scroll_width: Option<i64>,
    pub client_width: Option<i64>,
    pub touch_targets: Vec<DesignTouchTarget>,
    pub small_touch_targets: Vec<String>,
    pub unlabeled_controls: Vec<String>,
    pub control_count: Option<u32>,
    pub heading_skips: Vec<String>,
    pub focus_visible_support: Option<bool>,
    pub has_lang_attribute: bool,
    pub document_title: Option<String>,
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
    /// Inbuilt-browser panel: the single page session the panel reuses
    /// across navigations (Codex-style embedded browser).
    panel_session: Option<StableId>,
}

struct RealBrowserProcess {
    child: Child,
    _profile_dir: TempDir,
    port: u16,
    /// Browser-endpoint websocket path from DevToolsActivePort (line 2).
    /// Modern Chrome only serves websockets on this endpoint; page-level
    /// /devtools/page/<id> handshakes return 404.
    browser_ws_path: String,
}

struct RealBrowserPage {
    client: CdpClient,
    url: String,
    viewport: ViewportProfile,
}

struct CdpClient {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: u64,
    /// Flat-session identifier (Target.attachToTarget with flatten:true).
    /// Commands are routed to the page session; None speaks on the browser
    /// endpoint directly.
    session_id: Option<String>,
    console_errors: Vec<String>,
    page_errors: Vec<String>,
    network_failures: Vec<String>,
    http_status: u16,
    current_url_hint: Option<String>,
    document_ready_seen: bool,
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
        let required_ids = input
            .required_requirement_ids
            .iter()
            .collect::<BTreeSet<_>>();
        let verified_ids = input
            .verified_requirement_ids
            .iter()
            .collect::<BTreeSet<_>>();
        if input.required_requirement_ids.len() != input.requirements.len()
            || !required_ids.is_subset(&verified_ids)
        {
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
            panel_session: None,
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
            panel_session: None,
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
            internal_sandbox_degraded: false,
            graceful_shutdown: false,
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
        let mut panel_page_session_id: Option<StableId> = None;
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            let real_process = self.real_processes.get(&process_id).ok_or_else(|| {
                AcError::validation(
                    "BROWSER-PROCESS_UNKNOWN",
                    "real browser process is not registered",
                )
            })?;
            let page = RealBrowserPage::create(real_process.port, &real_process.browser_ws_path)?;
            // One CDP target per SESSION (tab): keying by session lets the
            // panel hold multiple tabs in ONE browser process.  The id is
            // generated ONCE and shared by the page map and the record.
            panel_page_session_id = Some(StableId::new("browsersession"));
            self.real_pages
                .insert(panel_page_session_id.clone().unwrap(), page);
        }
        let session = BrowserSessionRecord {
            id: panel_page_session_id
                .take()
                .unwrap_or_else(|| StableId::new("browsersession")),
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
            BrowserAction::ClickAt { .. } => "click_at",
            BrowserAction::PressKey { .. } => "press_key",
            BrowserAction::ScrollBy { .. } => "scroll_by",
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
            BrowserAction::Evaluate { .. } => "evaluate",
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
            value: None,
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

    /// Real layout/interaction metrics for design QA (Doc 06 §49-50).
    ///
    /// In CDP mode this evaluates a metrics bundle in the live page:
    ///   - horizontal overflow (`scrollWidth > clientWidth + 1` on document
    ///     element and body)
    ///   - viewport meta presence and initial-scale
    ///   - touch-target sizes of interactive elements (WCAG 2.5.8 ≥24px)
    ///   - control inventory with roles/names and unlabeled controls
    ///   - heading hierarchy (skipped levels)
    ///   - focus visibility capability (focus-visible styling)
    ///   - lang attribute and document title
    ///
    /// In deterministic mode there is no layout engine, so every metric that
    /// needs real layout is returned as `null` and `needs_manual_review`
    /// is true — the caller must fail honestly instead of fabricating
    /// pass/fail verdicts.
    /// Apply a viewport to a live session (design QA responsive checks).
    /// CDP mode sets device metrics on the page; deterministic mode records
    /// the viewport in the page state.
    pub fn set_viewport(
        &mut self,
        session_id: &StableId,
        viewport: ViewportProfile,
    ) -> AcResult<()> {
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            let page = self.real_page_mut(session_id)?;
            page.viewport = viewport;
            page.client.call(
                "Emulation.setDeviceMetricsOverride",
                json!({
                    "width": viewport.width,
                    "height": viewport.height,
                    "deviceScaleFactor": 1,
                    "mobile": viewport.width < 600
                }),
            )?;
            page.client.drain_events(Duration::from_millis(100))?;
            return Ok(());
        }
        if let Some(page) = self.pages.get_mut(session_id) {
            page.viewport = viewport;
        }
        Ok(())
    }

    pub fn design_metrics(&mut self, session_id: &StableId) -> AcResult<DesignMetrics> {
        if self.mode == BrowserAdapterMode::ChromiumCdp {
            return self.design_metrics_chromium_cdp(session_id);
        }
        // Deterministic harness: only structural facts available from the
        // fixture HTML — anything layout-dependent is explicitly null.
        // Structural a11y facts (labels, roles, control inventory, heading
        // order) ARE computable from HTML and are reported honestly;
        // touch-target sizes and overflow need a layout engine and stay
        // None so the caller flags NEEDS_MANUAL_REVIEW for them.
        let page = self.page(session_id)?;
        let has_viewport_meta =
            page.html.contains("name=\"viewport\"") || page.html.contains("name='viewport'");
        let has_lang = page.html.contains("<html") && page.html.contains("lang=");

        // Structural control inventory from the fixture HTML.
        // Deterministic mode extracts what static HTML can honestly provide:
        // aria-label/placeholder/title attributes, button inner text, and
        // <label> elements (any form labeling).  Sizes stay 0 — they need
        // a layout engine and are reported as unmeasured.
        let control_tags = ["<button", "<input", "<select", "<textarea", "<a "];
        let mut touch_targets = Vec::new();
        for tag in control_tags {
            let mut search = page.html.as_str();
            while let Some(pos) = search.find(tag) {
                let end = (pos + 200).min(search.len());
                let snippet = &search[pos..end];
                let mut label = extract_attribute(snippet, "aria-label")
                    .or_else(|| extract_attribute(snippet, "placeholder"))
                    .or_else(|| extract_attribute(snippet, "title"))
                    .unwrap_or_default();
                // Buttons and links carry their inner text as the label.
                if label.is_empty() {
                    if let Some(close) = snippet.find('>') {
                        let inner = &snippet[close + 1..];
                        if let Some(text) = inner.find("</") {
                            let text_value = inner[..text].trim();
                            if !text_value.is_empty() && text_value.len() < 80 {
                                label = text_value.to_string();
                            }
                        }
                    }
                }
                // A <label> element anywhere in the form labels form fields
                // (fixture-level structural approximation).
                if label.is_empty()
                    && matches!(tag, "<input" | "<select" | "<textarea")
                    && page.html.contains("<label")
                {
                    label = "(labelled)".to_string();
                }
                touch_targets.push(DesignTouchTarget {
                    tag: tag.trim_start_matches('<').trim_end().to_string(),
                    label,
                    role: if snippet.contains("role=") {
                        "explicit".to_string()
                    } else {
                        String::new()
                    },
                    width: 0,
                    height: 0,
                    visible: true,
                });
                search = &search[pos + tag.len()..];
            }
        }
        let unlabeled_controls: Vec<String> = touch_targets
            .iter()
            .filter(|target| target.label.is_empty() && target.tag != "a")
            .map(|target| format!("{} has no accessible name", target.tag))
            .collect();
        // Heading order from the fixture HTML (document order preserved).
        let ordered: Vec<i64> = {
            let mut positions: Vec<(usize, i64)> = Vec::new();
            for level in 1..=6 {
                let tag = format!("<h{level}");
                let mut search = page.html.as_str();
                while let Some(pos) = search.find(&tag) {
                    positions.push((pos, level as i64));
                    search = &search[pos + tag.len()..];
                }
            }
            positions.sort();
            positions.into_iter().map(|(_, level)| level).collect()
        };
        let mut heading_skips = Vec::new();
        for pair in ordered.windows(2) {
            if pair[1] > pair[0] + 1 {
                heading_skips.push(format!("heading jumps from h{} to h{}", pair[0], pair[1]));
            }
        }

        Ok(DesignMetrics {
            mode: "deterministic".to_string(),
            needs_manual_review: true,
            viewport: page.viewport,
            horizontal_overflow: None,
            has_viewport_meta,
            scroll_width: None,
            client_width: None,
            touch_targets,
            small_touch_targets: Vec::new(), // needs real layout
            unlabeled_controls,
            control_count: None,
            heading_skips,
            focus_visible_support: None,
            has_lang_attribute: has_lang,
            document_title: None,
        })
    }

    fn design_metrics_chromium_cdp(&mut self, session_id: &StableId) -> AcResult<DesignMetrics> {
        let viewport = self
            .real_pages
            .values()
            .next()
            .map(|p| p.viewport)
            .unwrap_or(ViewportProfile {
                name: "desktop",
                width: 1440,
                height: 900,
            });
        let page = self.real_page_mut(session_id)?;

        // Layout overflow + viewport readiness.
        let overflow_json = page.client.evaluate_value(
            "(() => { const d = document.documentElement, b = document.body;\
             return JSON.stringify({\
                 sw: Math.max(d.scrollWidth, b ? b.scrollWidth : 0),\
                 cw: d.clientWidth,\
                 meta: !!document.querySelector('meta[name=viewport]')\
             }); })()",
        )?;
        let overflow: Value =
            serde_json::from_str(overflow_json.as_str().unwrap_or("{}")).unwrap_or(Value::Null);

        // Touch targets: every interactive element with its rendered size.
        let targets_json = page.client.evaluate_value(
            "(() => {\
                 const sels = 'button, a[href], input, select, textarea, [role=button], [role=link], [role=tab], [role=switch], [role=checkbox], [onclick]';\
                 return JSON.stringify(Array.from(document.querySelectorAll(sels)).map(el => {\
                     const r = el.getBoundingClientRect();\
                     return {\
                         tag: el.tagName.toLowerCase(),\
                         label: (el.getAttribute('aria-label') || el.innerText || el.value || el.getAttribute('title') || '').trim().slice(0, 80),\
                         role: el.getAttribute('role') || '',\
                         w: Math.round(r.width),\
                         h: Math.round(r.height),\
                         visible: r.width > 0 && r.height > 0\
                     };\
                 })); })()",
        )?;
        let targets: Vec<Value> =
            serde_json::from_str(targets_json.as_str().unwrap_or("[]")).unwrap_or_default();

        let touch_targets: Vec<DesignTouchTarget> = targets
            .iter()
            .map(|t| DesignTouchTarget {
                tag: t["tag"].as_str().unwrap_or("").to_string(),
                label: t["label"].as_str().unwrap_or("").to_string(),
                role: t["role"].as_str().unwrap_or("").to_string(),
                width: t["w"].as_i64().unwrap_or(0),
                height: t["h"].as_i64().unwrap_or(0),
                visible: t["visible"].as_bool().unwrap_or(false),
            })
            .collect();
        let small_touch_targets: Vec<String> = touch_targets
            .iter()
            .filter(|t| t.visible && (t.width < 24 || t.height < 24))
            .map(|t| {
                format!(
                    "{} '{}' is {}x{}px (below 24px minimum)",
                    t.tag, t.label, t.width, t.height
                )
            })
            .collect();
        let unlabeled_controls: Vec<String> = touch_targets
            .iter()
            .filter(|t| t.label.is_empty() && t.tag != "a")
            .map(|t| format!("{} has no accessible name", t.tag))
            .collect();
        let control_count = touch_targets.len() as u32;

        // Heading hierarchy: h1..h6 order with skipped levels.
        let headings_json = page.client.evaluate_value(
            "(() => JSON.stringify(Array.from(document.querySelectorAll('h1,h2,h3,h4,h5,h6'))\
                 .map(h => parseInt(h.tagName[1]))))()",
        )?;
        let levels: Vec<i64> =
            serde_json::from_str(headings_json.as_str().unwrap_or("[]")).unwrap_or_default();
        let mut heading_skips = Vec::new();
        for pair in levels.windows(2) {
            if pair[1] > pair[0] + 1 {
                heading_skips.push(format!("heading jumps from h{} to h{}", pair[0], pair[1]));
            }
        }

        // Focus visibility: does the UA default or author CSS style focus?
        let focus_visible = page.client.evaluate_value(
            "(() => {\
                 const probe = document.querySelector('button, a[href], [role=button]');\
                 if (!probe) return null;\
                 try { probe.focus(); } catch (e) {}\
                 const st = getComputedStyle(probe);\
                 const outline = st.outlineStyle !== 'none' && st.outlineWidth !== '0px';\
                 const shadow = (st.boxShadow || '') !== '';\
                 const underline = st.textDecorationLine.includes('underline');\
                 return !!(outline || shadow || underline); })()",
        )?;

        let title = page.client.evaluate_value("document.title || ''")?;

        let scroll_width = overflow["sw"].as_i64();
        let client_width = overflow["cw"].as_i64();
        let horizontal_overflow = match (scroll_width, client_width) {
            (Some(sw), Some(cw)) => Some(sw > cw + 1),
            _ => None,
        };

        let has_lang_attribute = page
            .client
            .evaluate_value("!!document.documentElement.getAttribute('lang')")?
            .as_bool()
            .unwrap_or(false);

        Ok(DesignMetrics {
            mode: "cdp".to_string(),
            needs_manual_review: false,
            viewport,
            horizontal_overflow,
            has_viewport_meta: overflow["meta"].as_bool().unwrap_or(false),
            scroll_width,
            client_width,
            touch_targets,
            small_touch_targets,
            unlabeled_controls,
            control_count: Some(control_count),
            heading_skips,
            focus_visible_support: focus_visible.as_bool(),
            has_lang_attribute,
            document_title: title.as_str().map(ToString::to_string),
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

    /// Graceful, explicit close of a live browser process (batch N2).
    /// Sends CDP `Browser.close`, waits, falls back to SIGTERM/SIGKILL.
    /// Records `graceful_shutdown` on the process record and transitions
    /// the state machine Running/Ready → Closing → Closed.  This is the
    /// production close path callers should use instead of dropping the
    /// runtime with live processes (Drop uses the same machinery).
    pub fn close_process(&mut self, process_id: &StableId) -> AcResult<BrowserProcessRecord> {
        let process = self.processes.get_mut(process_id).ok_or_else(|| {
            AcError::validation(
                "BROWSER-PROCESS_UNKNOWN",
                "browser process is not registered",
            )
        })?;
        if !matches!(
            process.state,
            BrowserProcessState::Running | BrowserProcessState::Ready
        ) {
            return Err(AcError::validation(
                "BROWSER-PROCESS_NOT_RUNNING",
                "only a running or ready browser process can be closed",
            ));
        }
        process.state = BrowserProcessState::Closing;
        let mut graceful = false;
        // Session-keyed page cleanup: every tab (session) owned by this
        // process drops its CDP page.
        let owned: Vec<StableId> = self
            .sessions
            .iter()
            .filter(|(_, sess)| &sess.process_id == process_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in owned {
            self.real_pages.remove(&id);
        }
        if let Some(real) = self.real_processes.remove(process_id) {
            let mut real = real;
            graceful = terminate_browser_process_gracefully(&mut real);
            // The temp profile dir is dropped with `real` only after the
            // child has fully exited (terminate_browser_process_gracefully
            // guarantees that before returning).
        }
        let process = self.processes.get_mut(process_id).ok_or_else(|| {
            AcError::validation(
                "BROWSER-PROCESS_UNKNOWN",
                "browser process is not registered",
            )
        })?;
        process.state = BrowserProcessState::Closed;
        process.graceful_shutdown = graceful;
        Ok(process.clone())
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
            // Chrome-crash-dialog fix: never SIGKILL a live Chrome from a
            // crash LABEL alone (a stale websocket is not proof the process
            // is dead).  Escalate Browser.close -> SIGTERM -> SIGKILL and
            // let the graceful path delete the temp profile.
            let _ = terminate_browser_process_gracefully(&mut real_process);
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
        let (child, profile_dir, port, browser_ws_path, internal_sandbox_degraded) =
            self.launch_chrome_with_degraded_retry(&executable)?;
        let process = BrowserProcessRecord {
            id: StableId::new("browserproc"),
            profile_dir: profile_dir.path().display().to_string(),
            task_id,
            mode: BrowserAdapterMode::ChromiumCdp,
            state: BrowserProcessState::Running,
            created_at: TimestampMillis::now(),
            internal_sandbox_degraded,
            graceful_shutdown: false,
        };
        self.real_processes.insert(
            process.id.clone(),
            RealBrowserProcess {
                child,
                _profile_dir: profile_dir,
                port,
                browser_ws_path,
            },
        );
        self.processes.insert(process.id.clone(), process.clone());
        Ok(process)
    }

    /// Spawn Chrome and wait for its DevTools endpoint.  Chrome's internal
    /// sandbox requires OS support (macOS seatbelt); on hosts where that is
    /// unavailable Chrome accepts the launch but its renderer/DevTools session
    /// dies on first attach.  This is detected with a real browser-endpoint
    /// probe, and only then is Chrome relaunched with `--no-sandbox`.
    /// AgentCode's own process-restricted boundary always still applies; the
    /// degradation is recorded honestly on the process record.
    fn launch_chrome_with_degraded_retry(
        &mut self,
        executable: &Path,
    ) -> AcResult<(Child, TempDir, u16, String, bool)> {
        let base_args = [
            "--headless=new",
            "--remote-debugging-port=0",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-background-networking",
            "--disable-sync",
            "--disable-extensions",
            "--disable-popup-blocking",
        ];
        let attempt = |extra_args: &[&str]| -> AcResult<(Child, TempDir, u16, String)> {
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
            let mut browser_args: Vec<String> =
                base_args.iter().map(|arg| arg.to_string()).collect();
            browser_args.extend(extra_args.iter().map(|arg| arg.to_string()));
            browser_args.push(format!("--user-data-dir={}", profile_dir.path().display()));
            browser_args.push("about:blank".to_string());
            let plan = prepare_browser_spawn(executable, &browser_args, profile_dir.path())?;
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
            let (port, browser_ws_path) =
                match wait_for_devtools_port(profile_dir.path(), &mut child) {
                    Ok(found) => found,
                    Err(err) => {
                        terminate_untracked_chrome(&mut child);
                        return Err(err);
                    }
                };
            Ok((child, profile_dir, port, browser_ws_path))
        };

        let (mut child, profile_dir, port, browser_ws_path) = attempt(&[])?;
        if probe_chrome_browser_endpoint(port, &browser_ws_path).is_ok() {
            return Ok((child, profile_dir, port, browser_ws_path, false));
        }
        // The host cannot support Chrome's internal sandbox profile (its
        // DevTools session dies when a page session attaches).  Relaunch once
        // with Chrome's internal sandbox disabled; AgentCode's own
        // process-restricted sandbox boundary still wraps the browser.
        terminate_untracked_chrome(&mut child);
        let (child, profile_dir, port, browser_ws_path) = attempt(&["--no-sandbox"])?;
        probe_chrome_browser_endpoint(port, &browser_ws_path).map_err(|_| {
            AcError::new(
                "BROWSER-LAUNCH_FAILED",
                "Chrome DevTools endpoint did not stay reachable even with its internal sandbox disabled",
                ac_common::ErrorKind::Unavailable,
                ac_common::Retryability::NotRetryable,
            )
        })?;
        Ok((child, profile_dir, port, browser_ws_path, true))
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
            .clone();
        let _ = process_id;
        // Pages are SESSION-keyed (one CDP target per tab).
        self.real_pages.get_mut(session_id).ok_or_else(|| {
            AcError::validation("BROWSER-PAGE_UNKNOWN", "real browser page is not open")
        })
    }

    fn act_chromium_cdp(
        &mut self,
        session_id: &StableId,
        action: BrowserAction,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<BrowserActionResult> {
        let mut evaluate_value: Option<serde_json::Value> = None;
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
                BrowserAction::ClickAt { x, y } => {
                    page.click_at(*x, *y)?;
                    "click_at"
                }
                BrowserAction::PressKey { key } => {
                    page.press_key(key)?;
                    "press_key"
                }
                BrowserAction::ScrollBy { dx, dy, x, y } => {
                    page.scroll_by(*dx, *dy, *x, *y)?;
                    "scroll_by"
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
                BrowserAction::Evaluate { script } => {
                    // Runtime.evaluate; the returned JSON value travels on
                    // the action result (diagnostics + design QA metrics).
                    evaluate_value = Some(page.client.evaluate_value(script)?);
                    "evaluate"
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
            value: evaluate_value,
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
        page.client.drain_events(Duration::from_millis(500))?;
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

impl BrowserRuntime {
    /// The panel's persistent page session id, if the panel is open.
    pub fn panel_session(&self) -> Option<StableId> {
        self.panel_session.clone()
    }

    /// Remember (or clear) the panel's persistent page session.
    pub fn set_panel_session(&mut self, session_id: &StableId) {
        self.panel_session = Some(session_id.clone());
    }

    /// The process that owns the panel's persistent page session (for
    /// opening additional tabs in the SAME browser).
    pub fn panel_process(&self) -> Option<StableId> {
        let session_id = self.panel_session.as_ref()?;
        self.sessions.get(session_id).map(|s| s.process_id.clone())
    }

    /// The task that owns the panel's persistent page session — a new tab
    /// must reuse BOTH the process and its owning task (sessions are
    /// task-owned; a fresh random task id would be rejected).
    pub fn panel_task(&self) -> Option<StableId> {
        let session_id = self.panel_session.as_ref()?;
        self.sessions.get(session_id).map(|s| s.task_id.clone())
    }

    /// Whether a tab's session is still LIVE in this runtime (registered
    /// AND owned by a running process) — tab-registry hygiene after
    /// teardowns and restarts.
    pub fn has_live_session(&self, session_id: &StableId) -> bool {
        let Some(session) = self.sessions.get(session_id) else {
            return false;
        };
        let Some(process) = self.processes.get(&session.process_id) else {
            return false;
        };
        matches!(
            process.state,
            BrowserProcessState::Running | BrowserProcessState::Ready
        )
    }

    /// Drop one tab (page session): removes the session record and its
    /// page socket.  The headless Chrome target itself is reclaimed at
    /// teardown.
    pub fn close_session(&mut self, session_id: &StableId) {
        self.real_pages.remove(session_id);
        self.sessions.remove(session_id);
    }

    /// Gracefully close every live process (panel shutdown + daemon stop).
    pub fn close_all(&mut self) {
        let ids: Vec<StableId> = self.processes.keys().cloned().collect();
        for id in ids {
            let _ = self.close_process(&id);
        }
        self.panel_session = None;
    }

    /// CDP history traversal for the inbuilt browser panel ("back"/
    /// "forward").  Uses Page.getNavigationHistory + Page.navigateToHistoryEntry.
    pub fn traverse_history(&mut self, session_id: &StableId, direction: &str) -> AcResult<String> {
        if self.mode != BrowserAdapterMode::ChromiumCdp {
            return Err(AcError::validation(
                "BROWSER-HISTORY_UNSUPPORTED",
                "history traversal requires the real Chrome runtime",
            ));
        }
        let page = self.real_page_mut(session_id)?;
        let value = page
            .client
            .call("Page.getNavigationHistory", json!({}))
            .map_err(|err| AcError::validation("BROWSER-HISTORY_READ", err.to_string()))?;
        let index = value
            .get("currentIndex")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let total = value
            .get("entries")
            .and_then(|e| e.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let target = if direction == "back" {
            index - 1
        } else {
            index + 1
        };
        if target < 0 || target as usize >= total {
            return Err(AcError::validation(
                "BROWSER-HISTORY_AT_EDGE",
                format!("cannot go {direction}: no further history entry"),
            ));
        }
        let entry_id = value
            .get("entries")
            .and_then(|e| e.as_array())
            .and_then(|entries| entries.get(target as usize))
            .and_then(|entry| entry.get("id"))
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AcError::validation("BROWSER-HISTORY_ENTRY", "history entry id missing")
            })?;
        let url = page
            .client
            .call(
                "Page.navigateToHistoryEntry",
                json!({ "entryId": entry_id }),
            )
            .map_err(|err| AcError::validation("BROWSER-HISTORY_NAVIGATE", err.to_string()))?;
        let final_url = url
            .get("frame")
            .and_then(|f| f.get("url"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        Ok(final_url)
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
        for (process_id, mut process) in std::mem::take(&mut self.real_processes) {
            // Batch N2: try a graceful CDP `Browser.close` handshake first
            // (SIGKILL-only teardown is what makes Chrome show the user its
            // "quit unexpectedly" crash dialogs).  Fall back to SIGTERM,
            // then SIGKILL, and only delete the temp profile dir after the
            // process has fully exited.
            let graceful = terminate_browser_process_gracefully(&mut process);
            if let Some(record) = self.processes.get_mut(&process_id) {
                record.graceful_shutdown = graceful;
            }
        }
        for process in self.processes.values_mut() {
            if process.state == BrowserProcessState::Closing {
                process.state = BrowserProcessState::Closed;
            }
        }
    }
}

/// Graceful browser teardown (batch N2).  Order:
/// 1. CDP `Browser.close` on the browser endpoint (Chrome flushes its
///    profile and exits 0 — the "unexpected quit" crash markers never get
///    written),
/// 2. wait up to `GRACEFUL_CLOSE_DEADLINE` for the process to exit,
/// 3. SIGTERM fallback, short wait,
/// 4. SIGKILL last resort (still the honest forced path).
///
/// The temp profile dir is only dropped by the `RealBrowserProcess`'s own
/// `TempDir` destructor AFTER the child has fully exited.
///
/// Returns true when the shutdown was graceful (Browser.close or an
/// already-exited process), false when signals were required.
fn terminate_browser_process_gracefully(process: &mut RealBrowserProcess) -> bool {
    // Already gone: nothing to be graceful about, but also nothing to kill.
    if let Ok(Some(_)) = process.child.try_wait() {
        return true;
    }
    // 1) CDP Browser.close on the browser-level websocket.
    let ws_url = format!("ws://127.0.0.1:{}{}", process.port, process.browser_ws_path);
    if process.browser_ws_path.starts_with('/')
        && CdpClient::connect(&ws_url)
            .and_then(|mut client| client.call("Browser.close", json!({})))
            .is_ok()
        && wait_child_exit(&mut process.child, Duration::from_secs(5))
    {
        return true;
    }
    // 2) SIGTERM fallback via the system `kill` utility (workspace forbids
    //    `unsafe`, so libc::kill is not available; spawning /bin/kill is the
    //    safe equivalent), then a short wait for exit.
    #[cfg(unix)]
    {
        let pid = process.child.id().to_string();
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    if wait_child_exit(&mut process.child, Duration::from_secs(3)) {
        return true;
    }
    // 3) SIGKILL last resort (forced path — honest, but not graceful).
    let _ = process.child.kill();
    let _ = process.child.wait();
    false
}

/// Tear down a Chrome child that is not yet registered in the runtime
/// (launch-failure cleanup).  Chrome-crash-dialog fix: SIGKILL'ing Chrome is
/// exactly what makes macOS show its "Chrome quit unexpectedly" dialogs, so
/// send SIGTERM first and wait for a real exit; SIGKILL is the last resort
/// only when the process refuses to die.
fn terminate_untracked_chrome(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id().to_string();
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    if !wait_child_exit(child, Duration::from_secs(3)) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Poll the child for full exit up to `deadline`.  Returns true when it
/// exited on its own within the budget.
fn wait_child_exit(child: &mut Child, deadline_budget: Duration) -> bool {
    let deadline = Instant::now() + deadline_budget;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => return false,
        }
    }
    false
}

impl RealBrowserPage {
    fn create(port: u16, browser_ws_path: &str) -> AcResult<Self> {
        // Chrome 111+ deprecated the HTTP PUT /json/new flow: the devtools
        // HTTP listener dies after serving it, and page-endpoint websocket
        // handshakes return 404.  When DevToolsActivePort publishes a browser
        // endpoint path (all modern Chrome does), connect there and drive the
        // page through Target.createTarget + a flat session.  Only fall back
        // to the legacy page-websocket flow on old Chrome versions that
        // neither publish the browser path nor support the flat protocol.
        let browser_ws_url = if browser_ws_path.starts_with('/') {
            Some(format!("ws://127.0.0.1:{port}{browser_ws_path}"))
        } else if browser_ws_path.starts_with("ws") {
            Some(browser_ws_path.to_string())
        } else {
            None
        };
        if let Some(browser_ws_url) = browser_ws_url {
            let mut client = CdpClient::connect(&browser_ws_url)?;
            let target = client.call(
                "Target.createTarget",
                json!({ "url": "about:blank", "background": false }),
            )?;
            let target_id = target
                .get("targetId")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AcError::validation(
                        "BROWSER-CDP_TARGET",
                        "Chrome did not return a targetId for the new page",
                    )
                })?
                .to_string();
            let session = client.call(
                "Target.attachToTarget",
                json!({ "targetId": target_id, "flatten": true }),
            )?;
            let session_id = session
                .get("sessionId")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AcError::validation(
                        "BROWSER-CDP_TARGET",
                        "Chrome did not return a sessionId for the attached page",
                    )
                })?
                .to_string();
            client.session_id = Some(session_id);
            Self::enable_page_domains(&mut client)?;
            return Ok(Self {
                client,
                url: "about:blank".to_string(),
                viewport: default_viewports()[2],
            });
        }
        // Legacy path: old Chrome served page websockets directly.
        let mut client = Self::connect_page_ws(port)?;
        Self::enable_page_domains(&mut client)?;
        Ok(Self {
            client,
            url: "about:blank".to_string(),
            viewport: default_viewports()[2],
        })
    }

    fn connect_page_ws(port: u16) -> AcResult<CdpClient> {
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
            })?
            .to_string();
        CdpClient::connect(&ws_url)
    }

    fn enable_page_domains(client: &mut CdpClient) -> AcResult<()> {
        for method in [
            "Page.enable",
            "Runtime.enable",
            "DOM.enable",
            "Network.enable",
            "Accessibility.enable",
        ] {
            client.call(method, json!({}))?;
        }
        Ok(())
    }

    fn navigate(&mut self, url: &str) -> AcResult<()> {
        self.client.reset_navigation_watch();
        self.client
            .call("Page.navigate", json!({ "url": url.to_string() }))?;
        self.client.wait_for_load(Some(url))?;
        self.url = self
            .client
            .current_url_hint
            .clone()
            .unwrap_or_else(|| url.to_string());
        Ok(())
    }

    /// Click raw page coordinates via CDP Input.dispatchMouseEvent (real
    /// trusted event — the inbuilt browser's interactive surface).
    fn click_at(&mut self, x: i32, y: i32) -> AcResult<()> {
        let params = json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1,
        });
        self.client.call("Input.dispatchMouseEvent", params)?;
        let params = json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1,
        });
        self.client.call("Input.dispatchMouseEvent", params)?;
        self.client.drain_events(Duration::from_millis(250))?;
        self.url = self.current_url()?;
        Ok(())
    }

    /// Send a key event via CDP Input.dispatchKeyEvent.  `key` is a literal
    /// like "a", "Enter", "ArrowLeft", "Backspace".
    fn press_key(&mut self, key: &str) -> AcResult<()> {
        let text = if key.chars().count() == 1 {
            key.to_string()
        } else {
            String::new()
        };
        let params = json!({
            "type": if text.is_empty() { "keyDown" } else { "char" },
            "key": key,
            "text": text,
            "unmodifiedText": text,
            "nativeVirtualKeyCode": 0,
        });
        self.client.call("Input.dispatchKeyEvent", params)?;
        self.client.drain_events(Duration::from_millis(150))?;
        Ok(())
    }

    /// Wheel scroll via CDP Input.dispatchMouseEvent at viewport coords.
    fn scroll_by(&mut self, dx: i32, dy: i32, x: i32, y: i32) -> AcResult<()> {
        let params = json!({
            "type": "mouseWheel",
            "x": x,
            "y": y,
            "deltaX": dx,
            "deltaY": dy,
        });
        self.client.call("Input.dispatchMouseEvent", params)?;
        self.client.drain_events(Duration::from_millis(150))?;
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
        // JPEG q80: 5-8x smaller than PNG for real pages — the screenshot
        // crosses the daemon->UI IPC boundary on every browser action, so
        // payload size is the single biggest latency factor.
        let value = self.client.call(
            "Page.captureScreenshot",
            json!({ "format": "jpeg", "quality": 80, "fromSurface": true }),
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
        // One stable path per panel page — each capture overwrites the
        // previous shot instead of accumulating temp files.
        let path = dir.join("panel.jpg");
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
            session_id: None,
            console_errors: Vec::new(),
            page_errors: Vec::new(),
            network_failures: Vec::new(),
            http_status: 0,
            current_url_hint: None,
            document_ready_seen: false,
        })
    }

    fn call(&mut self, method: &str, params: Value) -> AcResult<Value> {
        self.call_until(method, params, Instant::now() + Duration::from_secs(8))
    }

    fn call_until(&mut self, method: &str, params: Value, deadline: Instant) -> AcResult<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let mut payload = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(session_id) = &self.session_id {
            payload["sessionId"] = json!(session_id);
        }
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .map_err(|err| AcError::validation("BROWSER-CDP_SEND", err.to_string()))?;
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
            ac_common::Retryability::Retryable,
        ))
    }

    fn reset_navigation_watch(&mut self) {
        self.document_ready_seen = false;
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
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            self.drain_events(remaining.min(Duration::from_millis(75)))?;
            if self.document_ready_seen
                && expected_url.is_none_or(|expected| {
                    self.current_url_hint
                        .as_deref()
                        .is_some_and(|actual| actual == expected || actual.starts_with(expected))
                })
            {
                return Ok(());
            }
            let poll_deadline = Instant::now() + remaining.min(Duration::from_millis(500));
            let ready_value = match self.call_until(
                "Runtime.evaluate",
                json!({
                    "expression": "(() => document.readyState)()",
                    "returnByValue": true
                }),
                poll_deadline,
            ) {
                Ok(value) => value,
                Err(error) if error.code() == "BROWSER-CDP_TIMEOUT" => continue,
                Err(error) => return Err(error),
            };
            let ready = ready_value
                .get("result")
                .and_then(|result| result.get("value"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let url_matches = if let Some(expected_url) = expected_url {
                let url_deadline = Instant::now()
                    + deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(500));
                let value = match self.call_until(
                    "Runtime.evaluate",
                    json!({
                        "expression": "(() => location.href)()",
                        "returnByValue": true
                    }),
                    url_deadline,
                ) {
                    Ok(value) => value,
                    Err(error) if error.code() == "BROWSER-CDP_TIMEOUT" => continue,
                    Err(error) => return Err(error),
                };
                let href = value
                    .get("result")
                    .and_then(|result| result.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                href == expected_url || href.starts_with(expected_url)
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
            ac_common::Retryability::Retryable,
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
            "Page.frameNavigated" => {
                if let Some(url) = params
                    .get("frame")
                    .and_then(|frame| frame.get("url"))
                    .and_then(Value::as_str)
                {
                    self.current_url_hint = Some(url.to_string());
                }
            }
            "Page.loadEventFired" | "Page.domContentEventFired" => {
                self.document_ready_seen = true;
            }
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

/// Discover a usable Chromium executable (real-browser QA capability
/// probe).  None means real-browser QA cannot run in this environment.
pub fn discover_chromium_executable() -> Option<PathBuf> {
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
        allow_degraded_execution: false,
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

/// Verify that Chrome's browser DevTools endpoint accepts a websocket and
/// survives a full create-target + attach + page-command round trip.  On
/// hosts whose OS cannot provide the seatbelt profile Chrome's internal
/// sandbox needs, the first page-scoped command tears the connection down;
/// this probe makes that failure observable before the process record is
/// published, so the caller can relaunch with honest degradation evidence.
fn probe_chrome_browser_endpoint(port: u16, browser_ws_path: &str) -> AcResult<()> {
    if !browser_ws_path.starts_with('/') {
        return Ok(());
    }
    let ws_url = format!("ws://127.0.0.1:{port}{browser_ws_path}");
    let mut client = CdpClient::connect(&ws_url)?;
    let target = client.call(
        "Target.createTarget",
        json!({ "url": "about:blank", "background": false }),
    )?;
    let target_id = target
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AcError::validation("BROWSER-CDP_TARGET", "Chrome did not return a targetId")
        })?
        .to_string();
    let session = client.call(
        "Target.attachToTarget",
        json!({ "targetId": target_id, "flatten": true }),
    )?;
    let session_id = session
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AcError::validation("BROWSER-CDP_TARGET", "Chrome did not return a sessionId")
        })?
        .to_string();
    client.session_id = Some(session_id.clone());
    client.call("Page.enable", json!({}))?;
    client
        .call("Runtime.evaluate", json!({ "expression": "1+1" }))?
        .get("result")
        .ok_or_else(|| {
            AcError::validation("BROWSER-CDP_TARGET", "probe evaluation returned no result")
        })?;
    client.session_id = None;
    let _ = client.call("Target.closeTarget", json!({ "targetId": target_id }));
    Ok(())
}

fn wait_for_devtools_port(profile_dir: &Path, child: &mut Child) -> AcResult<(u16, String)> {
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
            let mut lines = contents.lines();
            if let Some(port) = lines.next().and_then(|line| line.parse().ok()) {
                // Second line carries the browser websocket path token.  Modern
                // Chrome (111+) only accepts websocket handshakes on endpoints
                // listed here; direct /devtools/page/<id> handshakes return 404.
                let browser_path = lines
                    .next()
                    .map(|line| line.trim().to_string())
                    .unwrap_or_default();
                if !browser_path.is_empty() && browser_path.starts_with('/') {
                    return Ok((port, browser_path));
                }
                return Ok((port, String::new()));
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

/// Extract a double- or single-quoted attribute value from an HTML snippet.
/// Returns None when the attribute is absent or malformed.
fn extract_attribute(snippet: &str, attribute: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let pattern = format!("{attribute}={quote}");
        if let Some(start) = snippet.find(&pattern) {
            let value_start = start + pattern.len();
            let rest = &snippet[value_start..];
            let value_end = rest.find(quote)?;
            let value = &rest[..value_end];
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
    }
    None
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
                    required_requirement_ids: vec![StableId::new("req-a"), StableId::new("req-b")],
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

        let complete_requirement = StableId::new("req");
        let complete = engine
            .final_audit(
                FinalAuditInput {
                    original_goal: "implement auth".to_string(),
                    requirements: vec!["route wired".to_string()],
                    required_requirement_ids: vec![complete_requirement.clone()],
                    verified_requirement_ids: vec![complete_requirement],
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
        let optional_c = StableId::from_existing("criterion-c").unwrap();
        let only_a = engine
            .final_audit(
                FinalAuditInput {
                    original_goal: "ship covered requirements".to_string(),
                    requirements: vec!["A".to_string(), "B".to_string()],
                    required_requirement_ids: vec![required_a.clone(), required_b.clone()],
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
                    required_requirement_ids: vec![required_a.clone(), required_b.clone()],
                    verified_requirement_ids: vec![required_a, required_b, optional_c],
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
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
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
        // Wait for the CDP Network.responseReceived (404) event to be
        // dispatched.  Under full-suite contention the event can arrive late,
        // so poll with a deadline instead of relying on a single fixed sleep.
        // The assertion below is unchanged and remains strict.
        // F6 (final audit): under full-suite parallelism Chrome's CDP
        // Network.responseReceived event can arrive well past 10s on a
        // contended 8GB machine; 30s removes the flake while keeping the
        // assertion strict (poll, not sleep).
        let wait_deadline = Instant::now() + Duration::from_secs(30);
        let mut diagnostics = runtime.diagnostics(&session.id, &mut evidence).unwrap();
        while !diagnostics
            .network_failures
            .iter()
            .any(|failure| failure.contains("missing.html") || failure.contains("net::ERR"))
            && Instant::now() < wait_deadline
        {
            std::thread::sleep(Duration::from_millis(100));
            diagnostics = runtime.diagnostics(&session.id, &mut evidence).unwrap();
        }
        assert!(diagnostics
            .network_failures
            .iter()
            .any(|failure| { failure.contains("missing.html") || failure.contains("net::ERR") }));
        runtime.mark_crashed(&process.id).unwrap();
    }

    /// Batch N2: the NORMAL end-of-work browser close must be graceful —
    /// CDP `Browser.close` handshake, Chrome exits on its own (exit 0),
    /// the record marks `graceful_shutdown: true`, and the state machine
    /// lands on Closed.  A SIGKILL-only teardown is what produces the
    /// user-visible "Chrome quit unexpectedly" crash dialogs; this test
    /// proves the production close path no longer does that.
    #[test]
    fn batch_n2_graceful_browser_close_via_cdp() {
        if discover_chromium_executable().is_none() {
            // Honest environment gate, same pattern as the other
            // real-Chrome tests: without Chrome installed the graceful
            // path cannot be exercised — nothing is faked.
            return;
        }
        let mut runtime =
            BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
        let mut evidence = EvidenceStore::new();
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime
            .create_session(task.clone(), process.id.clone())
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Navigate {
                    url: "about:blank".to_string(),
                },
                &mut evidence,
            )
            .unwrap();

        let closed = runtime.close_process(&process.id).unwrap();
        assert_eq!(closed.state, BrowserProcessState::Closed);
        assert!(
            closed.graceful_shutdown,
            "normal close must be graceful (CDP Browser.close), got forced kill"
        );

        // Closing again is a validation error, not a crash.
        let err = runtime.close_process(&process.id).unwrap_err();
        assert_eq!(err.code(), "BROWSER-PROCESS_NOT_RUNNING");

        // Unknown process id stays an honest error.
        let err = runtime.close_process(&StableId::new("nope")).unwrap_err();
        assert_eq!(err.code(), "BROWSER-PROCESS_UNKNOWN");

        // Drop with no live processes must not hang or panic.
        drop(runtime);
    }

    /// design_metrics: deterministic harness must report honest
    /// NEEDS_MANUAL_REVIEW (no layout engine ⇒ no fabricated verdicts).
    #[test]
    fn design_metrics_deterministic_reports_needs_manual_review() {
        let mut runtime = BrowserRuntime::deterministic_harness_for_tests(
            CapabilityPolicy::new().allow(Capability::BrowserAutomation),
        );
        let mut evidence = EvidenceStore::new();
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime
            .create_session(task.clone(), process.id.clone())
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::OpenHtmlForTest {
                    url: "http://127.0.0.1:1/fixture".to_string(),
                    html: "<!doctype html><html lang=\"en\"><head><meta name=\"viewport\" \
                           content=\"width=device-width\"></head><body><button>Go</button>\
                           </body></html>"
                        .to_string(),
                },
                &mut evidence,
            )
            .unwrap();
        let metrics = runtime.design_metrics(&session.id).unwrap();
        assert_eq!(metrics.mode, "deterministic");
        assert!(
            metrics.needs_manual_review,
            "deterministic mode must flag manual review"
        );
        assert_eq!(
            metrics.horizontal_overflow, None,
            "layout metrics must be None, not fabricated"
        );
        assert!(
            metrics.has_viewport_meta,
            "structural fact from fixture HTML"
        );
        assert!(metrics.has_lang_attribute);
    }

    /// design_metrics: real Chrome CDP must measure real layout — overflow,
    /// touch-target sizes, unlabeled controls, focus visibility, lang.
    #[test]
    fn design_metrics_real_chrome_measures_layout_and_touch_targets() {
        if discover_chromium_executable().is_none() {
            eprintln!("SKIP: no Chromium executable available");
            return;
        }
        let Ok(listener) = std::net::TcpListener::bind("127.0.0.1:0") else {
            return;
        };
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while std::time::Instant::now() < deadline {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0_u8; 2048];
                    let n = stream.read(&mut buffer).unwrap_or(0);
                    let _request = String::from_utf8_lossy(&buffer[..n]);
                    // Tiny target (16px) + unlabeled control + wide element
                    // (overflow) + skipped heading + no lang.
                    let body = "<!doctype html><html><head><style>\
                        #wide { width: 3000px; height: 10px; }\
                        #tiny { width: 16px; height: 16px; }</style></head>\
                        <body><h1>Top</h1><h4>Skip</h4>\
                        <div id=\"wide\"></div>\
                        <button id=\"tiny\" aria-label=\"Tiny\">x</button>\
                        <button id=\"plain\">Labelled</button>\
                        <input id=\"nolabel\" type=\"text\" />\
                        </body></html>";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            }
        });

        let mut runtime =
            BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
        let mut evidence = EvidenceStore::new();
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime
            .create_session(task.clone(), process.id.clone())
            .unwrap();
        // Apply a viewport like design_qa_run does, then measure.
        runtime
            .set_viewport(
                &session.id,
                ViewportProfile {
                    name: "desktop",
                    width: 1440,
                    height: 900,
                },
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Navigate {
                    url: format!("http://{addr}/metrics"),
                },
                &mut evidence,
            )
            .unwrap();
        runtime
            .act(
                &session.id,
                BrowserAction::Wait { millis: 300 },
                &mut evidence,
            )
            .unwrap();
        let metrics = runtime.design_metrics(&session.id).unwrap();
        assert_eq!(metrics.mode, "cdp");
        assert!(
            !metrics.needs_manual_review,
            "CDP mode measures layout directly"
        );
        // Real overflow: the 3000px element must be detected.
        assert_eq!(
            metrics.horizontal_overflow,
            Some(true),
            "scroll/client: {:?}/{:?}",
            metrics.scroll_width,
            metrics.client_width
        );
        assert!(
            metrics.scroll_width.unwrap_or(0) > 2000,
            "scrollWidth must reflect the 3000px element"
        );
        // Tiny touch target below 24px must be flagged with real size.
        assert!(
            metrics
                .small_touch_targets
                .iter()
                .any(|t| t.contains("16x16px")),
            "small targets: {:?}",
            metrics.small_touch_targets
        );
        // Unlabeled input must be flagged.
        assert!(
            metrics
                .unlabeled_controls
                .iter()
                .any(|c| c.contains("input")),
            "unlabeled: {:?}",
            metrics.unlabeled_controls
        );
        // Heading skip h1→h4 must be detected.
        assert!(
            metrics.heading_skips.iter().any(|h| h.contains("h1 to h4")),
            "heading skips: {:?}",
            metrics.heading_skips
        );
        // No lang attribute in this fixture.
        assert!(!metrics.has_lang_attribute);
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
