use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

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
    DeterministicHarness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserProcessState {
    Running,
    Crashed,
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
    Open { url: String, html: String },
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

    pub fn derive_profile(
        &self,
        task_id: StableId,
        risk: VerificationRisk,
        capabilities: &ProjectCapabilities,
    ) -> VerificationProfile {
        let mut layers = BTreeSet::new();
        layers.insert(VerificationLayer::Format);
        if capabilities.cargo || capabilities.package_json {
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
            mode: BrowserAdapterMode::DeterministicHarness,
            processes: BTreeMap::new(),
            sessions: BTreeMap::new(),
            pages: BTreeMap::new(),
        }
    }

    pub fn launch(&mut self, task_id: StableId) -> AcResult<BrowserProcessRecord> {
        self.ensure_browser_allowed()?;
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
        let session = self.sessions.get_mut(session_id).ok_or_else(|| {
            AcError::validation(
                "BROWSER-SESSION_UNKNOWN",
                "browser session is not registered",
            )
        })?;
        let page = self.pages.entry(session_id.clone()).or_insert(PageState {
            url: "about:blank".to_string(),
            html: String::new(),
            fields: BTreeMap::new(),
            clicked: Vec::new(),
            scroll_y: 0,
            viewport: default_viewports()[2],
        });
        let action_name = match &action {
            BrowserAction::Open { url, html } => {
                page.url = url.clone();
                page.html = html.clone();
                session.current_url = Some(url.clone());
                "open"
            }
            BrowserAction::Click { selector } => {
                require_selector(&page.html, selector)?;
                page.clicked.push(selector.clone());
                if selector.contains("submit") || selector.contains("button") {
                    page.url = route_after_submit(&page.url);
                    session.current_url = Some(page.url.clone());
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
        session.updated_at = TimestampMillis::now();
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "browser-runtime".to_string(),
                commit: None,
                worktree: None,
                tool: Some("browser-action".to_string()),
            },
            format!("mem://browser/action/{}", StableId::new("baction")),
            format!(
                "session:{};action:{};url:{}",
                session_id, action_name, page.url
            ),
        )?;
        Ok(BrowserActionResult {
            session_id: session_id.clone(),
            action: action_name.to_string(),
            ok: true,
            url: page.url.clone(),
            evidence_ref,
        })
    }

    pub fn inspect_dom(
        &self,
        session_id: &StableId,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DomSnapshot> {
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
        &self,
        session_id: &StableId,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<BrowserDiagnostics> {
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
        if screenshot.artifact_uri.contains("overlap")
            || screenshot.artifact_uri.contains("clipped")
        {
            findings.push(blocking(
                "BROWSER-VISUAL_DEFECT",
                "visual model fallback detected clipping or overlap",
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
            adapter: "deterministic-local-visual-fallback".to_string(),
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
        browser: root.join("playwright.config.ts").exists()
            || root.join("playwright.config.js").exists(),
        security: root.join("deny.toml").exists()
            || root.join(".cargo").join("audit.toml").exists(),
    }
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
        match layer {
            VerificationLayer::Format => (
                vec!["npm", "run", "format", "--", "--check"],
                "package.json",
            ),
            VerificationLayer::Lint => (vec!["npm", "run", "lint"], "package.json"),
            VerificationLayer::Typecheck => (vec!["npm", "run", "typecheck"], "package.json"),
            VerificationLayer::Unit | VerificationLayer::Integration => {
                (vec!["npm", "test"], "package.json")
            }
            VerificationLayer::Build => (vec!["npm", "run", "build"], "package.json"),
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
    fn phase15_browser_flow_actions_dom_diagnostics_screenshot_and_visual_qa_work() {
        let mut runtime =
            BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
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
                BrowserAction::Open {
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
        assert!(visual.passed);
        assert_eq!(visual.screenshot_ref, shot.evidence_ref);
    }

    #[test]
    fn phase15_browser_capability_denial_and_selector_failure_are_explicit() {
        let mut denied = BrowserRuntime::new(CapabilityPolicy::new());
        assert_eq!(
            denied.launch(StableId::new("task")).unwrap_err().code(),
            "BROWSER-CAPABILITY_DENIED"
        );
        let mut runtime =
            BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
        let task = StableId::new("task");
        let process = runtime.launch(task.clone()).unwrap();
        let session = runtime.create_session(task, process.id).unwrap();
        let mut evidence = EvidenceStore::new();
        runtime
            .act(
                &session.id,
                BrowserAction::Open {
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
        let mut runtime =
            BrowserRuntime::new(CapabilityPolicy::new().allow(Capability::BrowserAutomation));
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
                BrowserAction::Open {
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
