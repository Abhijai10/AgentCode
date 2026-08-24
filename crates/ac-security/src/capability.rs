#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Capability {
    FilesystemRead(String),
    FilesystemWrite(String),
    ProcessExec(String),
    Network(String),
    SecretRead(String),
    BrowserAutomation,
    SecurityScan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustTier {
    BuiltIn,
    Project,
    UserInstalled,
    Untrusted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecurityDecision {
    Allow,
    Deny,
    RequireApproval,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RiskClass {
    R0,
    R1,
    R2,
    R3,
    R4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolRole {
    Planner,
    Worker,
    Researcher,
    Verifier,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionContext {
    pub role: ToolRole,
    pub mission: CapabilityPolicy,
    pub task: CapabilityPolicy,
    pub sandbox: CapabilityPolicy,
    pub risk: RiskClass,
    pub approval_granted: bool,
}

impl PermissionContext {
    pub fn standard_worker(policy: CapabilityPolicy) -> Self {
        Self {
            role: ToolRole::Worker,
            mission: policy.clone(),
            task: policy.clone(),
            sandbox: policy,
            risk: RiskClass::R1,
            approval_granted: false,
        }
    }

    pub fn evaluate(&self, requested: &[Capability]) -> SecurityDecision {
        let role = role_policy(self.role).evaluate(requested);
        let decisions = [
            role,
            self.mission.evaluate(requested),
            self.task.evaluate(requested),
            self.sandbox.evaluate(requested),
        ];
        if decisions.contains(&SecurityDecision::Deny) {
            return SecurityDecision::Deny;
        }
        if self.risk >= RiskClass::R3 && !self.approval_granted {
            return SecurityDecision::RequireApproval;
        }
        if decisions.contains(&SecurityDecision::RequireApproval) {
            SecurityDecision::RequireApproval
        } else {
            SecurityDecision::Allow
        }
    }
}

pub fn role_policy(role: ToolRole) -> CapabilityPolicy {
    match role {
        ToolRole::Planner => {
            CapabilityPolicy::new().allow(Capability::FilesystemRead("*".to_string()))
        }
        ToolRole::Worker => CapabilityPolicy::new()
            .allow(Capability::FilesystemRead("*".to_string()))
            .allow(Capability::FilesystemWrite("*".to_string()))
            .allow(Capability::ProcessExec("*".to_string())),
        ToolRole::Researcher => CapabilityPolicy::new()
            .allow(Capability::FilesystemRead("*".to_string()))
            .allow(Capability::Network("*".to_string())),
        ToolRole::Verifier => CapabilityPolicy::new()
            .allow(Capability::FilesystemRead("*".to_string()))
            .allow(Capability::ProcessExec("*".to_string())),
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapabilityPolicy {
    denied: BTreeSet<Capability>,
    allowed: BTreeSet<Capability>,
}

impl CapabilityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deny(mut self, capability: Capability) -> Self {
        self.denied.insert(capability);
        self
    }

    pub fn allow(mut self, capability: Capability) -> Self {
        self.allowed.insert(capability);
        self
    }

    pub fn evaluate(&self, requested: &[Capability]) -> SecurityDecision {
        if requested
            .iter()
            .any(|cap| matches_capability(&self.denied, cap))
        {
            return SecurityDecision::Deny;
        }
        if requested
            .iter()
            .all(|cap| matches_capability(&self.allowed, cap))
        {
            SecurityDecision::Allow
        } else {
            SecurityDecision::RequireApproval
        }
    }
}

fn matches_capability(set: &BTreeSet<Capability>, requested: &Capability) -> bool {
    if set.contains(requested) {
        return true;
    }
    match requested {
        Capability::FilesystemRead(_) => set.contains(&Capability::FilesystemRead("*".to_string())),
        Capability::FilesystemWrite(_) => {
            set.contains(&Capability::FilesystemWrite("*".to_string()))
        }
        Capability::ProcessExec(_) => set.contains(&Capability::ProcessExec("*".to_string())),
        Capability::Network(_) => set.contains(&Capability::Network("*".to_string())),
        Capability::SecretRead(_) => set.contains(&Capability::SecretRead("*".to_string())),
        Capability::BrowserAutomation | Capability::SecurityScan => false,
    }
}
