#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionRecord {
    pub id: StableId,
    pub source: String,
    pub version: String,
    pub trust_tier: TrustTier,
    pub declared_capabilities: BTreeSet<Capability>,
    pub granted_capabilities: BTreeSet<Capability>,
    pub registered_at: TimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillScope {
    BuiltIn,
    Project,
    Task,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillManifest {
    pub id: StableId,
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: String,
    pub scope: SkillScope,
    pub trust_tier: TrustTier,
    pub trigger_hints: Vec<String>,
    pub required_capabilities: BTreeSet<Capability>,
    pub context_cost: u32,
    pub project_id: Option<StableId>,
    pub task_id: Option<StableId>,
    pub full_instructions: String,
    pub loaded_at: Option<TimestampMillis>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillSummary {
    pub id: StableId,
    pub name: String,
    pub description: String,
    pub context_cost: u32,
    pub scope: SkillScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedSkill {
    pub manifest: SkillManifest,
    pub instructions: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillSelectionContext {
    pub language: Option<String>,
    pub framework: Option<String>,
    pub task_type: Option<String>,
    pub project_id: Option<StableId>,
    pub task_id: Option<StableId>,
}

#[derive(Default)]
pub struct SkillRegistry {
    skills: BTreeMap<StableId, SkillManifest>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, mut manifest: SkillManifest) -> AcResult<StableId> {
        validate_skill_manifest(&manifest)?;
        if matches!(manifest.scope, SkillScope::Project) && manifest.project_id.is_none() {
            return Err(AcError::validation(
                "SKILL-MISSING_PROJECT_SCOPE",
                "project-scoped skill requires project id",
            ));
        }
        if matches!(manifest.scope, SkillScope::Task) && manifest.task_id.is_none() {
            return Err(AcError::validation(
                "SKILL-MISSING_TASK_SCOPE",
                "task-scoped skill requires task id",
            ));
        }
        manifest.loaded_at = None;
        let id = manifest.id.clone();
        self.skills.insert(id.clone(), manifest);
        Ok(id)
    }

    pub fn summaries(&self, context: &SkillSelectionContext) -> Vec<SkillSummary> {
        self.skills
            .values()
            .filter(|skill| skill_matches_scope(skill, context))
            .map(|skill| SkillSummary {
                id: skill.id.clone(),
                name: skill.name.clone(),
                description: skill.description.clone(),
                context_cost: skill.context_cost,
                scope: skill.scope,
            })
            .collect()
    }

    pub fn select(&self, context: &SkillSelectionContext) -> Vec<SkillSummary> {
        let mut selected = self.summaries(context);
        selected.sort_by_key(|summary| {
            let skill = self
                .skills
                .get(&summary.id)
                .expect("summary came from registry");
            let score = skill
                .trigger_hints
                .iter()
                .filter(|hint| {
                    [
                        context.language.as_ref(),
                        context.framework.as_ref(),
                        context.task_type.as_ref(),
                    ]
                    .iter()
                    .flatten()
                    .any(|value| value.eq_ignore_ascii_case(hint))
                })
                .count();
            (usize::MAX - score, summary.context_cost)
        });
        selected
    }

    pub fn load_full(&mut self, id: &StableId) -> AcResult<LoadedSkill> {
        let skill = self
            .skills
            .get_mut(id)
            .ok_or_else(|| AcError::validation("SKILL-UNKNOWN", "skill is not registered"))?;
        skill.loaded_at = Some(TimestampMillis::now());
        Ok(LoadedSkill {
            manifest: skill.clone(),
            instructions: skill.full_instructions.clone(),
        })
    }

    pub fn import_skill_markdown(
        &mut self,
        source: impl Into<String>,
        markdown: &str,
        scope: SkillScope,
        trust_tier: TrustTier,
    ) -> AcResult<StableId> {
        let source = source.into();
        let name = markdown
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .unwrap_or("Imported Skill")
            .trim()
            .to_string();
        let description = markdown
            .lines()
            .find(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .unwrap_or("Imported portable skill")
            .trim()
            .to_string();
        self.register(SkillManifest {
            id: StableId::new("skill"),
            name,
            description,
            version: "imported-v1".to_string(),
            source,
            scope,
            trust_tier,
            trigger_hints: Vec::new(),
            required_capabilities: BTreeSet::new(),
            context_cost: markdown.len() as u32,
            project_id: None,
            task_id: None,
            full_instructions: markdown.to_string(),
            loaded_at: None,
        })
    }
}


#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: BTreeMap<StableId, ExtensionRecord>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        source: impl Into<String>,
        version: impl Into<String>,
        trust_tier: TrustTier,
        declared_capabilities: BTreeSet<Capability>,
    ) -> AcResult<StableId> {
        let source = source.into();
        let version = version.into();
        if source.trim().is_empty() || version.trim().is_empty() {
            return Err(AcError::validation(
                "SECURITY-INVALID_EXTENSION",
                "extension source and version are required",
            ));
        }
        let id = StableId::new("ext");
        self.extensions.insert(
            id.clone(),
            ExtensionRecord {
                id: id.clone(),
                source,
                version,
                trust_tier,
                declared_capabilities,
                granted_capabilities: BTreeSet::new(),
                registered_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    pub fn grant(&mut self, id: &StableId, capability: Capability) -> AcResult<()> {
        let extension = self.extensions.get_mut(id).ok_or_else(|| {
            AcError::validation("SECURITY-UNKNOWN_EXTENSION", "extension is not registered")
        })?;
        if !extension.declared_capabilities.contains(&capability) {
            return Err(AcError::policy_denied(
                "SECURITY-UNDECLARED_CAPABILITY",
                "extension cannot receive undeclared capability",
            ));
        }
        extension.granted_capabilities.insert(capability);
        Ok(())
    }

    pub fn get(&self, id: &StableId) -> Option<&ExtensionRecord> {
        self.extensions.get(id)
    }
}


fn validate_skill_manifest(manifest: &SkillManifest) -> AcResult<()> {
    if manifest.name.trim().is_empty()
        || manifest.description.trim().is_empty()
        || manifest.version.trim().is_empty()
        || manifest.source.trim().is_empty()
        || manifest.full_instructions.trim().is_empty()
    {
        return Err(AcError::validation(
            "SKILL-INVALID_MANIFEST",
            "skill manifest identity, summary, version, source and instructions are required",
        ));
    }
    Ok(())
}

fn skill_matches_scope(skill: &SkillManifest, context: &SkillSelectionContext) -> bool {
    match skill.scope {
        SkillScope::BuiltIn => true,
        SkillScope::Project => skill.project_id == context.project_id,
        SkillScope::Task => skill.task_id == context.task_id,
    }
}
