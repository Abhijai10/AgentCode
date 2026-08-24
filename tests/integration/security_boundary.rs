use std::collections::BTreeSet;
use std::path::PathBuf;

use ac_common::StableId;
use ac_evidence::EvidenceStore;
use ac_sandbox::{FilesystemRequest, SandboxManager, SandboxPolicy};
use ac_security::{
    Capability, CapabilityPolicy, McpRegistry, McpToolRecord, McpTransport, PermissionContext,
    RiskClass, SecurityDecision, ToolRole,
};
use ac_tool::{register_mcp_tool_with_broker, ToolBroker, ToolRequest, ToolStatus};

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
