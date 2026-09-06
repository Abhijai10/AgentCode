#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementKind {
    Functional,
    Verification,
    Safety,
    Operational,
}

impl RequirementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Functional => "FUNCTIONAL",
            Self::Verification => "VERIFICATION",
            Self::Safety => "SAFETY",
            Self::Operational => "OPERATIONAL",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementStatus {
    Open,
    Implemented,
    Verified,
    Blocked,
    Superseded,
}

impl RequirementStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "OPEN",
            Self::Implemented => "IMPLEMENTED",
            Self::Verified => "VERIFIED",
            Self::Blocked => "BLOCKED",
            Self::Superseded => "SUPERSEDED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionRequirement {
    pub id: StableId,
    pub description: String,
    pub kind: RequirementKind,
    pub priority: u8,
    pub source: String,
    pub verification_strategy: String,
    pub blocking: bool,
    pub implementation_status: RequirementStatus,
    pub verification_status: RequirementStatus,
    pub evidence_refs: Vec<StableId>,
    pub linked_task_ids: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionContract {
    pub id: StableId,
    pub mission_id: StableId,
    pub revision: u32,
    pub original_goal: String,
    pub reason: String,
    pub requirements: Vec<MissionRequirement>,
    pub created_at: TimestampMillis,
}

impl MissionContract {
    pub fn extract(mission_id: StableId, original_goal: impl Into<String>) -> AcResult<Self> {
        let original_goal = original_goal.into();
        if original_goal.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-EMPTY_CONTRACT_GOAL",
                "mission contract requires the immutable original goal",
            ));
        }
        let mut requirements = Vec::new();
        for (index, part) in original_goal
            .split(|ch| ['\n', ';', '.'].contains(&ch))
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .enumerate()
        {
            requirements.push(MissionRequirement {
                id: StableId::new("req"),
                description: part.to_string(),
                kind: if part.contains("test") || part.contains("verify") {
                    RequirementKind::Verification
                } else {
                    RequirementKind::Functional
                },
                priority: (100_u8).saturating_sub(index as u8),
                source: "original_goal".to_string(),
                verification_strategy: if part.contains("test") {
                    "run specified tests".to_string()
                } else {
                    "evidence-backed verifier review".to_string()
                },
                blocking: true,
                implementation_status: RequirementStatus::Open,
                verification_status: RequirementStatus::Open,
                evidence_refs: Vec::new(),
                linked_task_ids: Vec::new(),
            });
        }
        if requirements.is_empty() {
            requirements.push(MissionRequirement {
                id: StableId::new("req"),
                description: original_goal.clone(),
                kind: RequirementKind::Functional,
                priority: 100,
                source: "original_goal".to_string(),
                verification_strategy: "evidence-backed verifier review".to_string(),
                blocking: true,
                implementation_status: RequirementStatus::Open,
                verification_status: RequirementStatus::Open,
                evidence_refs: Vec::new(),
                linked_task_ids: Vec::new(),
            });
        }
        Ok(Self {
            id: StableId::new("contract"),
            mission_id,
            revision: 1,
            original_goal,
            reason: "initial extraction".to_string(),
            requirements,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn add_requirement(
        &self,
        description: impl Into<String>,
        reason: impl Into<String>,
    ) -> AcResult<Self> {
        let description = description.into();
        if description.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-EMPTY_REQUIREMENT",
                "new requirements need a description",
            ));
        }
        let mut next = self.clone();
        next.id = StableId::new("contract");
        next.revision += 1;
        next.reason = reason.into();
        next.created_at = TimestampMillis::now();
        next.requirements.push(MissionRequirement {
            id: StableId::new("req"),
            description,
            kind: RequirementKind::Functional,
            priority: 80,
            source: format!("contract_revision:{}", next.revision),
            verification_strategy: "evidence-backed verifier review".to_string(),
            blocking: true,
            implementation_status: RequirementStatus::Open,
            verification_status: RequirementStatus::Open,
            evidence_refs: Vec::new(),
            linked_task_ids: Vec::new(),
        });
        Ok(next)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeTaskType {
    Implementation,
    Research,
    Verification,
    Repair,
}

impl RuntimeTaskType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implementation => "IMPLEMENTATION",
            Self::Research => "RESEARCH",
            Self::Verification => "VERIFICATION",
            Self::Repair => "REPAIR",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RuntimeTaskRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannerTaskProposal {
    pub proposal_id: String,
    pub title: String,
    pub description: String,
    pub dependencies: Vec<StableId>,
    pub risk: RuntimeTaskRisk,
    pub task_type: RuntimeTaskType,
    pub acceptance_criteria: Vec<String>,
    pub skills: Vec<String>,
    pub context_profile: String,
    pub target_files: Vec<String>,
    pub requirement_ids: Vec<StableId>,
    pub priority: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePlan {
    pub id: StableId,
    pub mission_id: StableId,
    pub revision: u32,
    pub tasks: Vec<WorkerTask>,
    pub proposals: Vec<PlannerTaskProposal>,
    pub supersedes: Option<StableId>,
}

impl RuntimePlan {
    pub fn from_contract(contract: &MissionContract) -> AcResult<Self> {
        let proposals = contract
            .requirements
            .iter()
            .map(|requirement| PlannerTaskProposal {
                proposal_id: format!("proposal-{}", requirement.id),
                title: requirement.description.clone(),
                description: requirement.description.clone(),
                dependencies: Vec::new(),
                risk: if requirement.blocking {
                    RuntimeTaskRisk::High
                } else {
                    RuntimeTaskRisk::Medium
                },
                task_type: match requirement.kind {
                    RequirementKind::Verification => RuntimeTaskType::Verification,
                    _ => RuntimeTaskType::Implementation,
                },
                acceptance_criteria: vec![requirement.verification_strategy.clone()],
                skills: Vec::new(),
                context_profile: "NORMAL".to_string(),
                target_files: Vec::new(),
                requirement_ids: vec![requirement.id.clone()],
                priority: requirement.priority,
            })
            .collect::<Vec<_>>();
        let tasks = proposals
            .iter()
            .map(|proposal| WorkerTask {
                id: StableId::new("task"),
                mission_id: contract.mission_id.clone(),
                title: proposal.title.clone(),
                dependencies: proposal.dependencies.clone(),
                state: TaskState::Pending,
                assigned_worker: None,
                retry_count: 0,
                max_retries: 3,
                evidence_refs: Vec::new(),
                acceptance_criteria: proposal.acceptance_criteria.iter().enumerate().map(|(index, description)| AcceptanceCriterion { id: format!("{}:criterion:{index}", proposal.proposal_id), description: description.clone(), required: true }).collect(),
            })
            .collect::<Vec<_>>();
        let plan = Self {
            id: StableId::new("plan"),
            mission_id: contract.mission_id.clone(),
            revision: contract.revision,
            tasks,
            proposals,
            supersedes: None,
        };
        validate_plan_dag(&plan)?;
        Ok(plan)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentSessionState {
    Created,
    Running,
    Cancelling,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEventKind {
    SessionCreated,
    SessionStarted,
    WorkItemProcessed(String),
    ModelEvent(ProviderStreamEvent),
    ToolResult(StableId),
    ContextPackBuilt(StableId),
    CheckpointCreated(StableId),
    StepFailed(String),
    CancellationRequested,
    SessionStopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEvent {
    pub id: StableId,
    pub session_id: StableId,
    pub kind: RuntimeEventKind,
    pub created_at: TimestampMillis,
}
