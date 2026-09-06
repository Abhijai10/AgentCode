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
    /// Optional argv hash pin from the reviewed server manifest: when set,
    /// connect() verifies the server's ACTUAL argv hashes to this value and
    /// refuses to connect on mismatch (a swapped binary must not silently
    /// ride the admitted name).
    pub expected_argv_hash: Option<String>,
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
                expected_argv_hash: None,
            },
        );
        Ok(id)
    }

    /// Pin this server's argv to a manifest hash (sha256 of the
    /// NUL-joined argv vector).  Once pinned, connect() verifies the
    /// actual argv and refuses mismatches — the supply-chain posture the
    /// manifest admission promised.
    pub fn pin_argv_hash(&mut self, id: &StableId, hash: impl Into<String>) -> AcResult<()> {
        let hash = hash.into();
        if !hash.starts_with("sha256:") || hash.len() != 7 + 64 {
            return Err(AcError::validation(
                "MCP-INVALID_ARGV_HASH",
                "argv hash must be 'sha256:' followed by 64 hex chars",
            ));
        }
        let server = self.servers.get_mut(id).ok_or_else(|| {
            AcError::validation("MCP-SERVER_NOT_FOUND", "MCP server is not registered")
        })?;
        server.expected_argv_hash = Some(hash);
        Ok(())
    }

    /// Compute the canonical argv pin for an argv vector (sha256 of the
    /// NUL-joined argv).  Manifest authors use this to write the pin.
    pub fn argv_hash_for(argv: &[String]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for arg in argv {
            hasher.update(arg.as_bytes());
            hasher.update([0]);
        }
        let digest = hasher.finalize();
        format!("sha256:{}", digest.iter().map(|b| format!("{b:02x}")).collect::<String>())
    }

    /// Verify a server's actual argv against its pin.  Unpinned servers
    /// pass (pinning is opt-in per manifest); pinned servers must match
    /// EXACTLY — any drift refuses the connection.
    pub fn verify_argv(&self, id: &StableId, argv: &[String]) -> AcResult<()> {
        let server = self.servers.get(id).ok_or_else(|| {
            AcError::validation("MCP-SERVER_NOT_FOUND", "MCP server is not registered")
        })?;
        match &server.expected_argv_hash {
            None => Ok(()),
            Some(expected) => {
                let actual = Self::argv_hash_for(argv);
                if &actual == expected {
                    Ok(())
                } else {
                    Err(AcError::validation(
                        "MCP-ARGV_HASH_MISMATCH",
                        format!(
                            "server '{}' argv hash drifted from the pinned manifest                              (expected {expected}, got {actual}); refusing to connect —                              re-review and re-pin the server",
                            server.name
                        ),
                    ))
                }
            }
        }
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
