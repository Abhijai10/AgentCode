#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscussSessionState {
    Active,
    PromotedToPlan,
    PromotedToMission,
    Archived,
}

impl DiscussSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::PromotedToPlan => "promoted_to_plan",
            Self::PromotedToMission => "promoted_to_mission",
            Self::Archived => "archived",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscussMessageRole {
    User,
    Assistant,
    Researcher,
}

impl DiscussMessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Researcher => "researcher",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussSession {
    pub id: StableId,
    pub repository_id: StableId,
    pub title: String,
    pub state: DiscussSessionState,
    pub context_manifest_refs: Vec<StableId>,
    pub accepted_decision_refs: Vec<StableId>,
    pub created_at: TimestampMillis,
    pub updated_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussMessage {
    pub id: StableId,
    pub session_id: StableId,
    pub role: DiscussMessageRole,
    pub content: String,
    pub context_ref: Option<StableId>,
    pub evidence_refs: Vec<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussSource {
    pub path: String,
    pub snippet: String,
    pub evidence_note: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussAnswer {
    pub id: StableId,
    pub session_id: StableId,
    pub text: String,
    pub sources: Vec<DiscussSource>,
    pub context_pack_id: StableId,
    pub degraded_reason: Option<String>,
    pub routing_profile: TaskProfile,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussRepositoryContext {
    pub scope: RepositoryScope,
    pub files: Vec<(SourceFileIdentity, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussDecisionCandidate {
    pub id: StableId,
    pub session_id: StableId,
    pub decision: String,
    pub rationale: String,
    pub evidence_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussPlan {
    pub id: StableId,
    pub session_id: StableId,
    pub requirements: Vec<String>,
    pub tasks: Vec<String>,
    pub constraints: Vec<String>,
    pub open_questions: Vec<String>,
    pub accepted_decision_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussMissionDraft {
    pub id: StableId,
    pub plan_id: StableId,
    pub goal: Goal,
    pub inherited_decision_refs: Vec<StableId>,
    pub transcript_replay_required: bool,
}

#[derive(Default)]
pub struct DiscussMode;

impl DiscussMode {
    pub fn start_session(
        &self,
        repository_id: StableId,
        title: impl Into<String>,
    ) -> AcResult<DiscussSession> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(AcError::validation(
                "DISCUSS-EMPTY_TITLE",
                "discussion sessions require a title",
            ));
        }
        let now = TimestampMillis::now();
        Ok(DiscussSession {
            id: StableId::new("discuss"),
            repository_id,
            title,
            state: DiscussSessionState::Active,
            context_manifest_refs: Vec::new(),
            accepted_decision_refs: Vec::new(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn record_message(
        &self,
        session: &DiscussSession,
        role: DiscussMessageRole,
        content: impl Into<String>,
        context_ref: Option<StableId>,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<DiscussMessage> {
        let content = content.into();
        if content.trim().is_empty() {
            return Err(AcError::validation(
                "DISCUSS-EMPTY_MESSAGE",
                "discussion messages require content",
            ));
        }
        Ok(DiscussMessage {
            id: StableId::new("dmsg"),
            session_id: session.id.clone(),
            role,
            content,
            context_ref,
            evidence_refs,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn answer_repository_question(
        &self,
        session: &mut DiscussSession,
        question: impl Into<String>,
        code_intel: &mut CodeIntelligenceService,
        repository: DiscussRepositoryContext,
        memory: &MemoryService,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DiscussAnswer> {
        let question = question.into();
        if question.trim().is_empty() {
            return Err(AcError::validation(
                "DISCUSS-EMPTY_QUESTION",
                "repository questions require text",
            ));
        }
        let index = code_intel.index_repository(repository.scope.clone(), repository.files)?;
        let repo_map = code_intel.repo_map();
        let candidates = self.repository_sources(code_intel, &question);
        let decisions = memory
            .decisions()
            .into_iter()
            .filter(|decision| decision.scope.repository_id == session.repository_id)
            .map(|decision| decision.decision)
            .collect::<Vec<_>>();
        let context_pack = ContextEngine.build_context_pack(
            vec![
                ContextNode {
                    id: StableId::new("ctxnode"),
                    source_ref: index.id.clone(),
                    authority: AuthorityClass::RawEvidence,
                    content: repo_map.clone(),
                    token_estimate: estimate_tokens(&repo_map),
                    protected: true,
                    degraded: index.degraded,
                },
                ContextNode {
                    id: StableId::new("ctxnode"),
                    source_ref: StableId::new("memory"),
                    authority: AuthorityClass::AcceptedMemory,
                    content: decisions.join("\n"),
                    token_estimate: estimate_tokens(&decisions.join("\n")),
                    protected: false,
                    degraded: false,
                },
            ],
            4_000,
        )?;
        let source_lines = candidates
            .iter()
            .take(5)
            .map(|source| format!("{}: {}", source.path, source.snippet))
            .collect::<Vec<_>>();
        let answer_text = format!(
            "Repository-grounded answer for '{}'. Relevant files: {}. Decisions considered: {}.",
            question,
            if source_lines.is_empty() {
                "repo map only".to_string()
            } else {
                source_lines.join(" | ")
            },
            decisions.len()
        );
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "discuss-mode".to_string(),
                commit: Some(repository.scope.commit),
                worktree: Some(repository.scope.root),
                tool: Some("code-intel-context".to_string()),
            },
            format!("mem://discuss/answer/{}", StableId::new("answer")),
            content_hash(&format!("{answer_text}\n{repo_map}")),
        )?;
        session.context_manifest_refs.push(context_pack.id.clone());
        session.updated_at = TimestampMillis::now();
        Ok(DiscussAnswer {
            id: StableId::new("danswer"),
            session_id: session.id.clone(),
            text: answer_text,
            sources: candidates,
            context_pack_id: context_pack.id,
            degraded_reason: index.degraded.then(|| {
                "repository index skipped at least one unsupported or symlinked file".to_string()
            }),
            routing_profile: TaskProfile::discuss(StableId::new("task"), RoutingProfile::LocalFirst),
            evidence_ref,
        })
    }

    pub fn evaluate_read_only(&self, requested: &[Capability]) -> SecurityDecision {
        let policy = CapabilityPolicy::new()
            .allow(Capability::FilesystemRead("*".to_string()))
            .deny(Capability::FilesystemWrite("*".to_string()))
            .deny(Capability::ProcessExec("*".to_string()));
        PermissionContext {
            role: ToolRole::Planner,
            mission: policy.clone(),
            task: policy.clone(),
            sandbox: policy,
            risk: RiskClass::R1,
            approval_granted: false,
        }
        .evaluate(requested)
    }

    pub fn integrate_research(
        &self,
        session: &DiscussSession,
        query: impl Into<String>,
        result: Option<String>,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DiscussMessage> {
        let query = query.into();
        let content = result.unwrap_or_else(|| {
            format!("Research unavailable for '{query}'; discussion continues with repository context.")
        });
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "discuss-research".to_string(),
                commit: None,
                worktree: None,
                tool: Some("researcher".to_string()),
            },
            format!("mem://discuss/research/{}", StableId::new("research")),
            content_hash(&content),
        )?;
        self.record_message(
            session,
            DiscussMessageRole::Researcher,
            content,
            None,
            vec![evidence_ref],
        )
    }

    pub fn decision_candidate(
        &self,
        session: &DiscussSession,
        decision: impl Into<String>,
        rationale: impl Into<String>,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<DiscussDecisionCandidate> {
        let decision = decision.into();
        if decision.trim().is_empty() || evidence_refs.is_empty() {
            return Err(AcError::validation(
                "DISCUSS-INVALID_DECISION",
                "decision candidates require text and evidence",
            ));
        }
        Ok(DiscussDecisionCandidate {
            id: StableId::new("dcandidate"),
            session_id: session.id.clone(),
            decision,
            rationale: rationale.into(),
            evidence_refs,
        })
    }

    pub fn accept_decision(
        &self,
        session: &mut DiscussSession,
        candidate: DiscussDecisionCandidate,
        memory: &mut MemoryService,
    ) -> AcResult<StableId> {
        let decision_id = memory.record_decision(
            ac_context::MemoryScope {
                repository_id: session.repository_id.clone(),
                mission_id: None,
                task_id: None,
                branch: None,
            },
            candidate.decision,
            candidate.rationale,
            candidate.evidence_refs,
            None,
        )?;
        session.accepted_decision_refs.push(decision_id.clone());
        session.updated_at = TimestampMillis::now();
        Ok(decision_id)
    }

    pub fn promote_to_plan(
        &self,
        session: &mut DiscussSession,
        requirements: Vec<String>,
        tasks: Vec<String>,
        constraints: Vec<String>,
        open_questions: Vec<String>,
    ) -> AcResult<DiscussPlan> {
        if requirements.is_empty() || tasks.is_empty() {
            return Err(AcError::validation(
                "DISCUSS-PLAN_INCOMPLETE",
                "plan promotion requires requirements and tasks",
            ));
        }
        session.state = DiscussSessionState::PromotedToPlan;
        session.updated_at = TimestampMillis::now();
        Ok(DiscussPlan {
            id: StableId::new("dplan"),
            session_id: session.id.clone(),
            requirements,
            tasks,
            constraints,
            open_questions,
            accepted_decision_refs: session.accepted_decision_refs.clone(),
        })
    }

    pub fn promote_to_mission(
        &self,
        session: &mut DiscussSession,
        plan: &DiscussPlan,
    ) -> AcResult<DiscussMissionDraft> {
        let goal_text = format!(
            "{}\nRequirements:\n{}",
            plan.tasks.first().cloned().unwrap_or_default(),
            plan.requirements.join("\n")
        );
        session.state = DiscussSessionState::PromotedToMission;
        session.updated_at = TimestampMillis::now();
        Ok(DiscussMissionDraft {
            id: StableId::new("dmission"),
            plan_id: plan.id.clone(),
            goal: Goal::new(goal_text)?,
            inherited_decision_refs: plan.accepted_decision_refs.clone(),
            transcript_replay_required: false,
        })
    }

    fn repository_sources(
        &self,
        code_intel: &CodeIntelligenceService,
        question: &str,
    ) -> Vec<DiscussSource> {
        let terms = question
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .filter(|term| term.len() > 2)
            .collect::<Vec<_>>();
        let mut sources = Vec::new();
        for term in terms {
            for candidate in code_intel.search_text(term).into_iter().take(3) {
                if !sources.iter().any(|seen: &DiscussSource| seen.path == candidate.source_path) {
                    sources.push(DiscussSource {
                        path: candidate.source_path,
                        snippet: candidate.snippet,
                        evidence_note: candidate.evidence_note,
                    });
                }
            }
        }
        sources
    }
}

fn estimate_tokens(value: &str) -> u32 {
    value.split_whitespace().count().max(1) as u32
}
