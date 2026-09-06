use ac_common::StableId;
use ac_context::{AuthorityClass, ContextEngine, ContextNode};
use ac_evidence::EvidenceStore;
use ac_provider::{ProviderCapability, ProviderRegistry, ProviderStreamEvent};
use ac_security::{Capability, CapabilityPolicy};
use ac_tool::{ToolBroker, ToolDefinition, ToolExecutor, ToolRequest, ToolStatus};

struct EchoTool;

impl ToolExecutor for EchoTool {
    fn execute(&self, request: &ToolRequest) -> ac_common::AcResult<String> {
        Ok(format!("observed:{}", request.payload))
    }
}

#[test]
fn runtime_foundations_keep_authority_boundaries_separate() {
    let mut providers = ProviderRegistry::new();
    let provider_id = providers
        .register_provider(
            "local-test",
            None,
            vec![ProviderCapability::LocalModel],
            "local",
        )
        .unwrap();
    providers
        .register_model(
            &provider_id,
            "test-model",
            vec![ProviderCapability::Chat, ProviderCapability::Streaming],
            4096,
        )
        .unwrap();
    let request = providers
        .normalize_request("hello", vec![ProviderCapability::Streaming], 64)
        .unwrap();
    let attempt = providers.start_attempt(&request).unwrap();
    providers
        .record_event(&attempt, ProviderStreamEvent::Finished)
        .unwrap();

    let mut evidence = EvidenceStore::new();
    let mut broker =
        ToolBroker::new(CapabilityPolicy::new().allow(Capability::ProcessExec("echo".to_string())));
    broker
        .register_tool(
            ToolDefinition {
                id: "echo".to_string(),
                version: "1".to_string(),
                required_capabilities: vec![Capability::ProcessExec("echo".to_string())],
            },
            Box::new(EchoTool),
        )
        .unwrap();
    let result = broker
        .invoke(
            ToolRequest {
                id: StableId::new("toolreq"),
                tool_id: "echo".to_string(),
                tool_version: "1".to_string(),
                payload: "ok".to_string(),
                capabilities: Vec::new(),
            },
            &mut evidence,
        )
        .unwrap();

    let pack = ContextEngine
        .build_context_pack(
            vec![ContextNode {
                id: StableId::new("node"),
                source_ref: result.evidence_ref.clone(),
                authority: AuthorityClass::RawEvidence,
                content: result.observation,
                token_estimate: 5,
                protected: false,
                degraded: false,
            }],
            10,
        )
        .unwrap();

    assert_eq!(providers.attempts().len(), 1);
    assert_eq!(result.status, ToolStatus::Succeeded);
    assert_eq!(evidence.len(), 1);
    assert_eq!(pack.nodes[0].authority, AuthorityClass::RawEvidence);
}
