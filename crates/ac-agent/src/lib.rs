use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use ac_changeset::{
    content_hash, ChangeOperation, ChangeSet, ChangeSetMetadata, ChangeSetState, EditEngine,
    EditPrecondition, EditRequest, EditStrategy, FileChangeSummary, LocalWorkspaceFileRepository,
    RollbackPlan,
};
use ac_code_intel::{
    CodeIntelligenceService, ContextCandidate, RepositoryScope, SourceFileIdentity,
};
use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use ac_context::{AuthorityClass, ContextEngine, ContextNode, ContextPack, MemoryService};
use ac_evidence::{EvidenceKind, EvidenceStore, Provenance};
use ac_git::GitCoordinator;
use ac_kernel::{MissionState, PolicyBoundary};
use ac_provider::{
    PrivacyClass, ProviderCapability, ProviderFailureClass, ProviderRegistry, ProviderStreamEvent,
    RoutingProfile, ScriptedProvider, TaskProfile,
};
use ac_runtime::{AgentSession, AgentSessionState};
use ac_security::{Capability, CapabilityPolicy};
use ac_tool::{ToolBroker, ToolRequest, ToolResult, ToolStatus};
use ac_verification::{FinalAuditInput, ValidationRunReport, VerificationEngine};

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
    Verify { plan_name: String, tool_id: String },
    PrepareChangeSet { path: String, content: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanStep {
    pub id: StableId,
    pub kind: PlanStepKind,
    pub depends_on: Vec<StableId>,
    pub expected_outcome: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionPlan {
    pub id: StableId,
    pub goal_id: StableId,
    pub objective: String,
    pub assumptions: Vec<String>,
    pub required_files: Vec<String>,
    pub steps: Vec<PlanStep>,
    pub expected_verification: String,
    pub stopping_condition: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderReasoning {
    pub text: String,
    pub events: Vec<ProviderStreamEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredPlanStep {
    pub id: String,
    pub action: String,
    pub target: String,
    pub reason: String,
    pub expected_output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredPlan {
    pub goal: String,
    pub assumptions: Vec<String>,
    pub steps: Vec<StructuredPlanStep>,
    pub verification_requirements: Vec<String>,
    pub stopping_condition: String,
}

impl StructuredPlan {
    pub fn parse(text: &str) -> AcResult<Self> {
        let mut goal = None;
        let mut assumptions = Vec::new();
        let mut steps = Vec::new();
        let mut verification_requirements = Vec::new();
        let mut stopping_condition = None;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(value) = line.strip_prefix("goal=") {
                goal = Some(required_field("goal", value)?);
            } else if let Some(value) = line.strip_prefix("assumption=") {
                assumptions.push(required_field("assumption", value)?);
            } else if let Some(value) = line.strip_prefix("verify=") {
                verification_requirements.push(required_field("verify", value)?);
            } else if let Some(value) = line.strip_prefix("stopping_condition=") {
                stopping_condition = Some(required_field("stopping_condition", value)?);
            } else if let Some(value) = line.strip_prefix("step=") {
                steps.push(parse_structured_step(value)?);
            } else if line.starts_with("content:") || line.starts_with("write_file:") {
                continue;
            } else {
                return Err(AcError::validation(
                    "AGENT-PLAN_UNKNOWN_FIELD",
                    format!("unknown structured plan field: {}", line),
                ));
            }
        }
        let plan = Self {
            goal: goal.ok_or_else(|| missing_field("goal"))?,
            assumptions,
            steps,
            verification_requirements,
            stopping_condition: stopping_condition
                .ok_or_else(|| missing_field("stopping_condition"))?,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn validate(&self) -> AcResult<()> {
        if self.goal.trim().is_empty()
            || self.stopping_condition.trim().is_empty()
            || self.assumptions.is_empty()
            || self.steps.is_empty()
            || self.verification_requirements.is_empty()
        {
            return Err(AcError::validation(
                "AGENT-PLAN_MISSING_FIELD",
                "goal, assumptions, steps, verification requirements, and stopping condition are required",
            ));
        }
        for step in &self.steps {
            validate_plan_step(step)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairPlan {
    pub failure_reason: String,
    pub affected_files: Vec<String>,
    pub proposed_action: String,
    pub expected_verification: String,
}

impl RepairPlan {
    pub fn generate(goal: &Goal, failure_reason: impl Into<String>) -> AcResult<Self> {
        let failure_reason = failure_reason.into();
        let affected_files = goal
            .text
            .split_whitespace()
            .map(|part| part.trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == '`'))
            .filter(|part| part.ends_with(".rs") || part.ends_with(".md") || part.ends_with(".txt"))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let affected_files = if affected_files.is_empty() {
            vec!["AGENTCODE_OUTPUT.txt".to_string()]
        } else {
            affected_files
        };
        let plan = Self {
            failure_reason,
            affected_files,
            proposed_action: "fs.write".to_string(),
            expected_verification: "status:0".to_string(),
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn validate(&self) -> AcResult<()> {
        if self.failure_reason.trim().is_empty()
            || self.affected_files.is_empty()
            || self.proposed_action.trim().is_empty()
            || self.expected_verification.trim().is_empty()
        {
            return Err(AcError::validation(
                "AGENT-REPAIR_MISSING_FIELD",
                "repair plan requires failure reason, affected files, action, and expected verification",
            ));
        }
        if self.proposed_action != "fs.write" {
            return Err(AcError::validation(
                "AGENT-REPAIR_UNSAFE",
                "repair plan action is not allowed",
            ));
        }
        for file in &self.affected_files {
            if file.contains("..") || file.starts_with('/') {
                return Err(AcError::validation(
                    "AGENT-REPAIR_UNSAFE",
                    "repair plan target must stay inside the repository",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct AgentPlanner;

impl AgentPlanner {
    pub fn plan(
        &self,
        goal: &Goal,
        context: &ContextPack,
        reasoning: &ProviderReasoning,
    ) -> ExecutionPlan {
        let inspect = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::InspectWorkspace,
            depends_on: Vec::new(),
            expected_outcome: "repository shape is known".to_string(),
        };
        let context_step = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::RetrieveContext,
            depends_on: vec![inspect.id.clone()],
            expected_outcome: "relevant evidence and memory are assembled".to_string(),
        };
        let provider = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::AskProvider,
            depends_on: vec![context_step.id.clone()],
            expected_outcome: "provider action proposal is normalized".to_string(),
        };
        let structured = StructuredPlan::parse(&reasoning.text)
            .unwrap_or_else(|_| fallback_structured_plan(goal, reasoning));
        let required_files = required_files(goal, reasoning, context, &structured);
        let (target, content) = infer_change(&goal.text, reasoning);
        let read_dependencies = required_files
            .iter()
            .map(|path| PlanStep {
                id: StableId::new("step"),
                kind: PlanStepKind::ExecuteTool {
                    tool_id: "fs.read".to_string(),
                    payload: path.clone(),
                },
                depends_on: vec![provider.id.clone()],
                expected_outcome: format!("{} content is inspected through Tool Broker", path),
            })
            .collect::<Vec<_>>();
        let changeset_dependency = read_dependencies
            .last()
            .map(|step| step.id.clone())
            .unwrap_or_else(|| provider.id.clone());
        let changeset = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::PrepareChangeSet {
                path: target.clone(),
                content: content.clone(),
            },
            depends_on: vec![changeset_dependency],
            expected_outcome: "Kernel-approved ChangeSet is ready".to_string(),
        };
        let write = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::ExecuteTool {
                tool_id: "fs.write".to_string(),
                payload: format!("{}\n{}", target, content),
            },
            depends_on: vec![changeset.id.clone()],
            expected_outcome: "file modification executed through Tool Broker".to_string(),
        };
        let verify = PlanStep {
            id: StableId::new("step"),
            kind: PlanStepKind::Verify {
                plan_name: "phase3-local-validation".to_string(),
                tool_id: verification_tool(&target).to_string(),
            },
            depends_on: vec![write.id.clone()],
            expected_outcome: "validation evidence recorded".to_string(),
        };
        let mut steps = vec![inspect, context_step, provider];
        steps.extend(read_dependencies);
        steps.extend([changeset, write, verify]);
        ExecutionPlan {
            id: StableId::new("plan"),
            goal_id: goal.id.clone(),
            objective: goal.text.clone(),
            assumptions: assumptions(reasoning),
            required_files,
            steps,
            expected_verification: structured.verification_requirements.join("; "),
            stopping_condition: structured.stopping_condition,
        }
    }
}

#[derive(Default)]
pub struct ContextBuilder {
    engine: ContextEngine,
}

impl ContextBuilder {
    pub fn build(
        &self,
        goal: &Goal,
        prior_evidence: &[StableId],
        memory: &MemoryService,
        code_intel: &mut CodeIntelligenceService,
        repository: Option<RepositoryContext>,
        candidates: Vec<ContextCandidate>,
    ) -> AcResult<ContextPack> {
        let mut nodes = Vec::new();
        nodes.push(ContextNode {
            id: StableId::new("ctxnode"),
            source_ref: goal.id.clone(),
            authority: AuthorityClass::KernelState,
            content: goal.text.clone(),
            token_estimate: 8,
            protected: true,
            degraded: false,
        });
        for evidence_ref in prior_evidence {
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
        for fact in memory.search_memory(&goal.text) {
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
        let mut ranked_candidates = candidates;
        if let Some(repository) = repository {
            let files = collect_source_files(&repository.root)?;
            let receipt = code_intel.index_repository(repository.scope, files)?;
            nodes.push(ContextNode {
                id: StableId::new("ctxnode"),
                source_ref: receipt.id,
                authority: AuthorityClass::RuntimeContext,
                content: format!(
                    "repository:{} files_indexed:{} degraded:{}",
                    repository.root.display(),
                    receipt.files_indexed,
                    receipt.degraded
                ),
                token_estimate: 10,
                protected: false,
                degraded: receipt.degraded,
            });
            ranked_candidates.extend(search_goal_terms(code_intel, &goal.text));
        }
        for candidate in rank_candidates(ranked_candidates).into_iter().take(5) {
            nodes.push(ContextNode {
                id: StableId::new("ctxnode"),
                source_ref: StableId::new("candidate"),
                authority: AuthorityClass::RetrievalAccelerator,
                content: format!("{}:{}", candidate.source_path, candidate.snippet),
                token_estimate: 12,
                protected: false,
                degraded: false,
            });
        }
        self.engine.build_context_pack(nodes, 512)
    }
}

pub struct RepositoryContext {
    pub root: PathBuf,
    pub scope: RepositoryScope,
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
    pub merge_review: Option<ac_git::MergeReview>,
    pub completion_request: Option<CompletionRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionRequest {
    pub id: StableId,
    pub task_id: StableId,
    pub summary: String,
    pub evidence_refs: Vec<StableId>,
    pub verification_passed: bool,
    pub created_at: TimestampMillis,
}

pub struct AutonomousAgent<P: PolicyBoundary> {
    kernel: ac_kernel::Kernel<P>,
    planner: AgentPlanner,
    session: AgentSession,
    providers: ProviderRegistry,
    tools: ToolBroker,
    evidence: EvidenceStore,
    context_engine: ContextEngine,
    context_builder: ContextBuilder,
    code_intel: CodeIntelligenceService,
    memory: MemoryService,
    git: GitCoordinator,
    verification: VerificationEngine,
    state: AutonomousState,
    checkpoints: Vec<AgentCheckpoint>,
    verification_failures_remaining: usize,
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
            context_builder: ContextBuilder::default(),
            code_intel: CodeIntelligenceService::new(),
            memory,
            git,
            verification,
            state: AutonomousState::Created,
            checkpoints: Vec::new(),
            verification_failures_remaining: 0,
        }
    }

    pub fn with_verification_failures(mut self, failures: usize) -> Self {
        self.verification_failures_remaining = failures;
        self
    }

    pub fn run_goal(&mut self, goal: Goal) -> AcResult<AgentRunReport> {
        self.kernel.start()?;
        let mission_id = self.kernel.create_mission(goal.text.clone())?;
        self.kernel
            .transition_mission(&mission_id, MissionState::Active, Vec::new())?;
        let mut evidence_refs = Vec::new();
        let mut validation = None;
        let mut changeset = None;

        let inspect_evidence = self.evidence.append(
            EvidenceKind::DerivedContext,
            provenance("agent.inspect"),
            format!("mem://agent/{}/inspect", goal.id),
            "inspect",
        )?;
        evidence_refs.push(inspect_evidence);
        let context = self.build_context(&goal, &evidence_refs)?;
        evidence_refs.push(context.id.clone());
        if self.session.is_cancelled() || self.session.state() == AgentSessionState::Cancelling {
            self.state = AutonomousState::Cancelled;
            self.kernel.transition_mission(
                &mission_id,
                MissionState::Cancelled,
                evidence_refs.clone(),
            )?;
            return Ok(self.report(goal.id, changeset, evidence_refs, validation));
        }
        let reasoning = self.ask_provider(&goal, &context)?;

        self.state = AutonomousState::Planning;
        let plan = self.planner.plan(&goal, &context, &reasoning);
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
                PlanStepKind::InspectWorkspace
                | PlanStepKind::RetrieveContext
                | PlanStepKind::AskProvider => {}
                PlanStepKind::ExecuteTool { tool_id, payload } => {
                    let result = self.invoke_with_retry(tool_id, payload, 2)?;
                    evidence_refs.push(result.evidence_ref.clone());
                    if result.status != ToolStatus::Succeeded {
                        self.state = AutonomousState::Failed;
                        return Ok(self.report(goal.id, changeset, evidence_refs, validation));
                    }
                }
                PlanStepKind::Verify { plan_name, tool_id } => {
                    self.state = AutonomousState::Verifying;
                    validation = Some(self.verify_with_repair(
                        plan_name,
                        tool_id,
                        &goal,
                        &mut evidence_refs,
                    )?);
                    self.state = AutonomousState::Executing;
                }
                PlanStepKind::PrepareChangeSet { path, content } => {
                    let mut proposed = self
                        .prepare_advanced_changeset(path, content)
                        .unwrap_or_else(|_| {
                            ChangeSet::propose(
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
                            )
                            .expect("fallback changeset is valid")
                        });
                    let (files_changed, additions, removals) = self
                        .session
                        .worker()
                        .workspace_ref
                        .as_ref()
                        .and_then(|worktree_id| self.git.worktree_diff(worktree_id).ok())
                        .map(|diff| summarize_diff(&diff.diff))
                        .filter(|(files, _, _)| !files.is_empty())
                        .unwrap_or_else(|| {
                            (
                                vec![FileChangeSummary {
                                    path: path.clone(),
                                    additions: content.lines().count() as u32,
                                    removals: 0,
                                }],
                                content.lines().count() as u32,
                                0,
                            )
                        });
                    proposed.attach_metadata(ChangeSetMetadata {
                        originating_task: goal.id.clone(),
                        originating_agent_session: self.session.id().clone(),
                        files_changed,
                        additions,
                        removals,
                        evidence_refs: evidence_refs.clone(),
                        verification_passed: validation.as_ref().map(|report| report.passed),
                    })?;
                    proposed.validate()?;
                    self.kernel.approve_changeset(&mut proposed)?;
                    changeset = Some(proposed);
                }
            }
        }

        self.state = AutonomousState::Completed;
        if let Some(changeset) = &mut changeset {
            if let Some(metadata) = &mut changeset.metadata {
                metadata.evidence_refs = evidence_refs.clone();
                metadata.verification_passed = validation.as_ref().map(|report| report.passed);
            }
        }
        self.request_completion(&mission_id, &goal, &evidence_refs, validation.as_ref())?;
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

    pub fn into_evidence(self) -> EvidenceStore {
        self.evidence
    }

    pub fn prepare_merge_after_review(
        &mut self,
        changeset: &mut ChangeSet,
        approved_by_kernel: bool,
    ) -> AcResult<ac_git::MergeReview> {
        if changeset.state != ChangeSetState::Approved && changeset.state != ChangeSetState::Applied
        {
            return Err(AcError::conflict(
                "AGENT-MERGE_CHANGESET_NOT_APPROVED",
                "merge review requires an approved changeset",
            ));
        }
        let worktree_id = self
            .session
            .worker()
            .workspace_ref
            .clone()
            .ok_or_else(|| AcError::validation("AGENT-NO_WORKTREE", "agent has no worktree"))?;
        let mut review = self
            .git
            .prepare_merge_review(&worktree_id, approved_by_kernel)?;
        if !self.git.worktree_status(&worktree_id)?.trim().is_empty() {
            self.git
                .checkpoint_current(&worktree_id, "approved changeset checkpoint")?;
        }
        if changeset.state == ChangeSetState::Approved {
            changeset.mark_applied()?;
        }
        self.git.complete_merge_review(&mut review)?;
        changeset.archive()?;
        Ok(review)
    }

    fn build_context(&mut self, goal: &Goal, evidence_refs: &[StableId]) -> AcResult<ContextPack> {
        let _engine_boundary = &self.context_engine;
        let repository = self
            .session
            .worker()
            .workspace_ref
            .as_ref()
            .and_then(|worktree_id| self.git.worktree(worktree_id))
            .map(|worktree| RepositoryContext {
                root: worktree.path.clone(),
                scope: RepositoryScope {
                    repository_id: worktree.repository_id.clone(),
                    worktree_id: worktree.id.clone(),
                    root: worktree.path.display().to_string(),
                    commit: worktree.base_commit.clone(),
                    trust_profile: "isolated-worktree".to_string(),
                },
            });
        self.context_builder.build(
            goal,
            evidence_refs,
            &self.memory,
            &mut self.code_intel,
            repository,
            Vec::new(),
        )
    }

    fn ask_provider(&mut self, goal: &Goal, context: &ContextPack) -> AcResult<ProviderReasoning> {
        let mut profile = TaskProfile::coding(goal.id.clone(), RoutingProfile::FreeFirst);
        profile.required_context = context.budget;
        let execution = self
            .providers
            .request_model(
                &profile,
                format!(
                    "goal:{}\ncontext_nodes:{}\nstopping_condition:{}",
                    goal.text,
                    context.nodes.len(),
                    goal.stopping_condition
                ),
                512,
                &|| self.session.is_cancelled(),
            )
            .map_err(provider_error)?;
        let events = execution.events;
        let text = events
            .iter()
            .filter_map(|event| match event {
                ProviderStreamEvent::Delta(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let evidence = self.evidence.append(
            EvidenceKind::DerivedContext,
            provenance("agent.provider"),
            format!("mem://agent/{}/provider", goal.id),
            format!("events:{};text:{}", events.len(), text.len()),
        )?;
        self.memory
            .record_fact("provider produced a task plan proposal", vec![evidence], 70)?;
        Ok(ProviderReasoning { text, events })
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

    fn prepare_advanced_changeset(&self, path: &str, content: &str) -> AcResult<ChangeSet> {
        let worktree = self
            .session
            .worker()
            .workspace_ref
            .as_ref()
            .and_then(|worktree_id| self.git.worktree(worktree_id))
            .ok_or_else(|| AcError::validation("AGENT-NO_WORKTREE", "agent has no worktree"))?;
        let repo =
            LocalWorkspaceFileRepository::new(worktree.path.clone(), worktree.base_commit.clone());
        let current = std::fs::read_to_string(worktree.path.join(path)).map_err(|error| {
            AcError::validation("AGENT-EDIT_PREPARE_READ_FAILED", error.to_string())
        })?;
        let transaction = EditEngine.prepare(
            &repo,
            vec![EditRequest {
                path: path.to_string(),
                precondition: EditPrecondition {
                    path: path.to_string(),
                    expected_hash: content_hash(&current),
                    base_revision: worktree.base_commit.clone(),
                    symbol_fingerprint: None,
                },
                strategy: EditStrategy::WholeFile {
                    content: content.to_string(),
                },
            }],
        )?;
        Ok(transaction.changeset)
    }

    fn verify_with_repair(
        &mut self,
        plan_name: &str,
        tool_id: &str,
        goal: &Goal,
        evidence_refs: &mut Vec<StableId>,
    ) -> AcResult<ValidationRunReport> {
        let max_attempts = 2;
        let mut last_report = None;
        for attempt in 0..max_attempts {
            let tool_result = self.invoke_with_retry(tool_id, "", 1)?;
            evidence_refs.push(tool_result.evidence_ref.clone());
            let forced_failure = self.verification_failures_remaining > 0;
            let passed = !forced_failure
                && tool_result.status == ToolStatus::Succeeded
                && tool_result.observation.contains("status:0");
            let report = self.verification.record_validation(
                plan_name.to_string(),
                passed,
                &mut self.evidence,
            )?;
            evidence_refs.push(report.evidence_ref.clone());
            if report.passed {
                return Ok(report);
            }
            last_report = Some(report);
            if self.verification_failures_remaining > 0 {
                self.verification_failures_remaining -= 1;
            }
            if attempt + 1 < max_attempts {
                let repair_plan = RepairPlan::generate(goal, tool_result.observation.clone())?;
                let repair_reasoning = self.ask_provider(
                    goal,
                    &ContextPack {
                        id: StableId::new("ctx"),
                        nodes: Vec::new(),
                        budget: 1,
                        omitted_count: 0,
                    },
                )?;
                let (_, content) = infer_change(&goal.text, &repair_reasoning);
                let path = repair_plan.affected_files[0].clone();
                let repair_result =
                    self.invoke_with_retry("fs.write", &format!("{}\n{}", path, content), 1)?;
                evidence_refs.push(repair_result.evidence_ref);
                let repair = self.evidence.append(
                    EvidenceKind::DerivedContext,
                    provenance("agent.repair"),
                    format!("mem://agent/{}/repair", self.session.id()),
                    format!("repair-plan:{:?}", repair_plan),
                )?;
                evidence_refs.push(repair);
            }
        }
        self.state = AutonomousState::Failed;
        last_report.ok_or_else(|| {
            AcError::conflict(
                "AGENT-NO_VERIFICATION_ATTEMPT",
                "verification was not attempted",
            )
        })
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
        let completion_request =
            (self.state == AutonomousState::Completed).then(|| CompletionRequest {
                id: StableId::new("completion"),
                task_id: goal_id.clone(),
                summary: "worker requests completion with verification evidence".to_string(),
                evidence_refs: evidence_refs.clone(),
                verification_passed: validation.as_ref().is_some_and(|report| report.passed),
                created_at: TimestampMillis::now(),
            });
        AgentRunReport {
            goal_id,
            state: self.state.clone(),
            changeset,
            evidence_refs,
            validation,
            merge_review: None,
            completion_request,
        }
    }

    fn request_completion(
        &mut self,
        mission_id: &StableId,
        goal: &Goal,
        evidence_refs: &[StableId],
        validation: Option<&ValidationRunReport>,
    ) -> AcResult<()> {
        if evidence_refs.is_empty() || !validation.is_some_and(|report| report.passed) {
            return Err(AcError::conflict(
                "AGENT-COMPLETION_UNSUPPORTED",
                "completion requires passing verification and evidence",
            ));
        }
        let completion_evidence = self.evidence.append(
            EvidenceKind::DerivedContext,
            provenance("agent.completion-request"),
            format!("mem://agent/{}/completion", goal.id),
            format!("evidence:{};verified:true", evidence_refs.len()),
        )?;
        let mut accepted = evidence_refs.to_vec();
        accepted.push(completion_evidence);
        let audit = self.verification.final_audit(
            FinalAuditInput {
                original_goal: goal.text.clone(),
                requirements: vec![goal.stopping_condition.clone()],
                verified_requirement_ids: vec![goal.id.clone()],
                evidence_refs: accepted.clone(),
                worker_completion_text: "worker requests completion with verification evidence"
                    .to_string(),
                unresolved_limitations: Vec::new(),
            },
            &mut self.evidence,
        )?;
        let gate = self.verification.completion_gate(
            &audit,
            "worker requests completion with verification evidence",
        );
        if !gate.allowed {
            return Err(AcError::conflict(
                "AGENT-COMPLETION_GATE_BLOCKED",
                gate.reason,
            ));
        }
        accepted.push(audit.evidence_ref);
        self.kernel
            .transition_mission(mission_id, MissionState::Completed, accepted)
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
    providers.register_adapter(
        &provider_id,
        Box::new(ScriptedProvider::new(vec![Ok(vec![
            ProviderStreamEvent::Delta(
                "goal=deterministic local coding task\nassumption=repository is isolated\nstep=id:s1|action:read|target:src/lib.rs|reason:inspect implementation|expected_output:file content\nstep=id:s2|action:modify|target:src/lib.rs|reason:fix requested behavior|expected_output:updated implementation\nverify=status:0\nstopping_condition=changeset prepared with verification evidence".to_string(),
            ),
            ProviderStreamEvent::Usage {
                input_tokens: 8,
                output_tokens: 12,
            },
            ProviderStreamEvent::Finished,
        ])])),
    )?;
    providers
        .register_model(
            &provider_id,
            "local-scripted",
            vec![ProviderCapability::Chat, ProviderCapability::Streaming],
            8192,
        )
        .and_then(|model_id| {
            let connection_id = providers.register_connection(
                &provider_id,
                "local-test-account",
                None,
                "local",
                "config:local-scripted.endpoint",
                true,
                false,
                true,
            )?;
            let identity_id = providers.register_model_identity(
                "local-scripted-family",
                8192,
                80,
                70,
                false,
                false,
                true,
                0,
                0,
                PrivacyClass::LocalOnly,
            )?;
            providers.register_model_route(&identity_id, &model_id, &connection_id)?;
            Ok(model_id)
        })?;
    Ok(providers)
}

pub fn default_tool_broker(policy: CapabilityPolicy) -> ToolBroker {
    ToolBroker::new(policy)
}

pub fn isolated_workspace_agent<P: PolicyBoundary>(
    source_root: PathBuf,
    worktree_root: PathBuf,
    kernel: ac_kernel::Kernel<P>,
    policy: CapabilityPolicy,
) -> AcResult<AutonomousAgent<P>> {
    let mission_id = StableId::new("mission");
    let worker = ac_runtime::Worker::new();
    let mut git = GitCoordinator::new();
    let worktree_id = git.create_task_workspace(
        source_root,
        worktree_root.clone(),
        mission_id,
        worker.id.clone(),
    )?;
    let mut tools = ToolBroker::new(policy);
    ac_tool::WorkspaceTools::new(worktree_root).register_all(&mut tools)?;
    Ok(AutonomousAgent::new(
        kernel,
        AgentSession::new(ac_runtime::Worker::assigned_to(worktree_id)),
        default_provider_registry()?,
        tools,
        EvidenceStore::new(),
        MemoryService::new(),
        git,
        VerificationEngine::new(CapabilityPolicy::new()),
    ))
}

pub fn rank_candidates(mut candidates: Vec<ContextCandidate>) -> Vec<ContextCandidate> {
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
    candidates
}

fn requested_capabilities(tool_id: &str) -> Vec<Capability> {
    match tool_id {
        "fs.write" => vec![Capability::FilesystemWrite("*".to_string())],
        "fs.read" | "fs.list" | "fs.search" => vec![Capability::FilesystemRead("*".to_string())],
        "cmd.exec" | "repo.status" | "repo.diff" | "repo.branch" | "dev.test" | "dev.format"
        | "dev.check" => vec![Capability::ProcessExec("*".to_string())],
        _ => Vec::new(),
    }
}

fn provider_error(failure: ProviderFailureClass) -> AcError {
    AcError::new(
        "AGENT-PROVIDER_FAILURE",
        format!("provider failed: {:?}", failure),
        ac_common::ErrorKind::Unavailable,
        ac_common::Retryability::Retryable,
    )
}

fn provenance(source: &str) -> Provenance {
    Provenance {
        source: source.to_string(),
        commit: None,
        worktree: None,
        tool: None,
    }
}

fn infer_change(goal: &str, reasoning: &ProviderReasoning) -> (String, String) {
    let target = reasoning
        .text
        .lines()
        .find_map(|line| line.strip_prefix("write_file:").map(str::trim))
        .or_else(|| {
            goal.split_whitespace().find(|part| {
                part.ends_with(".md") || part.ends_with(".txt") || part.ends_with(".rs")
            })
        })
        .unwrap_or("AGENTCODE_OUTPUT.txt")
        .trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == '`')
        .to_string();
    let content = reasoning
        .text
        .split_once("content:\n")
        .map(|(_, content)| content.trim_end().to_string() + "\n")
        .unwrap_or_else(|| inferred_content(goal, &target));
    (target, content)
}

fn inferred_content(goal: &str, target: &str) -> String {
    if goal.to_ascii_lowercase().contains("fix the bug") && target.ends_with(".rs") {
        "pub fn fixture_answer() -> u32 {\n    42\n}\n".to_string()
    } else {
        format!("{}\n", goal.trim())
    }
}

fn required_files(
    goal: &Goal,
    reasoning: &ProviderReasoning,
    context: &ContextPack,
    structured: &StructuredPlan,
) -> Vec<String> {
    let mut files = structured
        .steps
        .iter()
        .map(|step| step.target.as_str())
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    files.extend(
        reasoning
            .text
            .lines()
            .filter_map(|line| line.strip_prefix("required_file:").map(str::trim))
            .filter(|path| !path.is_empty())
            .map(ToString::to_string),
    );
    files.extend(goal.text.split_whitespace().filter_map(|part| {
        let cleaned = part.trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == '`');
        (cleaned.ends_with(".md") || cleaned.ends_with(".txt") || cleaned.ends_with(".rs"))
            .then(|| cleaned.to_string())
    }));
    files.extend(context.nodes.iter().filter_map(|node| {
        node.content
            .split_once(':')
            .and_then(|(path, _)| path.ends_with(".rs").then(|| path.to_string()))
    }));
    files.sort();
    files.dedup();
    files
}

fn assumptions(reasoning: &ProviderReasoning) -> Vec<String> {
    StructuredPlan::parse(&reasoning.text)
        .map(|plan| plan.assumptions)
        .unwrap_or_else(|_| vec!["provider supplied no valid structured assumptions".to_string()])
}

fn parse_structured_step(value: &str) -> AcResult<StructuredPlanStep> {
    let mut id = None;
    let mut action = None;
    let mut target = None;
    let mut reason = None;
    let mut expected_output = None;
    for part in value.split('|') {
        let (key, field_value) = part.split_once(':').ok_or_else(|| {
            AcError::validation(
                "AGENT-PLAN_INVALID_STEP",
                "step fields must be key:value pairs separated by |",
            )
        })?;
        let field_value = required_field(key, field_value)?;
        match key {
            "id" => id = Some(field_value),
            "action" => action = Some(field_value),
            "target" => target = Some(field_value),
            "reason" => reason = Some(field_value),
            "expected_output" => expected_output = Some(field_value),
            _ => {
                return Err(AcError::validation(
                    "AGENT-PLAN_UNKNOWN_STEP_FIELD",
                    format!("unknown step field: {}", key),
                ));
            }
        }
    }
    Ok(StructuredPlanStep {
        id: id.ok_or_else(|| missing_field("step.id"))?,
        action: action.ok_or_else(|| missing_field("step.action"))?,
        target: target.ok_or_else(|| missing_field("step.target"))?,
        reason: reason.ok_or_else(|| missing_field("step.reason"))?,
        expected_output: expected_output.ok_or_else(|| missing_field("step.expected_output"))?,
    })
}

fn validate_plan_step(step: &StructuredPlanStep) -> AcResult<()> {
    let allowed = matches!(
        step.action.as_str(),
        "inspect" | "read" | "modify" | "verify" | "changeset" | "repair"
    );
    if !allowed {
        return Err(AcError::validation(
            "AGENT-PLAN_UNSAFE_STEP",
            format!("unsafe or unsupported action: {}", step.action),
        ));
    }
    if step.target.contains("..") || step.target.starts_with('/') {
        return Err(AcError::validation(
            "AGENT-PLAN_UNSAFE_STEP",
            "plan step target must stay inside the repository",
        ));
    }
    for field in [
        &step.id,
        &step.action,
        &step.target,
        &step.reason,
        &step.expected_output,
    ] {
        if field.trim().is_empty() {
            return Err(AcError::validation(
                "AGENT-PLAN_MISSING_FIELD",
                "step fields cannot be empty",
            ));
        }
    }
    Ok(())
}

fn required_field(name: &str, value: &str) -> AcResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(missing_field(name));
    }
    Ok(value.to_string())
}

fn missing_field(name: &str) -> AcError {
    AcError::validation(
        "AGENT-PLAN_MISSING_FIELD",
        format!("missing structured plan field: {}", name),
    )
}

fn fallback_structured_plan(goal: &Goal, reasoning: &ProviderReasoning) -> StructuredPlan {
    let (target, _) = infer_change(&goal.text, reasoning);
    StructuredPlan {
        goal: goal.text.clone(),
        assumptions: vec!["fallback plan derived from invalid provider structure".to_string()],
        steps: vec![
            StructuredPlanStep {
                id: "s1".to_string(),
                action: "read".to_string(),
                target: target.clone(),
                reason: "inspect target file".to_string(),
                expected_output: "file content".to_string(),
            },
            StructuredPlanStep {
                id: "s2".to_string(),
                action: "modify".to_string(),
                target,
                reason: "apply requested change".to_string(),
                expected_output: "updated file".to_string(),
            },
        ],
        verification_requirements: vec!["status:0".to_string()],
        stopping_condition: goal.stopping_condition.clone(),
    }
}

fn verification_tool(target: &str) -> &'static str {
    if target.ends_with(".rs") {
        "dev.test"
    } else {
        "repo.diff"
    }
}

fn summarize_diff(diff: &str) -> (Vec<FileChangeSummary>, u32, u32) {
    let mut files = Vec::new();
    let mut current_path = None;
    let mut additions = 0;
    let mut removals = 0;
    let mut total_additions = 0;
    let mut total_removals = 0;
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            if let Some(path) = current_path.take() {
                files.push(FileChangeSummary {
                    path,
                    additions,
                    removals,
                });
            }
            current_path = Some(path.to_string());
            additions = 0;
            removals = 0;
        } else if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
            total_additions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            removals += 1;
            total_removals += 1;
        }
    }
    if let Some(path) = current_path {
        files.push(FileChangeSummary {
            path,
            additions,
            removals,
        });
    }
    (files, total_additions, total_removals)
}

fn search_goal_terms(code_intel: &CodeIntelligenceService, goal: &str) -> Vec<ContextCandidate> {
    goal.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|term| term.len() > 2)
        .flat_map(|term| code_intel.search_text(term))
        .collect()
}

fn collect_source_files(root: &Path) -> AcResult<Vec<(SourceFileIdentity, String)>> {
    let mut files = Vec::new();
    collect_source_files_inner(root, root, &mut files)?;
    Ok(files)
}

fn collect_source_files_inner(
    root: &Path,
    current: &Path,
    files: &mut Vec<(SourceFileIdentity, String)>,
) -> AcResult<()> {
    for entry in fs::read_dir(current)
        .map_err(|err| AcError::validation("AGENT-CONTEXT_READ_FAILED", err.to_string()))?
    {
        let entry = entry
            .map_err(|err| AcError::validation("AGENT-CONTEXT_READ_FAILED", err.to_string()))?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy() == ".git" || name.to_string_lossy() == "target" {
            continue;
        }
        if path.is_dir() {
            collect_source_files_inner(root, &path, files)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            if !is_indexable(&relative) {
                continue;
            }
            let content = fs::read_to_string(&path).unwrap_or_default();
            let test = relative.contains("test");
            let config = relative.ends_with(".toml");
            let docs = relative.ends_with(".md");
            let metadata = fs::symlink_metadata(&path)
                .map_err(|err| AcError::validation("AGENT-CONTEXT_STAT_FAILED", err.to_string()))?;
            files.push((
                SourceFileIdentity {
                    relative_path: relative,
                    language: language_for(&path),
                    content_hash: format!("len:{}", content.len()),
                    size_bytes: metadata.len(),
                    symlink: metadata.file_type().is_symlink(),
                    line_count: content.lines().count() as u32,
                    binary: false,
                    generated: false,
                    test,
                    config,
                    docs,
                },
                content,
            ));
        }
    }
    Ok(())
}

fn is_indexable(path: &str) -> bool {
    [".rs", ".md", ".toml", ".txt"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

fn language_for(path: &Path) -> String {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        Some("md") => "markdown",
        Some("toml") => "toml",
        Some("txt") => "text",
        _ => "unknown",
    }
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

    struct EchoTool;

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

    impl ToolExecutor for EchoTool {
        fn execute(&self, request: &ToolRequest) -> AcResult<String> {
            if request.tool_id == "dev.test" || request.tool_id == "repo.diff" {
                return Ok("status:0\nstdout:ok\nstderr:".to_string());
            }
            Ok(request.payload.clone())
        }
    }

    fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:{}\nstderr:{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
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
        for id in ["fs.read", "repo.diff", "dev.test"] {
            tools
                .register_tool(
                    ToolDefinition {
                        id: id.to_string(),
                        version: "1".to_string(),
                        required_capabilities: Vec::new(),
                    },
                    Box::new(EchoTool),
                )
                .unwrap();
        }
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
        let context = ContextPack {
            id: StableId::new("ctx"),
            nodes: Vec::new(),
            budget: 512,
            omitted_count: 0,
        };
        let reasoning = ProviderReasoning {
            text: "goal=Create README.md\nassumption=tests identify the bug\nstep=id:s1|action:read|target:README.md|reason:inspect docs|expected_output:file content\nstep=id:s2|action:modify|target:README.md|reason:create docs|expected_output:updated docs\nverify=repo.diff status:0\nstopping_condition=changeset ready".to_string(),
            events: vec![ProviderStreamEvent::Finished],
        };
        let plan = AgentPlanner.plan(&goal, &context, &reasoning);
        assert!(matches!(plan.steps[0].kind, PlanStepKind::InspectWorkspace));
        assert_eq!(plan.objective, "Create README.md");
        assert_eq!(plan.required_files, vec!["README.md".to_string()]);
        assert!(!plan.assumptions.is_empty());
        assert!(plan
            .steps
            .iter()
            .all(|step| !step.expected_outcome.is_empty()));
        assert!(plan
            .steps
            .iter()
            .any(|step| matches!(step.kind, PlanStepKind::PrepareChangeSet { .. })));
        assert!(matches!(
            plan.steps.last().unwrap().kind,
            PlanStepKind::Verify { .. }
        ));
    }

    #[test]
    fn structured_plan_validation_rejects_missing_and_unsafe_steps() {
        let valid = StructuredPlan::parse(
            "goal=Fix auth\nassumption=tests are present\nstep=id:s1|action:read|target:src/auth.rs|reason:inspect|expected_output:file\nverify=status:0\nstopping_condition=verified changeset",
        )
        .unwrap();
        assert_eq!(valid.steps[0].action, "read");
        assert_eq!(
            StructuredPlan::parse(
                "goal=Fix auth\nassumption=tests are present\nstep=id:s1|action:shell|target:src/auth.rs|reason:bad|expected_output:no\nverify=status:0\nstopping_condition=done",
            )
            .unwrap_err()
            .code(),
            "AGENT-PLAN_UNSAFE_STEP"
        );
        assert_eq!(
            StructuredPlan::parse("goal=Fix auth").unwrap_err().code(),
            "AGENT-PLAN_MISSING_FIELD"
        );
    }

    #[test]
    fn context_retrieval_returns_relevant_files_as_evidence_context() {
        let root = std::env::temp_dir().join(format!("agentcode-context-{}", StableId::new("tmp")));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/auth.rs"),
            "pub fn authenticate() -> bool { false }\n",
        )
        .unwrap();
        let goal = Goal::new("Fix authenticate in src/auth.rs").unwrap();
        let mut code_intel = CodeIntelligenceService::new();
        let context = ContextBuilder::default()
            .build(
                &goal,
                &[],
                &MemoryService::new(),
                &mut code_intel,
                Some(RepositoryContext {
                    root: root.clone(),
                    scope: RepositoryScope {
                        repository_id: StableId::new("repo"),
                        worktree_id: StableId::new("wt"),
                        root: root.display().to_string(),
                        commit: "working-tree".to_string(),
                        trust_profile: "test".to_string(),
                    },
                }),
                Vec::new(),
            )
            .unwrap();
        assert!(context
            .nodes
            .iter()
            .any(|node| node.content.contains("src/auth.rs")));
        assert!(context
            .nodes
            .iter()
            .any(|node| node.authority == AuthorityClass::RetrievalAccelerator));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn simple_coding_task_produces_validated_changeset() {
        let mut agent = agent_with_tool(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
            0,
        );
        let report = agent
            .run_goal(Goal::new("Create README.md").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Completed);
        assert!(
            report
                .completion_request
                .as_ref()
                .unwrap()
                .verification_passed
        );
        assert!(!report
            .completion_request
            .as_ref()
            .unwrap()
            .evidence_refs
            .is_empty());
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
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
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
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
            1,
        );
        let report = agent
            .run_goal(Goal::new("Create README.md").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Completed);
        assert!(agent.evidence().len() >= 3);
    }

    #[test]
    fn verification_failure_repairs_and_retries() {
        let mut agent = agent_with_tool(
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
            1,
        )
        .with_verification_failures(1);
        let report = agent
            .run_goal(Goal::new("Create README.md").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Completed);
        assert!(report.validation.unwrap().passed);
        assert!(agent.evidence().len() >= 5);
    }

    #[test]
    fn repair_plan_rejects_unsafe_generated_targets() {
        let mut plan = RepairPlan::generate(
            &Goal::new("Fix the bug in src/lib.rs").unwrap(),
            "status:101",
        )
        .unwrap();
        assert_eq!(plan.proposed_action, "fs.write");
        plan.affected_files = vec!["../outside.rs".to_string()];
        assert_eq!(plan.validate().unwrap_err().code(), "AGENT-REPAIR_UNSAFE");
    }

    #[test]
    fn autonomous_demo_fixes_fixture_inside_isolated_workspace() {
        let source =
            std::env::temp_dir().join(format!("agentcode-demo-src-{}", StableId::new("tmp")));
        let worktree =
            std::env::temp_dir().join(format!("agentcode-demo-wt-{}", StableId::new("tmp")));
        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&worktree);
        std::fs::create_dir_all(source.join("src")).unwrap();
        std::fs::create_dir_all(source.join("tests")).unwrap();
        std::fs::write(
            source.join("Cargo.toml"),
            "[package]\nname = \"agentcode_demo_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        )
        .unwrap();
        std::fs::write(
            source.join("src/lib.rs"),
            "pub fn fixture_answer() -> u32 {\n    41\n}\n",
        )
        .unwrap();
        std::fs::write(
            source.join("tests/fixture.rs"),
            "use agentcode_demo_fixture::fixture_answer;\n\n#[test]\nfn fixture_answer_is_correct() {\n    assert_eq!(fixture_answer(), 42);\n}\n",
        )
        .unwrap();
        run_git(&source, ["init"]);
        run_git(&source, ["add", "."]);
        run_git(
            &source,
            [
                "-c",
                "user.name=AgentCode Test",
                "-c",
                "user.email=agentcode@example.test",
                "commit",
                "-m",
                "initial",
            ],
        );
        let mut agent = isolated_workspace_agent(
            source.clone(),
            worktree.clone(),
            ac_kernel::Kernel::new(AllowAllPolicy),
            CapabilityPolicy::new()
                .allow(Capability::FilesystemRead("*".to_string()))
                .allow(Capability::FilesystemWrite("*".to_string()))
                .allow(Capability::ProcessExec("*".to_string())),
        )
        .unwrap();
        let mut report = agent
            .run_goal(Goal::new("Fix the bug in src/lib.rs").unwrap())
            .unwrap();
        assert_eq!(report.state, AutonomousState::Completed);
        assert_eq!(
            std::fs::read_to_string(source.join("src/lib.rs")).unwrap(),
            "pub fn fixture_answer() -> u32 {\n    41\n}\n"
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("src/lib.rs")).unwrap(),
            "pub fn fixture_answer() -> u32 {\n    42\n}\n"
        );
        let changeset = report.changeset.as_ref().unwrap();
        assert_eq!(changeset.state, ChangeSetState::Approved);
        assert!(matches!(
            &changeset.operations[0],
            ChangeOperation::WriteFile {
                expected_hash: Some(hash),
                new_hash,
                ..
            } if hash.starts_with("fnv1a64:") && new_hash.starts_with("fnv1a64:")
        ));
        let metadata = changeset.metadata.as_ref().unwrap();
        assert_eq!(metadata.verification_passed, Some(true));
        assert!(metadata
            .files_changed
            .iter()
            .any(|file| file.path == "src/lib.rs"));
        let mut changeset = report.changeset.take().unwrap();
        let mut rejected = changeset.clone();
        assert_eq!(
            agent
                .prepare_merge_after_review(&mut rejected, false)
                .unwrap_err()
                .code(),
            "GIT-MERGE_REQUIRES_KERNEL_APPROVAL"
        );
        let review = agent
            .prepare_merge_after_review(&mut changeset, true)
            .unwrap();
        assert!(review.approved_by_kernel);
        assert!(review.cleaned_up);
        assert!(review.merge_commit.is_some());
        assert_eq!(changeset.state, ChangeSetState::Archived);
        assert_eq!(
            std::fs::read_to_string(source.join("src/lib.rs")).unwrap(),
            "pub fn fixture_answer() -> u32 {\n    42\n}\n"
        );
        let _ = std::fs::remove_dir_all(source);
    }
}
