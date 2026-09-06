#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum HookEvent {
    BeforeTool,
    AfterTool,
    AfterEdit,
    BeforeTest,
    AfterTest,
    BeforeTaskComplete,
    BeforeMissionComplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookFailurePolicy {
    Ignore,
    Warn,
    BlockOperation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookManifest {
    pub id: StableId,
    pub extension_id: StableId,
    pub event: HookEvent,
    pub priority: i32,
    pub timeout_ms: u64,
    pub idempotency_key: String,
    pub failure_policy: HookFailurePolicy,
    pub required_capabilities: BTreeSet<Capability>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookAction {
    Continue,
    Warn,
    RejectCompletion,
    Timeout,
    RecursiveDispatch,
    WorkspaceEscape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookOutcome {
    Succeeded,
    Warned,
    Blocked,
    TimedOut,
    IgnoredFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookInvocation {
    pub id: StableId,
    pub hook_id: StableId,
    pub event: HookEvent,
    pub outcome: HookOutcome,
    pub evidence_ref: Option<StableId>,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct HookRegistry {
    hooks: BTreeMap<StableId, HookManifest>,
    active_keys: BTreeSet<String>,
    completed_keys: BTreeSet<String>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, manifest: HookManifest) -> AcResult<StableId> {
        if manifest.timeout_ms == 0 || manifest.idempotency_key.trim().is_empty() {
            return Err(AcError::validation(
                "HOOK-INVALID_MANIFEST",
                "hook timeout and idempotency key are required",
            ));
        }
        let id = manifest.id.clone();
        self.hooks.insert(id.clone(), manifest);
        Ok(id)
    }

    pub fn dispatch(
        &mut self,
        event: HookEvent,
        action: HookAction,
        policy: &CapabilityPolicy,
    ) -> AcResult<Vec<HookInvocation>> {
        let mut hooks: Vec<HookManifest> = self
            .hooks
            .values()
            .filter(|hook| hook.event == event)
            .cloned()
            .collect();
        hooks.sort_by_key(|hook| hook.priority);
        let mut invocations = Vec::new();
        for hook in hooks {
            if !self.active_keys.insert(hook.idempotency_key.clone()) {
                invocations.push(hook_invocation(&hook, HookOutcome::Blocked));
                continue;
            }
            let outcome = if policy.evaluate(
                &hook
                    .required_capabilities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
            ) != SecurityDecision::Allow
            {
                HookOutcome::Blocked
            } else {
                match action {
                    HookAction::Continue => HookOutcome::Succeeded,
                    HookAction::Warn => HookOutcome::Warned,
                    HookAction::RejectCompletion
                        if matches!(
                            event,
                            HookEvent::BeforeTaskComplete | HookEvent::BeforeMissionComplete
                        ) =>
                    {
                        HookOutcome::Blocked
                    }
                    HookAction::RejectCompletion => HookOutcome::Warned,
                    HookAction::Timeout => match hook.failure_policy {
                        HookFailurePolicy::Ignore => HookOutcome::IgnoredFailure,
                        HookFailurePolicy::Warn => HookOutcome::Warned,
                        HookFailurePolicy::BlockOperation => HookOutcome::TimedOut,
                    },
                    HookAction::RecursiveDispatch | HookAction::WorkspaceEscape => {
                        HookOutcome::Blocked
                    }
                }
            };
            self.active_keys.remove(&hook.idempotency_key);
            self.completed_keys.insert(hook.idempotency_key.clone());
            invocations.push(hook_invocation(&hook, outcome));
        }
        Ok(invocations)
    }
}


fn hook_invocation(hook: &HookManifest, outcome: HookOutcome) -> HookInvocation {
    HookInvocation {
        id: StableId::new("hookrun"),
        hook_id: hook.id.clone(),
        event: hook.event,
        outcome,
        evidence_ref: None,
        created_at: TimestampMillis::now(),
    }
}
