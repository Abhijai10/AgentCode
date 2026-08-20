use std::collections::BTreeMap;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_evidence::{EvidenceKind, EvidenceStore, Provenance};
use ac_sandbox::{ExecRequest, SandboxManager};
use ac_security::{Capability, CapabilityPolicy, SecurityDecision};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDefinition {
    pub id: String,
    pub version: String,
    pub required_capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolRequest {
    pub id: StableId,
    pub tool_id: String,
    pub tool_version: String,
    pub payload: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolStatus {
    Succeeded,
    Failed,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResult {
    pub request_id: StableId,
    pub status: ToolStatus,
    pub observation: String,
    pub evidence_ref: StableId,
    pub finished_at: TimestampMillis,
}

pub trait ToolExecutor {
    fn execute(&self, request: &ToolRequest) -> AcResult<String>;
}

pub struct ToolBroker {
    definitions: BTreeMap<String, ToolDefinition>,
    executors: BTreeMap<String, Box<dyn ToolExecutor>>,
    policy: CapabilityPolicy,
}

impl ToolBroker {
    pub fn new(policy: CapabilityPolicy) -> Self {
        Self {
            definitions: BTreeMap::new(),
            executors: BTreeMap::new(),
            policy,
        }
    }

    pub fn register_tool(
        &mut self,
        definition: ToolDefinition,
        executor: Box<dyn ToolExecutor>,
    ) -> AcResult<()> {
        if definition.id.trim().is_empty() || definition.version.trim().is_empty() {
            return Err(AcError::validation(
                "TOOL-INVALID_DEFINITION",
                "tool id and version are required",
            ));
        }
        self.executors.insert(definition.id.clone(), executor);
        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    pub fn invoke(
        &self,
        request: ToolRequest,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ToolResult> {
        let definition = self
            .definitions
            .get(&request.tool_id)
            .ok_or_else(|| AcError::validation("TOOL-UNKNOWN_TOOL", "tool is not registered"))?;
        if definition.version != request.tool_version {
            return Err(AcError::validation(
                "TOOL-VERSION_MISMATCH",
                "tool request version does not match registry",
            ));
        }
        let mut requested = definition.required_capabilities.clone();
        requested.extend(request.capabilities.clone());
        if self.policy.evaluate(&requested) != SecurityDecision::Allow {
            let evidence_ref = evidence_store.append(
                EvidenceKind::CommandOutput,
                Provenance {
                    source: "tool-broker".to_string(),
                    commit: None,
                    worktree: None,
                    tool: Some(request.tool_id.clone()),
                },
                format!("mem://tool/{}/denied", request.id),
                "denied",
            )?;
            return Ok(ToolResult {
                request_id: request.id,
                status: ToolStatus::Denied,
                observation: "policy denied tool invocation".to_string(),
                evidence_ref,
                finished_at: TimestampMillis::now(),
            });
        }
        let executor = self.executors.get(&request.tool_id).ok_or_else(|| {
            AcError::validation("TOOL-MISSING_EXECUTOR", "tool executor not found")
        })?;
        let observation = executor.execute(&request)?;
        let evidence_ref = evidence_store.append(
            EvidenceKind::CommandOutput,
            Provenance {
                source: "tool-broker".to_string(),
                commit: None,
                worktree: None,
                tool: Some(request.tool_id.clone()),
            },
            format!("mem://tool/{}/output", request.id),
            format!("len:{}", observation.len()),
        )?;
        Ok(ToolResult {
            request_id: request.id,
            status: ToolStatus::Succeeded,
            observation,
            evidence_ref,
            finished_at: TimestampMillis::now(),
        })
    }
}

pub struct CommandPlanner<'a> {
    sandbox: &'a SandboxManager,
}

impl<'a> CommandPlanner<'a> {
    pub fn new(sandbox: &'a SandboxManager) -> Self {
        Self { sandbox }
    }

    pub fn plan(&self, request: ExecRequest) -> AcResult<StableId> {
        Ok(self.sandbox.prepare_execution(request)?.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoExecutor;

    impl ToolExecutor for EchoExecutor {
        fn execute(&self, request: &ToolRequest) -> AcResult<String> {
            Ok(request.payload.clone())
        }
    }

    #[test]
    fn broker_denies_before_executor_runs_without_policy() {
        let mut broker = ToolBroker::new(CapabilityPolicy::new());
        broker
            .register_tool(
                ToolDefinition {
                    id: "echo".to_string(),
                    version: "1".to_string(),
                    required_capabilities: vec![Capability::ProcessExec("echo".to_string())],
                },
                Box::new(EchoExecutor),
            )
            .unwrap();
        let mut evidence = EvidenceStore::new();
        let result = broker
            .invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: "echo".to_string(),
                    tool_version: "1".to_string(),
                    payload: "hello".to_string(),
                    capabilities: Vec::new(),
                },
                &mut evidence,
            )
            .unwrap();
        assert_eq!(result.status, ToolStatus::Denied);
        assert_eq!(evidence.len(), 1);
    }
}
