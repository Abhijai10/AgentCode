#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopAppearance {
    System,
    Light,
    Dark,
}

impl DesktopAppearance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopView {
    Home,
    Mission,
    Agent,
    Results,
    Discuss,
    Design,
    Security,
    Settings,
    Details,
}

impl DesktopView {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Mission => "mission",
            Self::Agent => "agent",
            Self::Results => "results",
            Self::Discuss => "discuss",
            Self::Design => "design",
            Self::Security => "security",
            Self::Settings => "settings",
            Self::Details => "details",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopPreferences {
    pub appearance: DesktopAppearance,
    pub notifications_enabled: bool,
    pub completion_sound_enabled: bool,
    pub reduced_motion: bool,
    pub budget_limit_micros: Option<u64>,
}

impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            appearance: DesktopAppearance::System,
            notifications_enabled: true,
            completion_sound_enabled: true,
            reduced_motion: false,
            budget_limit_micros: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopProject {
    pub id: StableId,
    pub name: String,
    pub path: String,
    pub repository_id: StableId,
    pub last_opened_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopSession {
    pub id: StableId,
    pub active_project_id: Option<StableId>,
    pub active_mission_id: Option<StableId>,
    pub selected_view: DesktopView,
    pub window_open: bool,
    pub daemon_connected: bool,
    pub preferences: DesktopPreferences,
    pub created_at: TimestampMillis,
    pub updated_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionProgressProjection {
    pub mission_id: StableId,
    pub goal: String,
    pub kernel_state: MissionState,
    pub current_phase: String,
    pub active_task: Option<String>,
    pub waiting_reason: Option<String>,
    pub required_approval: Option<StableId>,
    pub no_action_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopActivityItem {
    pub id: StableId,
    pub summary: String,
    pub raw_event_refs: Vec<StableId>,
    pub severity: String,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopChangeGroup {
    pub task_id: StableId,
    pub file_path: String,
    pub verification_state: String,
    pub additions: u32,
    pub deletions: u32,
    pub evidence_refs: Vec<StableId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalKind {
    Kernel,
    ToolPermission,
    ChangeSet,
}

impl ApprovalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::ToolPermission => "tool_permission",
            Self::ChangeSet => "changeset",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalDecision {
    Approved,
    Denied,
}

impl ApprovalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingApproval {
    pub id: StableId,
    pub mission_id: StableId,
    pub kind: ApprovalKind,
    pub explanation: String,
    pub options: Vec<String>,
    pub recommendation: String,
    pub evidence_refs: Vec<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRecord {
    pub id: StableId,
    pub approval_id: StableId,
    pub decision: ApprovalDecision,
    pub decided_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopNotification {
    pub id: StableId,
    pub mission_id: StableId,
    pub kind: String,
    pub privacy_safe_text: String,
    pub sound: Option<String>,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopSnapshot {
    pub session: DesktopSession,
    pub recent_projects: Vec<DesktopProject>,
    pub active_missions: Vec<MissionProgressProjection>,
    pub history: Vec<MissionProgressProjection>,
    pub activity: Vec<DesktopActivityItem>,
    pub changes: Vec<DesktopChangeGroup>,
    pub pending_approvals: Vec<PendingApproval>,
    pub notifications: Vec<DesktopNotification>,
    pub details_available: bool,
}

#[derive(Default)]
pub struct DesktopExperience {
    projects: Vec<DesktopProject>,
    activity: Vec<DesktopActivityItem>,
    changes: Vec<DesktopChangeGroup>,
    approvals: Vec<PendingApproval>,
    approval_records: Vec<ApprovalRecord>,
    notifications: Vec<DesktopNotification>,
}

impl DesktopExperience {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_session(&self, preferences: DesktopPreferences) -> DesktopSession {
        let now = TimestampMillis::now();
        DesktopSession {
            id: StableId::new("desktop"),
            active_project_id: None,
            active_mission_id: None,
            selected_view: DesktopView::Home,
            window_open: true,
            daemon_connected: true,
            preferences,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn open_project(
        &mut self,
        session: &mut DesktopSession,
        path: impl Into<String>,
        name: impl Into<String>,
        repository_id: StableId,
    ) -> AcResult<DesktopProject> {
        let path = path.into();
        let name = name.into();
        if path.trim().is_empty() || name.trim().is_empty() {
            return Err(AcError::validation(
                "DESKTOP-PROJECT_INVALID",
                "project path and name are required",
            ));
        }
        let project = DesktopProject {
            id: StableId::new("project"),
            name,
            path,
            repository_id,
            last_opened_at: TimestampMillis::now(),
        };
        session.active_project_id = Some(project.id.clone());
        session.updated_at = TimestampMillis::now();
        self.projects.retain(|old| old.path != project.path);
        self.projects.push(project.clone());
        Ok(project)
    }

    pub fn compose_goal(&self, goal: impl Into<String>) -> AcResult<Goal> {
        Goal::new(goal)
    }

    pub fn attach_mission(
        &self,
        session: &mut DesktopSession,
        mission_id: StableId,
        view: DesktopView,
    ) {
        session.active_mission_id = Some(mission_id);
        session.selected_view = view;
        session.updated_at = TimestampMillis::now();
    }

    pub fn mission_projection(
        &self,
        mission_id: StableId,
        goal: impl Into<String>,
        kernel_state: MissionState,
        active_task: Option<String>,
        waiting_reason: Option<String>,
        required_approval: Option<StableId>,
    ) -> MissionProgressProjection {
        MissionProgressProjection {
            mission_id,
            goal: goal.into(),
            kernel_state,
            current_phase: match kernel_state {
                MissionState::Created => "created",
                MissionState::Active => "running",
                MissionState::Completed => "completed",
                MissionState::Cancelled => "cancelled",
                MissionState::Failed => "failed",
            }
            .to_string(),
            active_task,
            waiting_reason,
            required_approval,
            no_action_required: matches!(kernel_state, MissionState::Completed),
        }
    }

    pub fn compress_activity(
        &mut self,
        summary: impl Into<String>,
        raw_event_refs: Vec<StableId>,
        severity: impl Into<String>,
    ) -> AcResult<DesktopActivityItem> {
        let summary = summary.into();
        if summary.trim().is_empty() || raw_event_refs.is_empty() {
            return Err(AcError::validation(
                "DESKTOP-ACTIVITY_INVALID",
                "activity requires a summary and raw event refs",
            ));
        }
        let item = DesktopActivityItem {
            id: StableId::new("activity"),
            summary,
            raw_event_refs,
            severity: severity.into(),
            created_at: TimestampMillis::now(),
        };
        self.activity.push(item.clone());
        Ok(item)
    }

    pub fn record_change_group(
        &mut self,
        task_id: StableId,
        file_path: impl Into<String>,
        verification_state: impl Into<String>,
        additions: u32,
        deletions: u32,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<DesktopChangeGroup> {
        let file_path = file_path.into();
        if file_path.trim().is_empty() {
            return Err(AcError::validation(
                "DESKTOP-CHANGE_INVALID",
                "change groups require a file path",
            ));
        }
        let group = DesktopChangeGroup {
            task_id,
            file_path,
            verification_state: verification_state.into(),
            additions,
            deletions,
            evidence_refs,
        };
        self.changes.push(group.clone());
        Ok(group)
    }

    pub fn request_approval(
        &mut self,
        mission_id: StableId,
        kind: ApprovalKind,
        explanation: impl Into<String>,
        options: Vec<String>,
        recommendation: impl Into<String>,
        evidence_refs: Vec<StableId>,
    ) -> AcResult<PendingApproval> {
        let explanation = explanation.into();
        if explanation.trim().is_empty() || options.is_empty() {
            return Err(AcError::validation(
                "DESKTOP-APPROVAL_INVALID",
                "approval requests require explanation and options",
            ));
        }
        let approval = PendingApproval {
            id: StableId::new("approval"),
            mission_id,
            kind,
            explanation,
            options,
            recommendation: recommendation.into(),
            evidence_refs,
        };
        self.approvals.push(approval.clone());
        Ok(approval)
    }

    pub fn decide_approval(
        &mut self,
        approval_id: &StableId,
        decision: ApprovalDecision,
    ) -> AcResult<ApprovalRecord> {
        if !self.approvals.iter().any(|approval| &approval.id == approval_id) {
            return Err(AcError::validation(
                "DESKTOP-APPROVAL_UNKNOWN",
                "approval request not found",
            ));
        }
        let record = ApprovalRecord {
            id: StableId::new("approval-record"),
            approval_id: approval_id.clone(),
            decision,
            decided_at: TimestampMillis::now(),
        };
        self.approval_records.push(record.clone());
        self.approvals.retain(|approval| &approval.id != approval_id);
        Ok(record)
    }

    pub fn close_window(&self, session: &mut DesktopSession) {
        session.window_open = false;
        session.updated_at = TimestampMillis::now();
    }

    pub fn restore_window(&self, session: &mut DesktopSession) {
        session.window_open = true;
        session.daemon_connected = true;
        session.updated_at = TimestampMillis::now();
    }

    pub fn notify(
        &mut self,
        mission_id: StableId,
        kind: impl Into<String>,
        privacy_safe_text: impl Into<String>,
        preferences: &DesktopPreferences,
    ) -> Option<DesktopNotification> {
        if !preferences.notifications_enabled {
            return None;
        }
        let kind = kind.into();
        if kind == "provider_failover" {
            return None;
        }
        let sound = match kind.as_str() {
            "mission_complete" if preferences.completion_sound_enabled => {
                Some("completion-subtle".to_string())
            }
            "needs_user" => Some("attention".to_string()),
            _ => None,
        };
        let notification = DesktopNotification {
            id: StableId::new("notification"),
            mission_id,
            kind,
            privacy_safe_text: privacy_safe_text.into(),
            sound,
            created_at: TimestampMillis::now(),
        };
        self.notifications.push(notification.clone());
        Some(notification)
    }

    pub fn snapshot(
        &self,
        session: DesktopSession,
        active_missions: Vec<MissionProgressProjection>,
        history: Vec<MissionProgressProjection>,
    ) -> DesktopSnapshot {
        DesktopSnapshot {
            session,
            recent_projects: self.projects.clone(),
            active_missions,
            history,
            activity: self.activity.clone(),
            changes: self.changes.clone(),
            pending_approvals: self.approvals.clone(),
            notifications: self.notifications.clone(),
            details_available: true,
        }
    }

    pub fn approval_records(&self) -> &[ApprovalRecord] {
        &self.approval_records
    }
}
