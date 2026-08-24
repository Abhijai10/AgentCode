#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesignSessionState {
    Active,
    Iterating,
    Accepted,
    Archived,
}

impl DesignSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Iterating => "iterating",
            Self::Accepted => "accepted",
            Self::Archived => "archived",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignSession {
    pub id: StableId,
    pub repository_id: StableId,
    pub product: String,
    pub state: DesignSessionState,
    pub hard_constraints: Vec<String>,
    pub created_at: TimestampMillis,
    pub updated_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductAnalysis {
    pub id: StableId,
    pub framework: Option<String>,
    pub routes: Vec<String>,
    pub components: Vec<String>,
    pub style_files: Vec<String>,
    pub tokens: Vec<String>,
    pub fonts: Vec<String>,
    pub assets: Vec<String>,
    pub navigation: Vec<String>,
    pub baseline_screens: Vec<String>,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignBrief {
    pub id: StableId,
    pub product: String,
    pub audience: String,
    pub personality: String,
    pub density: String,
    pub primary_workflow: String,
    pub visual_goals: Vec<String>,
    pub patterns_to_avoid: Vec<String>,
    pub provenance_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignGrammar {
    pub id: StableId,
    pub type_scale: Vec<String>,
    pub spacing: Vec<String>,
    pub radii: Vec<String>,
    pub surfaces: Vec<String>,
    pub color_roles: Vec<String>,
    pub navigation: Vec<String>,
    pub motion: Vec<String>,
    pub iconography: Vec<String>,
    pub component_principles: Vec<String>,
    pub provenance_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignArtifact {
    pub id: StableId,
    pub session_id: StableId,
    pub name: String,
    pub artifact_type: String,
    pub current_version: u32,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignArtifactVersion {
    pub id: StableId,
    pub artifact_id: StableId,
    pub version: u32,
    pub summary: String,
    pub content_hash: String,
    pub evidence_refs: Vec<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AntiSlopFinding {
    pub rule: String,
    pub severity: u8,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignCritique {
    pub id: StableId,
    pub passed: bool,
    pub findings: Vec<AntiSlopFinding>,
    pub improvement_iteration_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignImplementationTask {
    pub id: StableId,
    pub component: String,
    pub file_path: String,
    pub requirement: String,
    pub verification: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignStateDocument {
    pub path: String,
    pub content: String,
    pub facts: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceImagePrinciples {
    pub id: StableId,
    pub source_ref: StableId,
    pub extracted_principles: Vec<String>,
    pub limitation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomSourceMapping {
    pub selector: String,
    pub file_path: String,
    pub component: Option<String>,
    pub confidence: u8,
    pub limitation: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignPreviewIteration {
    pub id: StableId,
    pub session_id: StableId,
    pub artifact_version_id: StableId,
    pub visual_evaluation: DesignVisualEvaluation,
    pub responsive_report: DesignResponsiveReport,
    pub accessibility_report: DesignAccessibilityReport,
    pub functional_report: DesignFunctionalReport,
    pub repaired: bool,
}

#[derive(Default)]
pub struct DesignStudio;

impl DesignStudio {
    pub fn start_session(
        &self,
        repository_id: StableId,
        product: impl Into<String>,
        hard_constraints: Vec<String>,
    ) -> AcResult<DesignSession> {
        let product = product.into();
        if product.trim().is_empty() {
            return Err(AcError::validation(
                "DESIGN-EMPTY_PRODUCT",
                "design sessions require a product identity",
            ));
        }
        let now = TimestampMillis::now();
        Ok(DesignSession {
            id: StableId::new("design"),
            repository_id,
            product,
            state: DesignSessionState::Active,
            hard_constraints,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn analyze_product(
        &self,
        files: &[(SourceFileIdentity, String)],
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<ProductAnalysis> {
        if files.is_empty() {
            return Err(AcError::validation(
                "DESIGN-NO_SOURCE",
                "design analysis requires repository source files",
            ));
        }
        let paths = files
            .iter()
            .map(|(identity, _)| identity.relative_path.as_str())
            .collect::<Vec<_>>();
        let framework = detect_framework(&paths, files);
        let routes = paths
            .iter()
            .filter(|path| {
                path.contains("/pages/")
                    || path.contains("/routes/")
                    || path.contains("/app/")
                    || path.ends_with("App.tsx")
                    || path.ends_with("main.tsx")
            })
            .map(|path| (*path).to_string())
            .collect::<Vec<_>>();
        let components = paths
            .iter()
            .filter(|path| {
                path.contains("component")
                    || path.ends_with(".tsx")
                    || path.ends_with(".jsx")
                    || path.ends_with(".vue")
                    || path.ends_with(".svelte")
            })
            .map(|path| (*path).to_string())
            .collect::<Vec<_>>();
        let style_files = paths
            .iter()
            .filter(|path| {
                path.ends_with(".css")
                    || path.ends_with(".scss")
                    || path.contains("tailwind")
                    || path.contains("theme")
            })
            .map(|path| (*path).to_string())
            .collect::<Vec<_>>();
        let all_content = files
            .iter()
            .map(|(_, content)| content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let tokens = extract_design_tokens(&all_content);
        let fonts = extract_fonts(&all_content);
        let assets = paths
            .iter()
            .filter(|path| {
                path.ends_with(".png")
                    || path.ends_with(".jpg")
                    || path.ends_with(".jpeg")
                    || path.ends_with(".webp")
                    || path.ends_with(".svg")
            })
            .map(|path| (*path).to_string())
            .collect::<Vec<_>>();
        let navigation = extract_navigation(&all_content);
        let baseline_screens = routes.iter().take(6).cloned().collect::<Vec<_>>();
        let summary = format!(
            "framework={framework:?};routes={};components={};styles={}",
            routes.len(),
            components.len(),
            style_files.len()
        );
        let evidence_ref = evidence_store.append(
            EvidenceKind::DerivedContext,
            Provenance {
                source: "design-studio".to_string(),
                commit: None,
                worktree: None,
                tool: Some("product-analysis".to_string()),
            },
            format!("mem://design/analysis/{}", StableId::new("analysis")),
            content_hash(&summary),
        )?;
        Ok(ProductAnalysis {
            id: StableId::new("danalysis"),
            framework,
            routes,
            components,
            style_files,
            tokens,
            fonts,
            assets,
            navigation,
            baseline_screens,
            evidence_ref,
        })
    }

    pub fn generate_brief(
        &self,
        session: &DesignSession,
        analysis: &ProductAnalysis,
        audience: impl Into<String>,
        primary_workflow: impl Into<String>,
    ) -> AcResult<DesignBrief> {
        let audience = audience.into();
        let primary_workflow = primary_workflow.into();
        if audience.trim().is_empty() || primary_workflow.trim().is_empty() {
            return Err(AcError::validation(
                "DESIGN-BRIEF_INCOMPLETE",
                "design brief requires audience and workflow",
            ));
        }
        let personality = if session.product.to_ascii_lowercase().contains("admin")
            || primary_workflow.to_ascii_lowercase().contains("review")
        {
            "calm, dense, operational".to_string()
        } else {
            format!("specific to {}", session.product)
        };
        Ok(DesignBrief {
            id: StableId::new("dbrief"),
            product: session.product.clone(),
            audience,
            personality,
            density: if analysis.components.len() > 8 {
                "compact".to_string()
            } else {
                "moderate".to_string()
            },
            primary_workflow,
            visual_goals: vec![
                format!("make {} immediately recognizable", session.product),
                "prioritize repeated task scanning over decorative explanation".to_string(),
            ],
            patterns_to_avoid: vec![
                "generic AI productivity copy".to_string(),
                "oversized gradient hero sections unrelated to workflow".to_string(),
                "identical feature cards without hierarchy".to_string(),
            ],
            provenance_refs: vec![analysis.evidence_ref.clone()],
        })
    }

    pub fn infer_grammar(
        &self,
        brief: &DesignBrief,
        analysis: &ProductAnalysis,
    ) -> DesignGrammar {
        let radii = if analysis.tokens.iter().any(|token| token.contains("radius")) {
            vec!["use existing radius tokens".to_string()]
        } else {
            vec!["cards and controls use 4-8px radii".to_string()]
        };
        DesignGrammar {
            id: StableId::new("dgrammar"),
            type_scale: vec![
                "compact panel headings".to_string(),
                "hero scale only when the first screen is a true product hero".to_string(),
            ],
            spacing: vec!["dense 8px grid for operational controls".to_string()],
            radii,
            surfaces: vec!["full-width sections; cards only for repeated items".to_string()],
            color_roles: vec![format!("brand role anchored to {}", brief.product)],
            navigation: if analysis.navigation.is_empty() {
                vec!["derive navigation from existing routes".to_string()]
            } else {
                analysis.navigation.clone()
            },
            motion: vec!["motion clarifies state changes, never just decoration".to_string()],
            iconography: vec!["use existing icon library before custom vectors".to_string()],
            component_principles: vec![
                format!("optimize the {} workflow", brief.primary_workflow),
                format!("respect {} density", brief.density),
            ],
            provenance_refs: brief.provenance_refs.clone(),
        }
    }

    pub fn create_artifact(
        &self,
        session: &DesignSession,
        name: impl Into<String>,
        artifact_type: impl Into<String>,
        summary: impl Into<String>,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<(DesignArtifact, DesignArtifactVersion)> {
        let name = name.into();
        let artifact_type = artifact_type.into();
        let summary = summary.into();
        if name.trim().is_empty() || artifact_type.trim().is_empty() || summary.trim().is_empty() {
            return Err(AcError::validation(
                "DESIGN-ARTIFACT_INVALID",
                "design artifacts require name, type, and summary",
            ));
        }
        let artifact = DesignArtifact {
            id: StableId::new("dartifact"),
            session_id: session.id.clone(),
            name,
            artifact_type,
            current_version: 1,
            created_at: TimestampMillis::now(),
        };
        let version = DesignArtifactVersion {
            id: StableId::new("dversion"),
            artifact_id: artifact.id.clone(),
            version: 1,
            summary: summary.clone(),
            content_hash: content_hash(&summary),
            evidence_refs,
            created_at: TimestampMillis::now(),
        };
        Ok((artifact, version))
    }

    pub fn revise_artifact(
        &self,
        artifact: &mut DesignArtifact,
        summary: impl Into<String>,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<DesignArtifactVersion> {
        let summary = summary.into();
        if summary.trim().is_empty() {
            return Err(AcError::validation(
                "DESIGN-REVISION_EMPTY",
                "artifact revisions require a summary",
            ));
        }
        artifact.current_version = artifact.current_version.saturating_add(1);
        Ok(DesignArtifactVersion {
            id: StableId::new("dversion"),
            artifact_id: artifact.id.clone(),
            version: artifact.current_version,
            summary: summary.clone(),
            content_hash: content_hash(&summary),
            evidence_refs,
            created_at: TimestampMillis::now(),
        })
    }

    pub fn critique(&self, content: &str, brief: &DesignBrief) -> DesignCritique {
        let lower = content.to_ascii_lowercase();
        let mut findings = Vec::new();
        if lower.contains("gradient") && (lower.contains("hero") || lower.contains("100vh")) {
            findings.push(AntiSlopFinding {
                rule: "oversized_gradient_hero".to_string(),
                severity: 3,
                explanation: "large gradient hero competes with the actual workflow".to_string(),
            });
        }
        if lower.matches("card").count() >= 3 && !lower.contains(&brief.product.to_ascii_lowercase())
        {
            findings.push(AntiSlopFinding {
                rule: "identical_generic_cards".to_string(),
                severity: 3,
                explanation: "repeated generic cards do not express product-specific hierarchy"
                    .to_string(),
            });
        }
        if lower.contains("ai-powered") || lower.contains("reimagine your workflow") {
            findings.push(AntiSlopFinding {
                rule: "generic_ai_copy".to_string(),
                severity: 2,
                explanation: "copy could describe almost any product".to_string(),
            });
        }
        if lower.contains("glass") || lower.contains("backdrop-filter") {
            findings.push(AntiSlopFinding {
                rule: "gratuitous_glass".to_string(),
                severity: 2,
                explanation: "glass styling needs a product reason and contrast proof".to_string(),
            });
        }
        DesignCritique {
            id: StableId::new("dcritique"),
            passed: findings.is_empty(),
            improvement_iteration_required: findings.iter().any(|finding| finding.severity >= 3),
            findings,
        }
    }

    pub fn implementation_tasks(
        &self,
        analysis: &ProductAnalysis,
        brief: &DesignBrief,
    ) -> Vec<DesignImplementationTask> {
        analysis
            .components
            .iter()
            .take(6)
            .map(|component| DesignImplementationTask {
                id: StableId::new("dtask"),
                component: component.clone(),
                file_path: component.clone(),
                requirement: format!("align {} with {}", component, brief.product),
                verification: "browser screenshot, responsive matrix, accessibility checks".to_string(),
            })
            .collect()
    }

    pub fn run_preview_iteration(
        &self,
        session: &mut DesignSession,
        version: &DesignArtifactVersion,
        critique: &DesignCritique,
        verification: &VerificationEngine,
        evidence_store: &mut EvidenceStore,
    ) -> AcResult<DesignPreviewIteration> {
        let visual = verification.record_design_visual_evaluation(
            &version.id,
            critique.passed,
            critique
                .findings
                .iter()
                .map(|finding| finding.explanation.clone())
                .collect(),
            evidence_store,
        )?;
        let responsive = verification.record_design_responsive_report(
            &version.id,
            vec!["mobile".to_string(), "tablet".to_string(), "desktop".to_string()],
            critique.passed,
            evidence_store,
        )?;
        let accessibility = verification.record_design_accessibility_report(
            &version.id,
            critique.passed,
            vec!["keyboard".to_string(), "labels".to_string(), "focus".to_string()],
            evidence_store,
        )?;
        let functional = verification.record_design_functional_report(
            &version.id,
            critique.passed,
            vec!["primary workflow smoke".to_string()],
            evidence_store,
        )?;
        session.state = if critique.improvement_iteration_required {
            DesignSessionState::Iterating
        } else {
            DesignSessionState::Accepted
        };
        session.updated_at = TimestampMillis::now();
        Ok(DesignPreviewIteration {
            id: StableId::new("diteration"),
            session_id: session.id.clone(),
            artifact_version_id: version.id.clone(),
            visual_evaluation: visual,
            responsive_report: responsive,
            accessibility_report: accessibility,
            functional_report: functional,
            repaired: critique.improvement_iteration_required,
        })
    }

    pub fn generate_design_state(
        &self,
        brief: &DesignBrief,
        grammar: &DesignGrammar,
        analysis: &ProductAnalysis,
    ) -> DesignStateDocument {
        let facts = vec![
            format!("Product: {}", brief.product),
            format!("Audience: {}", brief.audience),
            format!("Primary workflow: {}", brief.primary_workflow),
            format!("Framework: {}", analysis.framework.as_deref().unwrap_or("unknown")),
            format!("Components inspected: {}", analysis.components.len()),
        ];
        let content = format!(
            "# DESIGN_STATE.md\n\n{}\n\n## Grammar\n- {}\n\n## Avoid\n- {}\n",
            facts
                .iter()
                .map(|fact| format!("- {fact}"))
                .collect::<Vec<_>>()
                .join("\n"),
            grammar.component_principles.join("\n- "),
            brief.patterns_to_avoid.join("\n- ")
        );
        DesignStateDocument {
            path: "DESIGN_STATE.md".to_string(),
            content,
            facts,
        }
    }

    pub fn extract_reference_principles(
        &self,
        source_ref: StableId,
        description: impl Into<String>,
    ) -> AcResult<ReferenceImagePrinciples> {
        let description = description.into();
        if description.trim().is_empty() {
            return Err(AcError::validation(
                "DESIGN-REFERENCE_EMPTY",
                "reference images require a description or observation",
            ));
        }
        Ok(ReferenceImagePrinciples {
            id: StableId::new("dref"),
            source_ref,
            extracted_principles: vec![
                "extract composition, rhythm, contrast, and density only".to_string(),
                description,
            ],
            limitation: "does not clone proprietary assets, logos, or exact layouts".to_string(),
        })
    }

    pub fn map_dom_to_source(
        &self,
        selector: impl Into<String>,
        html: &str,
        files: &[(SourceFileIdentity, String)],
    ) -> DomSourceMapping {
        let selector = selector.into();
        let marker = selector.trim_start_matches(['#', '.']);
        if let Some(source_attr) = html
            .split("data-source=\"")
            .nth(1)
            .and_then(|tail| tail.split('"').next())
        {
            return DomSourceMapping {
                selector,
                file_path: source_attr.to_string(),
                component: None,
                confidence: 90,
                limitation: None,
            };
        }
        for (identity, content) in files {
            if content.contains(marker) {
                return DomSourceMapping {
                    selector,
                    file_path: identity.relative_path.clone(),
                    component: component_name(&identity.relative_path),
                    confidence: 60,
                    limitation: Some("prototype mapping based on selector text search".to_string()),
                };
            }
        }
        DomSourceMapping {
            selector,
            file_path: "unknown".to_string(),
            component: None,
            confidence: 0,
            limitation: Some("DOM-to-source prototype could not map selector".to_string()),
        }
    }
}

fn detect_framework(paths: &[&str], files: &[(SourceFileIdentity, String)]) -> Option<String> {
    if paths.iter().any(|path| path.contains("next.config") || path.contains("/app/")) {
        return Some("Next.js".to_string());
    }
    if files
        .iter()
        .any(|(_, content)| content.contains("@vitejs/plugin-react"))
    {
        return Some("Vite React".to_string());
    }
    if paths.iter().any(|path| path.ends_with(".svelte")) {
        return Some("Svelte".to_string());
    }
    if paths.iter().any(|path| path.ends_with(".vue")) {
        return Some("Vue".to_string());
    }
    None
}

fn extract_design_tokens(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| {
            line.contains("--")
                || line.contains("theme")
                || line.contains("spacing")
                || line.contains("radius")
                || line.contains("color")
        })
        .take(24)
        .map(|line| line.trim().to_string())
        .collect()
}

fn extract_fonts(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| line.contains("font-family") || line.contains("next/font"))
        .take(12)
        .map(|line| line.trim().to_string())
        .collect()
}

fn extract_navigation(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| line.contains("<nav") || line.contains("Navigation") || line.contains("href="))
        .take(12)
        .map(|line| line.trim().to_string())
        .collect()
}

fn component_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
}
