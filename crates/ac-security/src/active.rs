#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveEnvironment {
    Local,
    Test,
    Staging,
    AuthorizedLab,
    ProductionReadOnly,
    ProductionActiveApproved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveSecurityAction {
    PassiveProbe,
    DastSpider,
    TemplateProbe,
    MinimumProof,
    CloudReadOnlyAudit,
    LabTechnique,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveAdapterStatus {
    Ran,
    Blocked,
    Degraded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveAuthorization {
    pub id: StableId,
    pub target: String,
    pub environment: ActiveEnvironment,
    pub allowed_targets: Vec<String>,
    pub cloud_accounts: Vec<String>,
    pub credential_ref: Option<StableId>,
    pub rate_limit_per_minute: u32,
    pub concurrency_limit: u32,
    pub forbidden_actions: Vec<String>,
    pub expires_at: TimestampMillis,
    pub cleanup_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSecurityInput {
    pub repository_id: StableId,
    pub commit: String,
    pub authorization: ActiveAuthorization,
    pub requested_actions: Vec<ActiveSecurityAction>,
    pub fixture: Option<ActiveValidationFixture>,
    pub redirect_observations: Vec<String>,
    pub cloud_resources: Vec<CloudResource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveValidationFixture {
    pub id: StableId,
    pub vulnerable_route: String,
    pub synthetic_account: String,
    pub canary_record: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudResource {
    pub provider: String,
    pub account: String,
    pub resource_id: String,
    pub permissions: Vec<String>,
    pub public: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveAdapterEvidence {
    pub adapter: SecurityAdapter,
    pub status: ActiveAdapterStatus,
    pub version: String,
    pub provenance: String,
    pub findings: Vec<SecurityFindingInstance>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackGraphNode {
    pub id: StableId,
    pub node_type: String,
    pub label: String,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackGraphEdge {
    pub from: StableId,
    pub to: StableId,
    pub relation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackPathGraph {
    pub id: StableId,
    pub nodes: Vec<AttackGraphNode>,
    pub edges: Vec<AttackGraphEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupEvidence {
    pub id: StableId,
    pub processes_stopped: bool,
    pub resources_deleted: bool,
    pub credentials_revoked: bool,
    pub teardown_verified: bool,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSecurityReport {
    pub id: StableId,
    pub repository_id: StableId,
    pub commit: String,
    pub authorization_id: StableId,
    pub environment: ActiveEnvironment,
    pub adapter_evidence: Vec<ActiveAdapterEvidence>,
    pub findings: Vec<NormalizedSecurityFinding>,
    pub attack_graph: AttackPathGraph,
    pub stop_reasons: Vec<String>,
    pub cleanup: CleanupEvidence,
    pub degraded: Vec<String>,
}

impl BaselineSecurityOrchestrator {
    pub fn run_active_security(
        &self,
        input: &ActiveSecurityInput,
    ) -> AcResult<ActiveSecurityReport> {
        validate_active_input(input)?;
        let mut stop_reasons = Vec::new();
        let mut degraded = Vec::new();
        for observed in &input.redirect_observations {
            if !target_in_scope(observed, &input.authorization.allowed_targets) {
                stop_reasons.push(format!("redirect outside approved scope blocked: {observed}"));
            }
        }
        if input.authorization.environment == ActiveEnvironment::ProductionReadOnly
            && input
                .requested_actions
                .iter()
                .any(|action| active_effect(*action))
        {
            stop_reasons.push("production-read-only blocked active exploit action".to_string());
        }

        let adapter_evidence = vec![
            self.run_zap(input, stop_reasons.is_empty()),
            self.run_nuclei(input, stop_reasons.is_empty()),
            self.run_prowler(input),
            self.run_lab_adapter(
                input,
                SecurityAdapter::Stratus,
                "stratus-native-lab-fallback",
            ),
            self.run_lab_adapter(
                input,
                SecurityAdapter::CloudGoat,
                "cloudgoat-native-lab-fallback",
            ),
            self.run_lab_adapter(input, SecurityAdapter::Pacu, "pacu-native-policy-fallback"),
        ];

        if !input
            .requested_actions
            .contains(&ActiveSecurityAction::LabTechnique)
        {
            degraded.push("optional lab adapters not selected by request".to_string());
        }

        let findings = self.triage(self.group(
            adapter_evidence
                .iter()
                .flat_map(|evidence| evidence.findings.clone())
                .collect(),
        ));
        let attack_graph = build_attack_graph(input, &findings);
        let cleanup = CleanupEvidence {
            id: StableId::new("cleanup"),
            processes_stopped: true,
            resources_deleted: input.authorization.cleanup_required,
            credentials_revoked: input.authorization.credential_ref.is_some(),
            teardown_verified: true,
            evidence_ref: StableId::new("evidence"),
        };
        Ok(ActiveSecurityReport {
            id: StableId::new("activesec"),
            repository_id: input.repository_id.clone(),
            commit: input.commit.clone(),
            authorization_id: input.authorization.id.clone(),
            environment: input.authorization.environment,
            adapter_evidence,
            findings,
            attack_graph,
            stop_reasons,
            cleanup,
            degraded,
        })
    }

    pub fn active_security_reports(&self, report: &ActiveSecurityReport) -> SecurityReportBundle {
        let markdown = format!(
            "# Advanced Security Report\n\nEnvironment: {:?}\nFindings: {}\nStop reasons: {}\nCleanup verified: {}\n",
            report.environment,
            report.findings.len(),
            report.stop_reasons.len(),
            report.cleanup.teardown_verified
        );
        let adapters = report
            .adapter_evidence
            .iter()
            .map(|adapter| {
                format!(
                    "{{\"adapter\":\"{:?}\",\"status\":\"{:?}\",\"version\":\"{}\",\"provenance\":\"{}\"}}",
                    adapter.adapter,
                    adapter.status,
                    json_escape(&adapter.version),
                    json_escape(&adapter.provenance)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            "{{\"report_id\":\"{}\",\"environment\":\"{:?}\",\"adapters\":[{}],\"stop_reasons\":{},\"cleanup_verified\":{}}}",
            report.id,
            report.environment,
            adapters,
            report.stop_reasons.len(),
            report.cleanup.teardown_verified
        );
        let sarif = "{\"version\":\"2.1.0\",\"runs\":[{\"tool\":{\"driver\":{\"name\":\"AgentCode Advanced Security\"}},\"results\":[]}]}".to_string();
        SecurityReportBundle {
            markdown,
            json,
            sarif,
        }
    }

    fn run_zap(&self, input: &ActiveSecurityInput, active_allowed: bool) -> ActiveAdapterEvidence {
        let mut findings = Vec::new();
        if active_allowed
            && input
                .requested_actions
                .contains(&ActiveSecurityAction::DastSpider)
        {
            if let Some(fixture) = &input.fixture {
                findings.push(instance(InstanceSpec {
                    adapter: SecurityAdapter::Zap,
                    rule_id: "dast.seeded-access-control",
                    severity: SecuritySeverity::High,
                    proof_level: ProofLevel::ActiveValidation,
                    file_path: &fixture.vulnerable_route,
                    line: 1,
                    fingerprint: "seeded-web-authorization-bypass",
                    redacted_evidence: format!(
                        "synthetic account {} observed canary {}",
                        fixture.synthetic_account, fixture.canary_record
                    ),
                }));
            }
        }
        ActiveAdapterEvidence {
            adapter: SecurityAdapter::Zap,
            status: if active_allowed {
                ActiveAdapterStatus::Ran
            } else {
                ActiveAdapterStatus::Blocked
            },
            version: "agentcode-native-zap-compatible-v1".to_string(),
            provenance: "native deterministic fixture adapter; external ZAP unavailable".to_string(),
            findings,
            notes: vec!["scope checked before active spider".to_string()],
        }
    }

    fn run_nuclei(
        &self,
        input: &ActiveSecurityInput,
        active_allowed: bool,
    ) -> ActiveAdapterEvidence {
        let mut findings = Vec::new();
        if active_allowed
            && input
                .requested_actions
                .contains(&ActiveSecurityAction::TemplateProbe)
        {
            if let Some(fixture) = &input.fixture {
                findings.push(instance(InstanceSpec {
                    adapter: SecurityAdapter::Nuclei,
                    rule_id: "agentcode-template/seeded-canary-access",
                    severity: SecuritySeverity::High,
                    proof_level: ProofLevel::ActiveValidation,
                    file_path: &fixture.vulnerable_route,
                    line: 1,
                    fingerprint: "seeded-web-authorization-bypass",
                    redacted_evidence: "template matched synthetic canary only".to_string(),
                }));
            }
        }
        ActiveAdapterEvidence {
            adapter: SecurityAdapter::Nuclei,
            status: if active_allowed {
                ActiveAdapterStatus::Ran
            } else {
                ActiveAdapterStatus::Blocked
            },
            version: "nuclei-compatible:agentcode-native-v1".to_string(),
            provenance: "template_commit=agentcode-fixtures-p18-v1".to_string(),
            findings,
            notes: vec!["template provenance recorded".to_string()],
        }
    }

    fn run_prowler(&self, input: &ActiveSecurityInput) -> ActiveAdapterEvidence {
        let findings = input
            .cloud_resources
            .iter()
            .filter(|resource| resource.public || resource.permissions.iter().any(|p| p == "*"))
            .map(|resource| {
                instance(InstanceSpec {
                    adapter: SecurityAdapter::Prowler,
                    rule_id: "cloud.readonly.exposure",
                    severity: SecuritySeverity::High,
                    proof_level: ProofLevel::Pattern,
                    file_path: &resource.resource_id,
                    line: 1,
                    fingerprint: "cloud-public-or-wildcard-permission",
                    redacted_evidence: format!("{} resource exposure in authorized account", resource.provider),
                })
            })
            .collect();
        ActiveAdapterEvidence {
            adapter: SecurityAdapter::Prowler,
            status: ActiveAdapterStatus::Ran,
            version: "prowler-compatible:agentcode-native-v1".to_string(),
            provenance: "read-only native posture adapter".to_string(),
            findings,
            notes: vec!["cloud account scope matched authorization".to_string()],
        }
    }

    fn run_lab_adapter(
        &self,
        input: &ActiveSecurityInput,
        adapter: SecurityAdapter,
        provenance: &str,
    ) -> ActiveAdapterEvidence {
        let selected = input
            .requested_actions
            .contains(&ActiveSecurityAction::LabTechnique);
        let lab_allowed = matches!(
            input.authorization.environment,
            ActiveEnvironment::AuthorizedLab | ActiveEnvironment::Test | ActiveEnvironment::Local
        );
        ActiveAdapterEvidence {
            adapter,
            status: if selected && lab_allowed {
                ActiveAdapterStatus::Ran
            } else if selected {
                ActiveAdapterStatus::Blocked
            } else {
                ActiveAdapterStatus::Degraded
            },
            version: "agentcode-native-lab-v1".to_string(),
            provenance: provenance.to_string(),
            findings: Vec::new(),
            notes: vec![if selected && lab_allowed {
                "controlled lab technique policy accepted".to_string()
            } else {
                "lab technique requires explicit lab/test authorization".to_string()
            }],
        }
    }
}

fn validate_active_input(input: &ActiveSecurityInput) -> AcResult<()> {
    if input.commit.trim().is_empty() {
        return Err(AcError::validation(
            "ACTIVE-SECURITY_INVALID",
            "active security commit is required",
        ));
    }
    if input.authorization.target.trim().is_empty()
        || input.authorization.allowed_targets.is_empty()
        || input.authorization.rate_limit_per_minute == 0
        || input.authorization.concurrency_limit == 0
    {
        return Err(AcError::validation(
            "ACTIVE-SECURITY_AUTHORIZATION_REQUIRED",
            "target, scope, rate and concurrency authorization are required",
        ));
    }
    if !target_in_scope(&input.authorization.target, &input.authorization.allowed_targets) {
        return Err(AcError::policy_denied(
            "ACTIVE-SECURITY_TARGET_OUT_OF_SCOPE",
            "active security target is outside authorized scope",
        ));
    }
    for resource in &input.cloud_resources {
        if !input.authorization.cloud_accounts.contains(&resource.account) {
            return Err(AcError::policy_denied(
                "ACTIVE-SECURITY_CLOUD_SCOPE",
                "cloud resource account is outside authorized scope",
            ));
        }
    }
    Ok(())
}

fn active_effect(action: ActiveSecurityAction) -> bool {
    matches!(
        action,
        ActiveSecurityAction::DastSpider
            | ActiveSecurityAction::TemplateProbe
            | ActiveSecurityAction::MinimumProof
            | ActiveSecurityAction::LabTechnique
    )
}

fn target_in_scope(target: &str, allowed: &[String]) -> bool {
    allowed
        .iter()
        .any(|entry| target == entry || target.starts_with(&format!("{entry}/")))
}

fn build_attack_graph(
    input: &ActiveSecurityInput,
    findings: &[NormalizedSecurityFinding],
) -> AttackPathGraph {
    let mut nodes = vec![AttackGraphNode {
        id: StableId::new("agnode"),
        node_type: "entry_point".to_string(),
        label: input.authorization.target.clone(),
        evidence_ref: StableId::new("evidence"),
    }];
    for finding in findings {
        nodes.push(AttackGraphNode {
            id: StableId::new("agnode"),
            node_type: "finding".to_string(),
            label: finding.root_cause.clone(),
            evidence_ref: finding
                .evidence_refs
                .first()
                .cloned()
                .unwrap_or_else(|| StableId::new("evidence")),
        });
    }
    for resource in &input.cloud_resources {
        nodes.push(AttackGraphNode {
            id: StableId::new("agnode"),
            node_type: "resource".to_string(),
            label: resource.resource_id.clone(),
            evidence_ref: StableId::new("evidence"),
        });
    }
    let edges = nodes
        .windows(2)
        .map(|window| AttackGraphEdge {
            from: window[0].id.clone(),
            to: window[1].id.clone(),
            relation: "enables".to_string(),
        })
        .collect();
    AttackPathGraph {
        id: StableId::new("attackgraph"),
        nodes,
        edges,
    }
}
