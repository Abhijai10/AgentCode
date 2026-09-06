use std::collections::BTreeSet;
use std::path::PathBuf;

use ac_common::StableId;
use ac_db::ControlPlaneDb;
use ac_evidence::EvidenceStore;
use ac_sandbox::{FilesystemRequest, SandboxManager, SandboxPolicy};
use ac_security::{
    ActiveAuthorization, ActiveEnvironment, ActiveSecurityAction, ActiveSecurityInput,
    ActiveValidationFixture, AiHarnessKind, AiSecurityInput, BaselineSecurityOrchestrator,
    Capability, CapabilityPolicy, McpRegistry, McpToolRecord, McpTransport, PermissionContext,
    RiskClass, SecurityDecision, SecurityPolicy, ToolRole,
};
use ac_tool::{register_mcp_tool_with_broker, ToolBroker, ToolRequest, ToolStatus};
use ac_verification::VerificationEngine;

#[test]
fn security_boundaries_reject_escape_denied_capability_and_unsafe_mcp() {
    let root = std::env::temp_dir().join(format!(
        "agentcode-security-boundary-{}",
        StableId::new("tmp")
    ));
    std::fs::create_dir_all(&root).unwrap();
    let sandbox = SandboxManager::new(SandboxPolicy::new(vec![root.clone()]));
    assert_eq!(
        sandbox.evaluate_filesystem(&FilesystemRequest {
            path: PathBuf::from("../outside"),
            write: true
        }),
        SecurityDecision::Deny
    );

    let mut evidence = EvidenceStore::new();
    let mut broker = ToolBroker::new(CapabilityPolicy::new());
    let server_id = StableId::new("mcpserver");
    let tool = McpToolRecord {
        id: StableId::new("mcptool"),
        server_id: server_id.clone(),
        name: "danger".to_string(),
        description: "unsafe operation".to_string(),
        schema: "{}".to_string(),
        risk: RiskClass::R4,
        required_capabilities: BTreeSet::from([Capability::Network("*".to_string())]),
    };
    let broker_tool_id = register_mcp_tool_with_broker(&mut broker, &tool).unwrap();
    let result = broker
        .invoke(
            ToolRequest {
                id: StableId::new("toolreq"),
                tool_id: broker_tool_id,
                tool_version: "1".to_string(),
                payload: "{}".to_string(),
                capabilities: vec![Capability::Network("*".to_string())],
            },
            &mut evidence,
        )
        .unwrap();
    assert_eq!(result.status, ToolStatus::Denied);

    let mut mcp = McpRegistry::new();
    let registered_server = mcp
        .register_server("untrusted", "1", McpTransport::Stdio)
        .unwrap();
    mcp.connect(&registered_server).unwrap();
    let mut mcp_tool = tool;
    mcp_tool.server_id = registered_server.clone();
    let registered_tool = mcp.register_tool(mcp_tool).unwrap();
    let decision = mcp
        .authorize_invocation(
            &registered_tool,
            &PermissionContext {
                role: ToolRole::Worker,
                mission: CapabilityPolicy::new().allow(Capability::Network("*".to_string())),
                task: CapabilityPolicy::new().allow(Capability::Network("*".to_string())),
                sandbox: CapabilityPolicy::new().allow(Capability::Network("*".to_string())),
                risk: RiskClass::R1,
                approval_granted: true,
            },
        )
        .unwrap();
    assert_eq!(decision, SecurityDecision::Deny);
    assert_eq!(mcp.discover_servers().len(), 1);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn advanced_and_ai_security_flow_through_evidence_and_persistence() {
    let orchestrator = BaselineSecurityOrchestrator::new(SecurityPolicy::baseline());
    let mut evidence = EvidenceStore::new();
    let verifier = VerificationEngine::new(CapabilityPolicy::new().allow(Capability::SecurityScan));
    let db_path = std::env::temp_dir().join(format!(
        "agentcode-sec-integration-{}.sqlite",
        StableId::new("db")
    ));

    let active_input = ActiveSecurityInput {
        repository_id: StableId::new("repo"),
        commit: "integration-p18".to_string(),
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
            expires_at: ac_common::TimestampMillis::from_millis(
                ac_common::TimestampMillis::now().as_millis() + 60_000,
            ),
            cleanup_required: true,
        },
        requested_actions: vec![ActiveSecurityAction::DastSpider],
        fixture: Some(ActiveValidationFixture {
            id: StableId::new("fixture"),
            vulnerable_route: "http://fixture.local/admin".to_string(),
            synthetic_account: "synthetic-user".to_string(),
            canary_record: "canary".to_string(),
        }),
        redirect_observations: Vec::new(),
        cloud_resources: Vec::new(),
    };
    let active_report = orchestrator.run_active_security(&active_input).unwrap();
    let active_evidence = verifier
        .record_active_security_report(&active_report, &mut evidence)
        .unwrap();
    assert_eq!(active_evidence.scanner, "advanced-security");

    let ai_report = orchestrator
        .run_ai_security(&AiSecurityInput {
            repository_id: StableId::new("repo"),
            commit: "integration-p19".to_string(),
            files: vec![(
                "src/agent.rs".to_string(),
                "openai user_prompt rag tool_call mcp agent SECRET=synthetic".to_string(),
            )],
            selected_harnesses: vec![AiHarnessKind::Promptfoo],
        })
        .unwrap();
    let ai_evidence = verifier
        .record_ai_security_report(&ai_report, &mut evidence)
        .unwrap();
    assert_eq!(ai_evidence.scanner, "ai-security");

    let mut db = ControlPlaneDb::open(&db_path).unwrap();
    db.migrate().unwrap();
    db.save_active_security_report(&active_input, &active_report)
        .unwrap();
    db.save_ai_security_report(&ai_report).unwrap();
    assert!(
        db.active_security_report(active_report.id.as_str())
            .unwrap()
            .unwrap()
            .cleanup_verified
    );
    assert!(!db
        .security_findings(ai_report.id.as_str())
        .unwrap()
        .is_empty());
    let _ = std::fs::remove_file(db_path);
}
