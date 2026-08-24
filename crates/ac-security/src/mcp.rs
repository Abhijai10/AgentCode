#[derive(Clone, Debug, Eq, PartialEq)]
pub enum McpTransport {
    Stdio,
    Stream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpHealth {
    Discovered,
    Connected,
    Crashed,
    Degraded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpServerRecord {
    pub id: StableId,
    pub name: String,
    pub version: String,
    pub transport: McpTransport,
    pub trust_tier: TrustTier,
    pub health: McpHealth,
    pub restart_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpToolRecord {
    pub id: StableId,
    pub server_id: StableId,
    pub name: String,
    pub description: String,
    pub schema: String,
    pub risk: RiskClass,
    pub required_capabilities: BTreeSet<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpInvocationRecord {
    pub id: StableId,
    pub server_id: StableId,
    pub tool_id: StableId,
    pub status: SecurityDecision,
    pub output: String,
    pub evidence_ref: StableId,
    pub created_at: TimestampMillis,
}

#[derive(Default)]
pub struct McpRegistry {
    servers: BTreeMap<StableId, McpServerRecord>,
    tools: BTreeMap<StableId, McpToolRecord>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_server(
        &mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        transport: McpTransport,
    ) -> AcResult<StableId> {
        let name = name.into();
        let version = version.into();
        if name.trim().is_empty() || version.trim().is_empty() {
            return Err(AcError::validation(
                "MCP-INVALID_SERVER",
                "MCP server name and version are required",
            ));
        }
        let id = StableId::new("mcp");
        self.servers.insert(
            id.clone(),
            McpServerRecord {
                id: id.clone(),
                name,
                version,
                transport,
                trust_tier: TrustTier::Untrusted,
                health: McpHealth::Discovered,
                restart_count: 0,
            },
        );
        Ok(id)
    }

    pub fn connect(&mut self, id: &StableId) -> AcResult<()> {
        let server = self
            .servers
            .get_mut(id)
            .ok_or_else(|| AcError::validation("MCP-UNKNOWN_SERVER", "server is not registered"))?;
        server.health = McpHealth::Connected;
        Ok(())
    }

    pub fn mark_crashed(&mut self, id: &StableId) -> AcResult<()> {
        let server = self
            .servers
            .get_mut(id)
            .ok_or_else(|| AcError::validation("MCP-UNKNOWN_SERVER", "server is not registered"))?;
        server.health = McpHealth::Crashed;
        Ok(())
    }

    pub fn reconnect(&mut self, id: &StableId) -> AcResult<()> {
        let server = self
            .servers
            .get_mut(id)
            .ok_or_else(|| AcError::validation("MCP-UNKNOWN_SERVER", "server is not registered"))?;
        server.restart_count += 1;
        server.health = McpHealth::Connected;
        Ok(())
    }

    pub fn grant_trust(&mut self, id: &StableId, trust_tier: TrustTier) -> AcResult<()> {
        let server = self
            .servers
            .get_mut(id)
            .ok_or_else(|| AcError::validation("MCP-UNKNOWN_SERVER", "server is not registered"))?;
        server.trust_tier = trust_tier;
        Ok(())
    }

    pub fn register_tool(&mut self, tool: McpToolRecord) -> AcResult<StableId> {
        if !self.servers.contains_key(&tool.server_id) {
            return Err(AcError::validation(
                "MCP-UNKNOWN_SERVER",
                "MCP tool server is not registered",
            ));
        }
        let id = tool.id.clone();
        self.tools.insert(id.clone(), tool);
        Ok(id)
    }

    pub fn discover_servers(&self) -> Vec<McpServerRecord> {
        self.servers.values().cloned().collect()
    }

    pub fn discover_tools(&self, server_id: &StableId) -> Vec<McpToolRecord> {
        self.tools
            .values()
            .filter(|tool| &tool.server_id == server_id)
            .cloned()
            .collect()
    }

    pub fn expose_tools(
        &self,
        server_id: &StableId,
        role: ToolRole,
        task_policy: &CapabilityPolicy,
    ) -> Vec<McpToolRecord> {
        let Some(server) = self.servers.get(server_id) else {
            return Vec::new();
        };
        if matches!(server.trust_tier, TrustTier::Untrusted) {
            return Vec::new();
        }
        let role_policy = role_policy(role);
        self.discover_tools(server_id)
            .into_iter()
            .filter(|tool| {
                let caps = tool
                    .required_capabilities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                role_policy.evaluate(&caps) == SecurityDecision::Allow
                    && task_policy.evaluate(&caps) == SecurityDecision::Allow
            })
            .collect()
    }

    pub fn authorize_invocation(
        &self,
        tool_id: &StableId,
        context: &PermissionContext,
    ) -> AcResult<SecurityDecision> {
        let tool = self
            .tools
            .get(tool_id)
            .ok_or_else(|| AcError::validation("MCP-UNKNOWN_TOOL", "MCP tool is not registered"))?;
        let server = self
            .servers
            .get(&tool.server_id)
            .ok_or_else(|| AcError::validation("MCP-UNKNOWN_SERVER", "server is not registered"))?;
        if matches!(server.trust_tier, TrustTier::Untrusted) {
            return Ok(SecurityDecision::Deny);
        }
        Ok(context.evaluate(
            &tool
                .required_capabilities
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        ))
    }
}
