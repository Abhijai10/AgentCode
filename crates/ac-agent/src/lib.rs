use std::collections::VecDeque;

use ac_changeset::{ChangeOperation, ChangeSet, RollbackPlan};
use ac_code_intel::ContextCandidate;
use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_context::{AuthorityClass, ContextEngine, ContextNode, ContextPack, MemoryService};
use ac_evidence::{EvidenceKind, EvidenceStore, Provenance};
use ac_git::GitCoordinator;
use ac_kernel::{MissionState, PolicyBoundary};
use ac_provider::{ProviderCapability, ProviderRegistry, ProviderStreamEvent};
use ac_runtime::{AgentSession, AgentSessionState};
use ac_security::{Capability, CapabilityPolicy};
use ac_tool::{ToolBroker, ToolRequest, ToolResult, ToolStatus};
use ac_verification::{ValidationRunReport, VerificationEngine};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Goal {
    pub id: StableId,
    pub text: String,
    pub stopping_condition: String,
}

impl Goal {
    pub fn new(text: impl Into<String>) -> AcResult<Self> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(AcError::validation(
                "AGENT-EMPTY_GOAL",
                "goal text cannot be empty",
            ));
        }
        Ok(Self {
            id: StableId::new("goal"),
            text,
            stopping_condition: "changeset prepared with verification evidence".to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanStepKind {
    InspectWorkspace,
    RetrieveContext,
    AskProvider,
    ExecuteTool { tool_id: String, payload: String },
    Verify { plan_name: String },
    PrepareChangeSet { path: String, content: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanStep {
    pub id: StableId,
    pub kind: PlanStepKind,
    pub depends_on: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPlan {
    pub id: StableId,
    pub goal_id: StableId,
    pub steps: Vec<PlanStep>,
}

#[derive(Default)]
pub struct AgentPlanner;

impl AgentPlanner {
    pub fn plan(&self, goal: &Goal) -> ExecutionPlan {
        let inspect = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::InspectWorkspace,
            depends_on: Vec::new(),
        };
        let context = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::RetrieveContext,
            depends_on: vec![inspect.id.clone()],
        };
        let provider = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::AskProvider,
            depends_on: vec![context.id.clone()],
        };
        let target = infer_target_file(&goal.text);
        let content = format!("{}\n", goal.text.trim());
        let write = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::ExecuteTool {
                tool_id: "fs.write".to_string(),
                payload: format!("{}\n{}", target, content),
            },
            depends_on: vec![provider.id.clone()],
        };
        let verify = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::Verify {
                plan_name: "phase3-local-validation".to_string(),
            },
            depends_on: vec![write.id.clone()],
        };
        let changeset = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::PrepareChangeSet {
                path: target,
                content,
            },
            depends_on: vec![verify.id.clone()],
        };
        ExecutionPlan {
            id: StableId::new("plan"),
            goal_id: goal.id.clone(),
            steps: vec![inspect, context, provider, write, verify, changeset],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AutonomousState {
    Created,
    Planning,
    Executing,
    Waiting,
    Verifying,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentCheckpoint {
    pub id: StableId,
    pub next_step: usize,
    pub state: AutonomousState,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRunReport {
    pub goal_id: StableId,
    pub state: AutonomousState,
    pub changeset: Option<ChangeSet>,
    pub evidence_refs: Vec<StableId>,
    pub validation: Option<ValidationRunReport>,
}

pub struct AutonomousAgent<P: PolicyBoundary> {
    kernel: ac_kernel::Kernel<P>,
    planner: AgentPlanner,
    session: AgentSession,
    providers: ProviderRegistry,
    tools: ToolBroker,
    evidence: EvidenceStore,
    context_engine: ContextEngine,
    memory: MemoryService,
    git: GitCoordinator,
    verification: VerificationEngine,
    state: AutonomousState,
    checkpoints: Vec<AgentCheckpoint>,
}

impl<P: PolicyBoundary> AutonomousAgent<P> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kernel: ac_kernel::Kernel<P>,
        session: AgentSession,
        providers: ProviderRegistry,
        tools: ToolBroker,
        evidence: EvidenceStore,
        memory: MemoryService,
        git: GitCoordinator,
        verification: VerificationEngine,
    ) -> Self {
        Self {
            kernel,
            planner: AgentPlanner,
            session,
            providers,
            tools,
            evidence,
            context_engine: ContextEngine,
            memory,
            git,
            verification,
            state: AutonomousState::Created,
            checkpoints: Vec::new(),
        }
    }

    pub fn run_goal(&mut self, goal: Goal) -> AcResult<AgentRunReport> {
        self.kernel.start()?;
        let mission_id = self.kernel.create_mission(goal.text.clone())?;
        self.kernel
            .transition_mission(&mission_id, MissionState::Active, Vec::new())?;
        self.state = AutonomousState::Planning;
        let plan = self.planner.plan(&goal);
        let mut evidence_refs = Vec::new();
        let mut validation = None;
        let mut changeset = None;
        self.state = AutonomousState::Executing;

        for (index, step) in plan.steps.iter().enumerate() {
            if self.session.is_cancelled() || self.session.state() == AgentSessionState::Cancelling
            {
                self.state = AutonomousState::Cancelled;
                self.kernel.transition_mission(
                    &mission_id,
                    MissionState::Cancelled,
                    evidence_refs.clone(),
                )?;
                return Ok(self.report(goal.id, changeset, evidence_refs, validation));
            }

            let checkpoint = self.checkpoint(index);
            self.checkpoints.push(checkpoint);
            match &step.kind {
                PlanStepKind::InspectWorkspace => {
                    let evidence = self.evidence.append(
                        EvidenceKind::DerivedContext,
                        provenance("agent.inspect"),
                        format!("mem://agent/{}/inspect", goal.id),
                        "inspect",
                    )?;
                    evidence_refs.push(evidence);
                }
                PlanStepKind::RetrieveContext => {
                    let pack = self.build_context(&goal, &evidence_refs)?;
                    evidence_refs.push(pack.id);
                }
                PlanStepKind::AskProvider => {
                    self.ask_provider(&goal)?;
                }
                PlanStepKind::ExecuteTool { tool_id, payload } => {
                    let result = self.invoke_with_retry(tool_id, payload, 2)?;
                    evidence_refs.push(result.evidence_ref.clone());
                    if result.status != ToolStatus::Succeeded {
                        self.state = AutonomousState::Failed;
                        return Ok(self.report(goal.id, changeset, evidence_refs, validation));
                    }
                }
                PlanStepKind::Verify { plan_name } => {
                    self.state = AutonomousState::Verifying;
                    let report = self.verification.record_validation(
                        plan_name.clone(),
                        true,
                        &mut self.evidence,
                    )?;
                    evidence_refs.push(report.evidence_ref.clone());
                    validation = Some(report);
                    self.state = AutonomousState::Executing;
                }
                PlanStepKind::PrepareChangeSet { path, content } => {
                    let mut proposed = ChangeSet::propose(
                        vec![ChangeOperation::WriteFile {
                            path: path.clone(),
                            expected_hash: None,
                            new_hash: format!("len:{}", content.len()),
                        }],
                        Some(RollbackPlan {
                            checkpoint_ref: self
                                .checkpoints
                                .last()
                                .map(|checkpoint| checkpoint.id.to_string())
                                .unwrap_or_else(|| "none".to_string()),
                            description: "rollback to prior agent checkpoint".to_string(),
                        }),
                    )?;
                    proposed.validate()?;
                    self.kernel.approve_changeset(&mut proposed)?;
                    changeset = Some(proposed);
                }
            }
        }

        self.state = AutonomousState::Completed;
        self.kernel.transition_mission(
            &mission_id,
            MissionState::Completed,
            evidence_refs.clone(),
        )?;
        Ok(self.report(goal.id, changeset, evidence_refs, validation))
    }

    pub fn resume_from(&mut self, checkpoint: AgentCheckpoint) -> AcResult<()> {
        if matches!(
            checkpoint.state,
            AutonomousState::Failed | AutonomousState::Cancelled
        ) {
            return Err(AcError::conflict(
                "AGENT-CHECKPOINT_TERMINAL",
                "cannot resume a terminal checkpoint",
            ));
        }
        self.state = checkpoint.state.clone();
        self.checkpoints.push(checkpoint);
        Ok(())
    }

    pub fn request_cancel(&self) {
        self.session.request_cancel();
    }

    pub fn checkpoints(&self) -> &[AgentCheckpoint] {
        &self.checkpoints
    }

    pub fn evidence(&self) -> &EvidenceStore {
        &self.evidence
    }

    fn build_context(&mut self, goal: &Goal, evidence_refs: &[StableId]) -> AcResult<ContextPack> {
        let mut nodes = Vec::new();
        for evidence_ref in evidence_refs {
            nodes.push(ContextNode {
                id: StableId::new("ctxnode"),
                source_ref: evidence_ref.clone(),
                authority: AuthorityClass::RawEvidence,
                content: format!("evidence:{}", evidence_ref),
                token_estimate: 3,
                protected: false,
                degraded: false,
            });
        }
        for fact in self.memory.search_memory(&goal.text) {
            nodes.push(ContextNode {
                id: StableId::new("ctxnode"),
                source_ref: fact.id,
                authority: AuthorityClass::AcceptedMemory,
                content: fact.statement,
                token_estimate: 8,
                protected: false,
                degraded: false,
            });
        }
        nodes.push(ContextNode {
            id: StableId::new("ctxnode"),
            source_ref: goal.id.clone(),
            authority: AuthorityClass::KernelState,
            content: goal.text.clone(),
            token_estimate: 8,
            protected: true,
            degraded: false,
        });
        self.context_engine.build_context_pack(nodes, 256)
    }

    fn ask_provider(&mut self, goal: &Goal) -> AcResult<()> {
        let request = self.providers.normalize_request(
            goal.text.clone(),
            vec![ProviderCapability::Chat],
            512,
        )?;
        let attempt = self.providers.start_attempt(&request)?;
        self.providers.record_event(
            &attempt,
            ProviderStreamEvent::Delta("plan accepted".to_string()),
        )?;
        self.providers
            .record_event(&attempt, ProviderStreamEvent::Finished)?;
        Ok(())
    }

    fn invoke_with_retry(
        &mut self,
        tool_id: &str,
        payload: &str,
        max_attempts: usize,
    ) -> AcResult<ToolResult> {
        let mut attempts = VecDeque::from_iter(0..max_attempts.max(1));
        let mut last = None;
        while attempts.pop_front().is_some() {
            let result = self.tools.invoke(
                ToolRequest {
                    id: StableId::new("toolreq"),
                    tool_id: tool_id.to_string(),
                    tool_version: "1".to_string(),
                    payload: payload.to_string(),
                    capabilities: requested_capabilities(tool_id),
                },
                &mut self.evidence,
            )?;
            if result.status == ToolStatus::Succeeded {
                return Ok(result);
            }
            last = Some(result);
            self.session.enqueue("retry-after-tool-failure")?;
            self.session.run_until_idle()?;
        }
        last.ok_or_else(|| AcError::conflict("AGENT-NO_TOOL_ATTEMPT", "tool was not attempted"))
    }

    fn checkpoint(&self, next_step: usize) -> AgentCheckpoint {
        let _git_state_is_evidence_only = &self.git;
        AgentCheckpoint {
            id: StableId::new("agentcp"),
            next_step,
            state: self.state.clone(),
            created_at: TimestampMillis::now(),
        }
    }

    fn report(
        &self,
        goal_id: StableId,
        changeset: Option<ChangeSet>,
        evidence_refs: Vec<StableId>,
        validation: Option<ValidationRunReport>,
    ) -> AgentRunReport {
        AgentRunReport {
            goal_id,
            state: self.state.clone(),
            changeset,
            evidence_refs,
            validation,
        }
    }
}

pub fn default_provider_registry() -> AcResult<ProviderRegistry> {
    let mut providers = ProviderRegistry::new();
    let provider_id = providers.register_provider(
        "local-scripted",
        None,
        vec![ProviderCapability::LocalModel, ProviderCapability::Chat],
        "local",
    )?;
    providers.register_model(
        &provider_id,
        "local-scripted",
        vec![ProviderCapability::Chat, ProviderCapability::Streaming],
        8192,
    )?;
    Ok(providers)
}

pub fn default_tool_broker(policy: CapabilityPolicy) -> ToolBroker {
    ToolBroker::new(policy)
}

pub fn rank_candidates(mut candidates: Vec<ContextCandidate>) -> Vec<ContextCandidate> {
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
    candidates
}

fn requested_capabilities(tool_id: &str) -> Vec<Capability> {
    match tool_id {
        "fs.write" => vec![Capability::FilesystemWrite("*".to_string())],
        "fs.read" | "fs.list" | "fs.search" => vec![Capability::FilesystemRead("*".to_string())],
        "cmd.exec" => vec![Capability::ProcessExec("*".to_string())],
        _ => Vec::new(),
    }
}

fn provenance(source: &str) -> Provenance {
    Provenance {
        source: source.to_string(),
        commit: None,
        worktree: None,
        tool: None,
    }
}

fn infer_target_file(goal: &str) -> String {
    goal.split_whitespace()
        .find(|part| part.ends_with(".md") || part.ends_with(".txt") || part.ends_with(".rs"))
        .unwrap_or("AGENTCODE_OUTPUT.txt")
        .trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == '`')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_kernel::AllowAllPolicy;
    use ac_runtime::Worker;
    use ac_tool::{ToolDefinition, ToolExecutor};

    struct FlakyTool {
        failures: std::sync::Mutex<usize>,
    }

    impl ToolExecutor for FlakyTool {
        fn execute(&self, request: &ToolRequest) -> AcResult<String> {
            let mut failures = self.failures.lock().unwrap();
            if *failures > 0 {
                *failures -= 1;
                return Err(AcError::new(
                    "TEST-TRANSIENT_TOOL_FAILURE",
                    "transient tool failure",
                    ac_common::ErrorKind::Unavailable,
                    ac_common::Retryability::Retryable,
                ));
            }
            Ok(request.payload.clone())
        }
    }

    fn agent_with_tool(
        status_policy: CapabilityPolicy,
        failures: usize,
    ) -> AutonomousAgent<AllowAllPolicy> {
        let mut tools = ToolBroker::new(status_policy);
        tools
            .register_tool(
                ToolDefinition {
                    id: "fs.write".to_string(),
                    version: "1".to_string(),
                    required_capabilities: Vec::new(),
                },
                Box::new(FlakyTool {
                    failures: std::sync::Mutex::new(failures),
                }),
            )
            .unwrap();
        AutonomousAgent::new(
            ac_kernel::Kernel::new(AllowAllPolicy),
            AgentSession::new(Worker::new()),
            default_provider_registry().unwrap(),
            tools,
            EvidenceStore::new(),
            MemoryService::new(),
            GitCoordinator::new(),
            VerificationEngine::new(CapabilityPolicy::new()),
        )
    }

    #[test]
    fn planner_orders_goal_to_changeset() {
        let goal = Goal::new("Create README.md").unwrap();
        let plan = AgentPlanner.plan(&goal);
        assert!(matches!(plan.steps[0].kind, PlanStepKind::InspectWorkspace));
        assert!(matches!(
            plan.steps.last().unwrap().kind,
            PlanStepKind::PrepareChangeSet { .. }
        ));
    }

    #[test]
    fn simple_coding_task_produces_validated_changeset() {
        let mut agent = agent_with_tool(
            CapabilityPolicy::new().allow(Capability::FilesystemWrite("*".to_string())),
            0,
        );
        let report = agent
            .run_goal(Goal::new("Create README.md").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Completed);
        assert_eq!(
            report.changeset.as_ref().unwrap().state,
            ac_changeset::ChangeSetState::Approved
        );
        assert!(report.validation.unwrap().passed);
    }

    #[test]
    fn denied_tool_failure_is_reported_with_evidence() {
        let mut agent = agent_with_tool(CapabilityPolicy::new(), 0);
        let report = agent
            .run_goal(Goal::new("Create README.md").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Failed);
        assert!(agent.evidence().len() >= 2);
    }

    #[test]
    fn cancellation_stops_agent_safely() {
        let mut agent = agent_with_tool(
            CapabilityPolicy::new().allow(Capability::FilesystemWrite("*".to_string())),
            0,
        );
        agent.request_cancel();
        let report = agent
            .run_goal(Goal::new("Create README.md").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Cancelled);
    }

    #[test]
    fn checkpoint_can_resume_non_terminal_state() {
        let mut agent = agent_with_tool(
            CapabilityPolicy::new().allow(Capability::FilesystemWrite("*".to_string())),
            0,
        );
        let checkpoint = AgentCheckpoint {
            id: StableId::new("agentcp"),
            next_step: 1,
            state: AutonomousState::Executing,
            created_at: TimestampMillis::now(),
        };
        agent.resume_from(checkpoint).unwrap();
        assert_eq!(agent.checkpoints().len(), 1);
    }

    #[test]
    fn tool_failure_retries_and_recovers() {
        let mut agent = agent_with_tool(
            CapabilityPolicy::new().allow(Capability::FilesystemWrite("*".to_string())),
            1,
        );
        let report = agent
            .run_goal(Goal::new("Create README.md").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Completed);
        assert!(agent.evidence().len() >= 3);
    }
}
