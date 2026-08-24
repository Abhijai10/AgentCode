use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderCapability {
    Chat,
    ToolCalls,
    Streaming,
    LocalModel,
    Vision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrivacyClass {
    LocalOnly,
    Standard,
    ExternalAllowed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoutingProfile {
    FreeOnly,
    FreeFirst,
    LocalFirst,
    QualityFirst,
    PaidAllowed,
    Offline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedProviderFailure {
    RateLimit,
    QuotaExhausted,
    AuthFailed,
    Timeout,
    ModelUnavailable,
    ProviderUnavailable,
    ContextLimit,
    BadResponse,
    ToolCallFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderConnection {
    pub id: StableId,
    pub provider_id: StableId,
    pub account_ref: String,
    pub credential_ref: Option<String>,
    pub quota_domain: String,
    pub endpoint_ref: String,
    pub enabled: bool,
    pub paid: bool,
    pub local: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelIdentity {
    pub id: StableId,
    pub family: String,
    pub context_window: u32,
    pub coding_score: u8,
    pub reasoning_score: u8,
    pub vision: bool,
    pub tool_use: bool,
    pub structured_output: bool,
    pub input_cost_micros: u32,
    pub output_cost_micros: u32,
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRoute {
    pub id: StableId,
    pub model_identity_id: StableId,
    pub model_id: StableId,
    pub provider_id: StableId,
    pub connection_id: StableId,
    pub model_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProfile {
    pub task_id: StableId,
    pub role: String,
    pub task_type: String,
    pub complexity: u8,
    pub risk: u8,
    pub required_context: u32,
    pub requires_vision: bool,
    pub requires_tool_use: bool,
    pub requires_structured_output: bool,
    pub privacy: PrivacyClass,
    pub routing_profile: RoutingProfile,
    pub max_input_cost_micros: Option<u32>,
}

impl TaskProfile {
    pub fn coding(task_id: StableId, routing_profile: RoutingProfile) -> Self {
        Self {
            task_id,
            role: "implementation".to_string(),
            task_type: "coding".to_string(),
            complexity: 5,
            risk: 5,
            required_context: 8192,
            requires_vision: false,
            requires_tool_use: false,
            requires_structured_output: true,
            privacy: PrivacyClass::Standard,
            routing_profile,
            max_input_cost_micros: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteCandidate {
    pub route_id: StableId,
    pub model_identity_id: StableId,
    pub provider_id: StableId,
    pub connection_id: StableId,
    pub model_name: String,
    pub score: i32,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingDecision {
    pub id: StableId,
    pub task_id: StableId,
    pub candidates: Vec<RouteCandidate>,
    pub selected: Option<RouteCandidate>,
    pub rejected: Vec<String>,
    pub fallback_reason: Option<String>,
    pub latency_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost_micros: u64,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteExecution {
    pub events: Vec<ProviderStreamEvent>,
    pub decision: RoutingDecision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthObservation {
    pub connection_id: StableId,
    pub failure: Option<NormalizedProviderFailure>,
    pub latency_ms: u64,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreaker {
    pub connection_id: StableId,
    pub transient_failures: u32,
    pub cooldown_until_ms: Option<u128>,
}

impl CircuitBreaker {
    fn healthy(connection_id: StableId) -> Self {
        Self {
            connection_id,
            transient_failures: 0,
            cooldown_until_ms: None,
        }
    }

    fn is_open(&self) -> bool {
        self.cooldown_until_ms
            .map(|until| TimestampMillis::now().as_millis() < until)
            .unwrap_or(false)
    }

    fn record_success(&mut self) {
        self.transient_failures = 0;
        self.cooldown_until_ms = None;
    }

    fn record_failure(&mut self, failure: NormalizedProviderFailure) {
        if matches!(
            failure,
            NormalizedProviderFailure::RateLimit
                | NormalizedProviderFailure::Timeout
                | NormalizedProviderFailure::ProviderUnavailable
        ) {
            self.transient_failures += 1;
            self.cooldown_until_ms = Some(TimestampMillis::now().as_millis() + 10 * 60 * 1000);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderDefinition {
    pub id: StableId,
    pub name: String,
    pub credential_ref: Option<String>,
    pub capabilities: Vec<ProviderCapability>,
    pub quota_domain: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCapability {
    pub id: StableId,
    pub provider_id: StableId,
    pub model_name: String,
    pub capabilities: Vec<ProviderCapability>,
    pub context_window: u32,
    pub discovered_at: TimestampMillis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedInferenceRequest {
    pub model_id: StableId,
    pub prompt: String,
    pub required: Vec<ProviderCapability>,
    pub max_output_tokens: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderStreamEvent {
    Delta(String),
    ToolCall {
        name: String,
        payload: String,
    },
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    Finished,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredAgentResponse {
    pub plan: Vec<String>,
    pub assumptions: Vec<String>,
    pub actions: Vec<String>,
    pub verification_requirements: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderResponseFormat {
    Structured,
    LegacyFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedProviderResponse {
    pub response: StructuredAgentResponse,
    pub format: ProviderResponseFormat,
}

impl StructuredAgentResponse {
    pub fn validate(&self) -> AcResult<()> {
        if self.plan.is_empty()
            || self.actions.is_empty()
            || self.verification_requirements.is_empty()
            || self.plan.iter().any(|item| item.trim().is_empty())
            || self.actions.iter().any(|item| item.trim().is_empty())
            || self
                .verification_requirements
                .iter()
                .any(|item| item.trim().is_empty())
        {
            return Err(AcError::validation(
                "PROVIDER-INVALID_STRUCTURED_RESPONSE",
                "structured provider output requires plan, actions, and verification requirements",
            ));
        }
        Ok(())
    }

    pub fn from_legacy_text(text: &str) -> AcResult<Self> {
        let mut plan = Vec::new();
        let mut assumptions = Vec::new();
        let mut actions = Vec::new();
        let mut verification_requirements = Vec::new();
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
            if let Some(value) = line
                .strip_prefix("plan=")
                .or_else(|| line.strip_prefix("plan:"))
            {
                plan.push(value.trim().to_string());
            } else if let Some(value) = line
                .strip_prefix("assumption=")
                .or_else(|| line.strip_prefix("assumption:"))
            {
                assumptions.push(value.trim().to_string());
            } else if let Some(value) = line
                .strip_prefix("action=")
                .or_else(|| line.strip_prefix("action:"))
            {
                actions.push(value.trim().to_string());
            } else if let Some(value) = line
                .strip_prefix("verify=")
                .or_else(|| line.strip_prefix("verification="))
                .or_else(|| line.strip_prefix("verification_requirement="))
            {
                verification_requirements.push(value.trim().to_string());
            }
        }
        if plan.is_empty() && !text.trim().is_empty() {
            plan.push(text.trim().to_string());
        }
        if actions.is_empty() && !plan.is_empty() {
            actions.push("review provider plan".to_string());
        }
        if verification_requirements.is_empty() && !plan.is_empty() {
            verification_requirements.push("run configured verification profile".to_string());
        }
        let response = Self {
            plan,
            assumptions,
            actions,
            verification_requirements,
        };
        response.validate()?;
        Ok(response)
    }
}

pub fn parse_structured_agent_response(raw: &str) -> AcResult<StructuredAgentResponse> {
    let raw = raw.trim();
    if !(raw.starts_with('{') && raw.ends_with('}')) {
        return Err(AcError::validation(
            "PROVIDER-STRUCTURED_RESPONSE_REQUIRED",
            "structured provider output must be a JSON object",
        ));
    }
    let response = StructuredAgentResponse {
        plan: extract_string_array(raw, "plan")?,
        assumptions: extract_string_array(raw, "assumptions").unwrap_or_default(),
        actions: extract_string_array(raw, "actions")?,
        verification_requirements: extract_string_array(raw, "verification_requirements")?,
    };
    response.validate()?;
    Ok(response)
}

pub fn validate_provider_events(
    events: &[ProviderStreamEvent],
    allow_legacy_fallback: bool,
) -> AcResult<ValidatedProviderResponse> {
    let raw = events
        .iter()
        .filter_map(|event| match event {
            ProviderStreamEvent::Delta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    match parse_structured_agent_response(&raw) {
        Ok(response) => Ok(ValidatedProviderResponse {
            response,
            format: ProviderResponseFormat::Structured,
        }),
        Err(_) if allow_legacy_fallback => Ok(ValidatedProviderResponse {
            response: StructuredAgentResponse::from_legacy_text(&raw)?,
            format: ProviderResponseFormat::LegacyFallback,
        }),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderFailureClass {
    RateLimited,
    Timeout,
    ServerError,
    StreamAborted,
    MalformedResponse,
    Auth,
    Permission,
    ContextOverflow,
    UnsupportedCapability,
    Cancelled,
}

pub trait ProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfiguredProviderAdapter {
    pub endpoint_ref: String,
    pub credential_ref: String,
    pub model_name: String,
}

impl ConfiguredProviderAdapter {
    pub fn new(
        endpoint_ref: impl Into<String>,
        credential_ref: impl Into<String>,
        model_name: impl Into<String>,
    ) -> AcResult<Self> {
        let endpoint_ref = endpoint_ref.into();
        let credential_ref = credential_ref.into();
        let model_name = model_name.into();
        if endpoint_ref.trim().is_empty()
            || credential_ref.trim().is_empty()
            || model_name.trim().is_empty()
        {
            return Err(AcError::validation(
                "PROVIDER-INVALID_CONFIG",
                "endpoint, credential reference, and model name are required",
            ));
        }
        Ok(Self {
            endpoint_ref,
            credential_ref,
            model_name,
        })
    }
}

impl ProviderAdapter for ConfiguredProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        if request.prompt.trim().is_empty() {
            return Err(ProviderFailureClass::MalformedResponse);
        }
        Ok(vec![
            ProviderStreamEvent::Delta(format!(
                "provider_config:{}\nmodel:{}\nstructured_plan:requested",
                self.endpoint_ref, self.model_name
            )),
            ProviderStreamEvent::Usage {
                input_tokens: request.prompt.split_whitespace().count() as u32,
                output_tokens: 3,
            },
            ProviderStreamEvent::Finished,
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpProviderAdapter {
    pub endpoint: String,
    pub credential_env: Option<String>,
    pub model_name: String,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OllamaProviderAdapter {
    inner: HttpProviderAdapter,
}

impl OllamaProviderAdapter {
    pub fn new(endpoint: impl Into<String>, model_name: impl Into<String>) -> AcResult<Self> {
        Ok(Self {
            inner: HttpProviderAdapter::new(endpoint, None, model_name, 30_000)?,
        })
    }
}

impl ProviderAdapter for OllamaProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        self.inner.stream(request, cancel)
    }
}

impl HttpProviderAdapter {
    pub fn new(
        endpoint: impl Into<String>,
        credential_env: Option<String>,
        model_name: impl Into<String>,
        timeout_ms: u64,
    ) -> AcResult<Self> {
        let endpoint = endpoint.into();
        let model_name = model_name.into();
        if !endpoint.starts_with("http://") || model_name.trim().is_empty() || timeout_ms == 0 {
            return Err(AcError::validation(
                "PROVIDER-INVALID_HTTP_CONFIG",
                "http endpoint, model name, and timeout are required",
            ));
        }
        Ok(Self {
            endpoint,
            credential_env,
            model_name,
            timeout_ms,
        })
    }
}

impl ProviderAdapter for HttpProviderAdapter {
    fn stream(
        &self,
        request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        let endpoint = parse_http_endpoint(&self.endpoint)?;
        let timeout = Duration::from_millis(self.timeout_ms);
        let address = endpoint
            .address
            .to_socket_addrs()
            .map_err(|_| ProviderFailureClass::ServerError)?
            .next()
            .ok_or(ProviderFailureClass::ServerError)?;
        let mut stream = TcpStream::connect_timeout(&address, timeout)
            .map_err(|_| ProviderFailureClass::Timeout)?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|_| ProviderFailureClass::ServerError)?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|_| ProviderFailureClass::ServerError)?;
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        let credential = self
            .credential_env
            .as_ref()
            .and_then(|name| std::env::var(name).ok());
        let body = format!(
            "{{\"model\":\"{}\",\"prompt\":\"{}\",\"max_output_tokens\":{}}}",
            escape_json(&self.model_name),
            escape_json(&request.prompt),
            request.max_output_tokens
        );
        let auth = credential
            .as_ref()
            .map(|value| format!("Authorization: Bearer {}\r\n", value))
            .unwrap_or_default();
        let wire = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\n{}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            endpoint.path,
            endpoint.host_header,
            auth,
            body.len(),
            body
        );
        stream
            .write_all(wire.as_bytes())
            .map_err(|_| ProviderFailureClass::Timeout)?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|_| ProviderFailureClass::Timeout)?;
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        let (headers, body) = response
            .split_once("\r\n\r\n")
            .ok_or(ProviderFailureClass::MalformedResponse)?;
        let status = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse::<u16>().ok())
            .ok_or(ProviderFailureClass::MalformedResponse)?;
        events_from_http_parts(status, body, request)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScriptedProvider {
    responses: Vec<Result<Vec<ProviderStreamEvent>, ProviderFailureClass>>,
}

impl ScriptedProvider {
    pub fn new(responses: Vec<Result<Vec<ProviderStreamEvent>, ProviderFailureClass>>) -> Self {
        Self { responses }
    }
}

impl ProviderAdapter for ScriptedProvider {
    fn stream(
        &self,
        _request: &NormalizedInferenceRequest,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        if cancel() {
            return Err(ProviderFailureClass::Cancelled);
        }
        self.responses
            .first()
            .cloned()
            .unwrap_or_else(|| Ok(vec![ProviderStreamEvent::Finished]))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteAttempt {
    pub id: StableId,
    pub model_id: StableId,
    pub provider_id: StableId,
    pub started_at: TimestampMillis,
    pub events: Vec<ProviderStreamEvent>,
    pub failure: Option<ProviderFailureClass>,
}

#[derive(Default)]
pub struct ProviderRegistry {
    providers: BTreeMap<StableId, ProviderDefinition>,
    models: BTreeMap<StableId, ModelCapability>,
    adapters: BTreeMap<StableId, Box<dyn ProviderAdapter>>,
    connections: BTreeMap<StableId, ProviderConnection>,
    model_identities: BTreeMap<StableId, ModelIdentity>,
    routes: BTreeMap<StableId, ModelRoute>,
    circuit_breakers: BTreeMap<StableId, CircuitBreaker>,
    health_observations: Vec<HealthObservation>,
    routing_decisions: Vec<RoutingDecision>,
    attempts: Vec<RouteAttempt>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_provider(
        &mut self,
        name: impl Into<String>,
        credential_ref: Option<String>,
        capabilities: Vec<ProviderCapability>,
        quota_domain: impl Into<String>,
    ) -> AcResult<StableId> {
        let name = name.into();
        let quota_domain = quota_domain.into();
        if name.trim().is_empty() || quota_domain.trim().is_empty() {
            return Err(AcError::validation(
                "PROVIDER-INVALID_DEFINITION",
                "provider name and quota domain are required",
            ));
        }
        let id = StableId::new("provider");
        self.providers.insert(
            id.clone(),
            ProviderDefinition {
                id: id.clone(),
                name,
                credential_ref,
                capabilities,
                quota_domain,
            },
        );
        Ok(id)
    }

    pub fn register_adapter(
        &mut self,
        provider_id: &StableId,
        adapter: Box<dyn ProviderAdapter>,
    ) -> AcResult<()> {
        if !self.providers.contains_key(provider_id) {
            return Err(AcError::validation(
                "PROVIDER-UNKNOWN_PROVIDER",
                "provider must be registered before adapter",
            ));
        }
        self.adapters.insert(provider_id.clone(), adapter);
        Ok(())
    }

    pub fn register_model(
        &mut self,
        provider_id: &StableId,
        model_name: impl Into<String>,
        capabilities: Vec<ProviderCapability>,
        context_window: u32,
    ) -> AcResult<StableId> {
        let model_name = model_name.into();
        if !self.providers.contains_key(provider_id) {
            return Err(AcError::validation(
                "PROVIDER-UNKNOWN_PROVIDER",
                "provider must be registered before models",
            ));
        }
        if model_name.trim().is_empty() || context_window == 0 {
            return Err(AcError::validation(
                "PROVIDER-INVALID_MODEL",
                "model name and context window are required",
            ));
        }
        let id = StableId::new("model");
        self.models.insert(
            id.clone(),
            ModelCapability {
                id: id.clone(),
                provider_id: provider_id.clone(),
                model_name,
                capabilities,
                context_window,
                discovered_at: TimestampMillis::now(),
            },
        );
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_connection(
        &mut self,
        provider_id: &StableId,
        account_ref: impl Into<String>,
        credential_ref: Option<String>,
        quota_domain: impl Into<String>,
        endpoint_ref: impl Into<String>,
        enabled: bool,
        paid: bool,
        local: bool,
    ) -> AcResult<StableId> {
        if !self.providers.contains_key(provider_id) {
            return Err(AcError::validation(
                "PROVIDER-UNKNOWN_PROVIDER",
                "provider must exist before registering a connection",
            ));
        }
        let account_ref = account_ref.into();
        let quota_domain = quota_domain.into();
        let endpoint_ref = endpoint_ref.into();
        if account_ref.trim().is_empty()
            || quota_domain.trim().is_empty()
            || endpoint_ref.trim().is_empty()
        {
            return Err(AcError::validation(
                "PROVIDER-INVALID_CONNECTION",
                "account, quota domain, and endpoint reference are required",
            ));
        }
        let id = StableId::new("connection");
        self.connections.insert(
            id.clone(),
            ProviderConnection {
                id: id.clone(),
                provider_id: provider_id.clone(),
                account_ref,
                credential_ref,
                quota_domain,
                endpoint_ref,
                enabled,
                paid,
                local,
            },
        );
        self.circuit_breakers
            .insert(id.clone(), CircuitBreaker::healthy(id.clone()));
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_model_identity(
        &mut self,
        family: impl Into<String>,
        context_window: u32,
        coding_score: u8,
        reasoning_score: u8,
        vision: bool,
        tool_use: bool,
        structured_output: bool,
        input_cost_micros: u32,
        output_cost_micros: u32,
        privacy: PrivacyClass,
    ) -> AcResult<StableId> {
        let family = family.into();
        if family.trim().is_empty() || context_window == 0 {
            return Err(AcError::validation(
                "PROVIDER-INVALID_MODEL_IDENTITY",
                "model family and context window are required",
            ));
        }
        let id = StableId::new("model-family");
        self.model_identities.insert(
            id.clone(),
            ModelIdentity {
                id: id.clone(),
                family,
                context_window,
                coding_score,
                reasoning_score,
                vision,
                tool_use,
                structured_output,
                input_cost_micros,
                output_cost_micros,
                privacy,
            },
        );
        Ok(id)
    }

    pub fn register_model_route(
        &mut self,
        model_identity_id: &StableId,
        model_id: &StableId,
        connection_id: &StableId,
    ) -> AcResult<StableId> {
        let identity = self
            .model_identities
            .get(model_identity_id)
            .ok_or_else(|| {
                AcError::validation(
                    "PROVIDER-UNKNOWN_MODEL_IDENTITY",
                    "model identity not found",
                )
            })?;
        let model = self.models.get(model_id).ok_or_else(|| {
            AcError::validation("PROVIDER-UNKNOWN_MODEL", "model capability not found")
        })?;
        let connection = self.connections.get(connection_id).ok_or_else(|| {
            AcError::validation(
                "PROVIDER-UNKNOWN_CONNECTION",
                "provider connection not found",
            )
        })?;
        if model.provider_id != connection.provider_id {
            return Err(AcError::validation(
                "PROVIDER-ROUTE_PROVIDER_MISMATCH",
                "model and connection must belong to the same provider",
            ));
        }
        let id = StableId::new("route");
        self.routes.insert(
            id.clone(),
            ModelRoute {
                id: id.clone(),
                model_identity_id: identity.id.clone(),
                model_id: model.id.clone(),
                provider_id: model.provider_id.clone(),
                connection_id: connection.id.clone(),
                model_name: model.model_name.clone(),
            },
        );
        Ok(id)
    }

    pub fn select_model(&self, required: &[ProviderCapability]) -> AcResult<&ModelCapability> {
        self.models
            .values()
            .find(|model| required.iter().all(|cap| model.capabilities.contains(cap)))
            .ok_or_else(|| {
                AcError::validation(
                    "PROVIDER-NO_CAPABLE_MODEL",
                    "no registered model satisfies requested capabilities",
                )
            })
    }

    pub fn normalize_request(
        &self,
        prompt: impl Into<String>,
        required: Vec<ProviderCapability>,
        max_output_tokens: u32,
    ) -> AcResult<NormalizedInferenceRequest> {
        let prompt = prompt.into();
        if prompt.trim().is_empty() || max_output_tokens == 0 {
            return Err(AcError::validation(
                "PROVIDER-INVALID_REQUEST",
                "prompt and output token budget are required",
            ));
        }
        let model = self.select_model(&required)?;
        Ok(NormalizedInferenceRequest {
            model_id: model.id.clone(),
            prompt,
            required,
            max_output_tokens,
        })
    }

    pub fn start_attempt(&mut self, request: &NormalizedInferenceRequest) -> AcResult<StableId> {
        let model = self.models.get(&request.model_id).ok_or_else(|| {
            AcError::validation("PROVIDER-UNKNOWN_MODEL", "request model is not registered")
        })?;
        let id = StableId::new("route");
        self.attempts.push(RouteAttempt {
            id: id.clone(),
            model_id: model.id.clone(),
            provider_id: model.provider_id.clone(),
            started_at: TimestampMillis::now(),
            events: Vec::new(),
            failure: None,
        });
        Ok(id)
    }

    pub fn record_event(
        &mut self,
        attempt_id: &StableId,
        event: ProviderStreamEvent,
    ) -> AcResult<()> {
        let attempt = self
            .attempts
            .iter_mut()
            .find(|attempt| &attempt.id == attempt_id)
            .ok_or_else(|| {
                AcError::validation("PROVIDER-UNKNOWN_ATTEMPT", "route attempt not found")
            })?;
        attempt.events.push(event);
        Ok(())
    }

    pub fn finish_failed(
        &mut self,
        attempt_id: &StableId,
        failure: ProviderFailureClass,
    ) -> AcResult<()> {
        let attempt = self
            .attempts
            .iter_mut()
            .find(|attempt| &attempt.id == attempt_id)
            .ok_or_else(|| {
                AcError::validation("PROVIDER-UNKNOWN_ATTEMPT", "route attempt not found")
            })?;
        attempt.failure = Some(failure);
        Ok(())
    }

    pub fn stream_with_retry(
        &mut self,
        request: &NormalizedInferenceRequest,
        max_attempts: usize,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
        let model = self
            .models
            .get(&request.model_id)
            .ok_or(ProviderFailureClass::UnsupportedCapability)?
            .clone();
        let mut remaining = VecDeque::from_iter(0..max_attempts.max(1));
        let mut last_failure = None;
        while remaining.pop_front().is_some() {
            if cancel() {
                return Err(ProviderFailureClass::Cancelled);
            }
            let attempt_id = self
                .start_attempt(request)
                .map_err(|_| ProviderFailureClass::MalformedResponse)?;
            let result = {
                let adapter = self
                    .adapters
                    .get(&model.provider_id)
                    .ok_or(ProviderFailureClass::UnsupportedCapability)?;
                adapter.stream(request, cancel)
            };
            match result {
                Ok(events) => {
                    for event in &events {
                        self.record_event(&attempt_id, event.clone())
                            .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    }
                    if matches!(events.last(), Some(ProviderStreamEvent::Finished)) {
                        return Ok(events);
                    }
                    self.finish_failed(&attempt_id, ProviderFailureClass::StreamAborted)
                        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    last_failure = Some(ProviderFailureClass::StreamAborted);
                }
                Err(failure) => {
                    self.finish_failed(&attempt_id, failure)
                        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    if !is_retryable(failure) {
                        return Err(failure);
                    }
                    last_failure = Some(failure);
                }
            }
        }
        Err(last_failure.unwrap_or(ProviderFailureClass::ServerError))
    }

    pub fn attempts(&self) -> &[RouteAttempt] {
        &self.attempts
    }

    pub fn ranked_candidates(&self, profile: &TaskProfile) -> Vec<RouteCandidate> {
        let mut rejected = Vec::new();
        let mut candidates = self
            .routes
            .values()
            .filter_map(|route| self.candidate_for_route(route, profile, &mut rejected).ok())
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
        candidates
    }

    pub fn request_model(
        &mut self,
        profile: &TaskProfile,
        prompt: impl Into<String>,
        max_output_tokens: u32,
        cancel: &dyn Fn() -> bool,
    ) -> Result<RouteExecution, ProviderFailureClass> {
        let prompt = prompt.into();
        let mut rejected = Vec::new();
        let mut candidates = self
            .routes
            .values()
            .filter_map(|route| self.candidate_for_route(route, profile, &mut rejected).ok())
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
        let mut fallback_reason = None;
        let mut last_failure = None;
        for candidate in candidates.clone() {
            if cancel() {
                return Err(ProviderFailureClass::Cancelled);
            }
            let route = self
                .routes
                .get(&candidate.route_id)
                .ok_or(ProviderFailureClass::UnsupportedCapability)?
                .clone();
            let request = NormalizedInferenceRequest {
                model_id: route.model_id,
                prompt: prompt.clone(),
                required: required_from_profile(profile),
                max_output_tokens,
            };
            let attempt_id = self
                .start_attempt(&request)
                .map_err(|_| ProviderFailureClass::MalformedResponse)?;
            let started = TimestampMillis::now();
            let result = {
                let adapter = self
                    .adapters
                    .get(&candidate.provider_id)
                    .ok_or(ProviderFailureClass::UnsupportedCapability)?;
                adapter.stream(&request, cancel)
            };
            let latency_ms = TimestampMillis::now()
                .as_millis()
                .saturating_sub(started.as_millis()) as u64;
            match result {
                Ok(events) if matches!(events.last(), Some(ProviderStreamEvent::Finished)) => {
                    for event in &events {
                        self.record_event(&attempt_id, event.clone())
                            .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    }
                    self.record_route_success(&candidate.connection_id, latency_ms);
                    let (input_tokens, output_tokens) = token_usage(&events);
                    let estimated_cost_micros = self.estimate_cost(
                        &candidate.model_identity_id,
                        input_tokens,
                        output_tokens,
                    );
                    let decision = self.routing_decision(
                        profile,
                        candidates,
                        rejected,
                        Some(candidate),
                        fallback_reason,
                        latency_ms,
                        input_tokens,
                        output_tokens,
                        estimated_cost_micros,
                    );
                    self.routing_decisions.push(decision.clone());
                    return Ok(RouteExecution { events, decision });
                }
                Ok(events) => {
                    for event in &events {
                        self.record_event(&attempt_id, event.clone())
                            .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    }
                    self.finish_failed(&attempt_id, ProviderFailureClass::StreamAborted)
                        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    let normalized = normalize_failure(ProviderFailureClass::StreamAborted);
                    self.record_route_failure(
                        &candidate.connection_id,
                        normalized.clone(),
                        latency_ms,
                    );
                    fallback_reason = Some(format!("{}:{:?}", candidate.connection_id, normalized));
                    last_failure = Some(ProviderFailureClass::StreamAborted);
                }
                Err(failure) => {
                    self.finish_failed(&attempt_id, failure)
                        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
                    let normalized = normalize_failure(failure);
                    self.record_route_failure(
                        &candidate.connection_id,
                        normalized.clone(),
                        latency_ms,
                    );
                    fallback_reason = Some(format!("{}:{:?}", candidate.connection_id, normalized));
                    last_failure = Some(failure);
                }
            }
        }
        let decision = self.routing_decision(
            profile,
            candidates,
            rejected,
            None,
            fallback_reason,
            0,
            0,
            0,
            0,
        );
        self.routing_decisions.push(decision);
        Err(last_failure.unwrap_or(ProviderFailureClass::UnsupportedCapability))
    }

    pub fn routing_decisions(&self) -> &[RoutingDecision] {
        &self.routing_decisions
    }

    pub fn health_observations(&self) -> &[HealthObservation] {
        &self.health_observations
    }

    pub fn circuit_breaker(&self, connection_id: &StableId) -> Option<&CircuitBreaker> {
        self.circuit_breakers.get(connection_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn routing_decision(
        &self,
        profile: &TaskProfile,
        candidates: Vec<RouteCandidate>,
        rejected: Vec<String>,
        selected: Option<RouteCandidate>,
        fallback_reason: Option<String>,
        latency_ms: u64,
        input_tokens: u32,
        output_tokens: u32,
        estimated_cost_micros: u64,
    ) -> RoutingDecision {
        RoutingDecision {
            id: StableId::new("routing"),
            task_id: profile.task_id.clone(),
            candidates,
            selected,
            rejected,
            fallback_reason,
            latency_ms,
            input_tokens,
            output_tokens,
            estimated_cost_micros,
            created_at: TimestampMillis::now(),
        }
    }

    fn candidate_for_route(
        &self,
        route: &ModelRoute,
        profile: &TaskProfile,
        rejected: &mut Vec<String>,
    ) -> Result<RouteCandidate, ()> {
        let identity = self
            .model_identities
            .get(&route.model_identity_id)
            .ok_or(())?;
        let connection = self.connections.get(&route.connection_id).ok_or(())?;
        let model = self.models.get(&route.model_id).ok_or(())?;
        let breaker_open = self
            .circuit_breakers
            .get(&connection.id)
            .map(CircuitBreaker::is_open)
            .unwrap_or(false);
        let reject = |rejected: &mut Vec<String>, reason: String| {
            rejected.push(format!("{}:{}", route.id, reason));
            Err(())
        };
        if !connection.enabled {
            return reject(rejected, "connection_disabled".to_string());
        }
        if breaker_open {
            return reject(rejected, "circuit_open".to_string());
        }
        if profile.routing_profile == RoutingProfile::Offline && !connection.local {
            return reject(rejected, "offline_requires_local".to_string());
        }
        if profile.routing_profile == RoutingProfile::FreeOnly && connection.paid {
            return reject(rejected, "paid_disallowed".to_string());
        }
        if profile.privacy == PrivacyClass::LocalOnly && !connection.local {
            return reject(rejected, "privacy_requires_local".to_string());
        }
        if profile.required_context > identity.context_window {
            return reject(rejected, "context_too_small".to_string());
        }
        if profile.requires_vision && !identity.vision {
            return reject(rejected, "vision_missing".to_string());
        }
        if profile.requires_tool_use && !identity.tool_use {
            return reject(rejected, "tool_use_missing".to_string());
        }
        if profile.requires_structured_output && !identity.structured_output {
            return reject(rejected, "structured_output_missing".to_string());
        }
        if !required_from_profile(profile)
            .iter()
            .all(|capability| model.capabilities.contains(capability))
        {
            return reject(rejected, "capability_missing".to_string());
        }
        if let Some(max_cost) = profile.max_input_cost_micros {
            if identity.input_cost_micros > max_cost
                && profile.routing_profile != RoutingProfile::PaidAllowed
            {
                return reject(rejected, "budget_exceeded".to_string());
            }
        }
        let mut score = if profile.role.contains("verification") || profile.role.contains("critic")
        {
            identity.reasoning_score as i32 * 2 + identity.coding_score as i32
        } else {
            identity.coding_score as i32 * 2 + identity.reasoning_score as i32
        };
        score += (identity.context_window / 4096).min(20) as i32;
        score -= (profile
            .complexity
            .saturating_sub(identity.reasoning_score / 20)) as i32;
        if connection.local {
            score += match profile.routing_profile {
                RoutingProfile::LocalFirst | RoutingProfile::Offline => 50,
                _ => 5,
            };
        }
        if !connection.paid {
            score += match profile.routing_profile {
                RoutingProfile::FreeOnly | RoutingProfile::FreeFirst => 60,
                _ => 5,
            };
        } else if profile.routing_profile == RoutingProfile::QualityFirst {
            score += 10;
        }
        Ok(RouteCandidate {
            route_id: route.id.clone(),
            model_identity_id: identity.id.clone(),
            provider_id: route.provider_id.clone(),
            connection_id: connection.id.clone(),
            model_name: route.model_name.clone(),
            score,
            reasons: vec![
                format!("role:{}", profile.role),
                format!("task:{}", profile.task_type),
                format!("profile:{:?}", profile.routing_profile),
                format!("local:{}", connection.local),
                format!("paid:{}", connection.paid),
            ],
        })
    }

    fn record_route_success(&mut self, connection_id: &StableId, latency_ms: u64) {
        if let Some(breaker) = self.circuit_breakers.get_mut(connection_id) {
            breaker.record_success();
        }
        self.health_observations.push(HealthObservation {
            connection_id: connection_id.clone(),
            failure: None,
            latency_ms,
            created_at: TimestampMillis::now(),
        });
    }

    fn record_route_failure(
        &mut self,
        connection_id: &StableId,
        failure: NormalizedProviderFailure,
        latency_ms: u64,
    ) {
        if let Some(breaker) = self.circuit_breakers.get_mut(connection_id) {
            breaker.record_failure(failure.clone());
        }
        self.health_observations.push(HealthObservation {
            connection_id: connection_id.clone(),
            failure: Some(failure),
            latency_ms,
            created_at: TimestampMillis::now(),
        });
    }

    fn estimate_cost(
        &self,
        model_identity_id: &StableId,
        input_tokens: u32,
        output_tokens: u32,
    ) -> u64 {
        self.model_identities
            .get(model_identity_id)
            .map(|identity| {
                input_tokens as u64 * identity.input_cost_micros as u64
                    + output_tokens as u64 * identity.output_cost_micros as u64
            })
            .unwrap_or(0)
    }
}

fn is_retryable(failure: ProviderFailureClass) -> bool {
    matches!(
        failure,
        ProviderFailureClass::RateLimited
            | ProviderFailureClass::Timeout
            | ProviderFailureClass::ServerError
            | ProviderFailureClass::StreamAborted
    )
}

pub fn normalize_failure(failure: ProviderFailureClass) -> NormalizedProviderFailure {
    match failure {
        ProviderFailureClass::RateLimited => NormalizedProviderFailure::RateLimit,
        ProviderFailureClass::Timeout => NormalizedProviderFailure::Timeout,
        ProviderFailureClass::ServerError => NormalizedProviderFailure::ProviderUnavailable,
        ProviderFailureClass::StreamAborted => NormalizedProviderFailure::ProviderUnavailable,
        ProviderFailureClass::MalformedResponse => NormalizedProviderFailure::BadResponse,
        ProviderFailureClass::Auth | ProviderFailureClass::Permission => {
            NormalizedProviderFailure::AuthFailed
        }
        ProviderFailureClass::ContextOverflow => NormalizedProviderFailure::ContextLimit,
        ProviderFailureClass::UnsupportedCapability => NormalizedProviderFailure::ModelUnavailable,
        ProviderFailureClass::Cancelled => NormalizedProviderFailure::ProviderUnavailable,
    }
}

fn required_from_profile(profile: &TaskProfile) -> Vec<ProviderCapability> {
    let mut required = vec![ProviderCapability::Chat];
    if profile.requires_tool_use {
        required.push(ProviderCapability::ToolCalls);
    }
    if profile.requires_vision {
        required.push(ProviderCapability::Vision);
    }
    required
}

fn token_usage(events: &[ProviderStreamEvent]) -> (u32, u32) {
    events
        .iter()
        .find_map(|event| {
            if let ProviderStreamEvent::Usage {
                input_tokens,
                output_tokens,
            } = event
            {
                Some((*input_tokens, *output_tokens))
            } else {
                None
            }
        })
        .unwrap_or((0, 0))
}

struct ParsedHttpEndpoint {
    address: String,
    host_header: String,
    path: String,
}

fn parse_http_endpoint(endpoint: &str) -> Result<ParsedHttpEndpoint, ProviderFailureClass> {
    let rest = endpoint
        .strip_prefix("http://")
        .ok_or(ProviderFailureClass::MalformedResponse)?;
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    if authority.trim().is_empty() {
        return Err(ProviderFailureClass::MalformedResponse);
    }
    let address = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{}:80", authority)
    };
    Ok(ParsedHttpEndpoint {
        address,
        host_header: authority.to_string(),
        path: format!("/{}", path),
    })
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn extract_string_array(raw: &str, key: &str) -> AcResult<Vec<String>> {
    let needle = format!("\"{}\"", key);
    let after_key = raw
        .split_once(&needle)
        .map(|(_, after)| after)
        .ok_or_else(|| {
            AcError::validation(
                "PROVIDER-MISSING_STRUCTURED_FIELD",
                format!("structured provider output missing {key}"),
            )
        })?;
    let after_colon = after_key
        .split_once(':')
        .map(|(_, after)| after)
        .ok_or_else(|| {
            AcError::validation(
                "PROVIDER-MALFORMED_STRUCTURED_FIELD",
                format!("structured provider output field {key} is malformed"),
            )
        })?;
    let start = after_colon.find('[').ok_or_else(|| {
        AcError::validation(
            "PROVIDER-MALFORMED_STRUCTURED_FIELD",
            format!("structured provider output field {key} must be an array"),
        )
    })?;
    let array = &after_colon[start + 1..];
    let end = array.find(']').ok_or_else(|| {
        AcError::validation(
            "PROVIDER-MALFORMED_STRUCTURED_FIELD",
            format!("structured provider output field {key} must close its array"),
        )
    })?;
    Ok(array[..end]
        .split(',')
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                Some(
                    trimmed[1..trimmed.len() - 1]
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\")
                        .trim()
                        .to_string(),
                )
            } else {
                None
            }
        })
        .filter(|value| !value.is_empty())
        .collect())
}

fn extract_delta(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(start) = trimmed.find("\"delta\"") {
        let after_key = &trimmed[start + "\"delta\"".len()..];
        let after_colon = after_key.split_once(':')?.1.trim_start();
        let value = after_colon.strip_prefix('"')?;
        let end = value.find('"')?;
        return Some(value[..end].replace("\\n", "\n").replace("\\\"", "\""));
    }
    Some(trimmed.to_string())
}

fn events_from_http_parts(
    status: u16,
    body: &str,
    request: &NormalizedInferenceRequest,
) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
    match status {
        200..=299 => {
            let delta = extract_delta(body).ok_or(ProviderFailureClass::MalformedResponse)?;
            Ok(vec![
                ProviderStreamEvent::Delta(delta),
                ProviderStreamEvent::Usage {
                    input_tokens: request.prompt.split_whitespace().count() as u32,
                    output_tokens: body.split_whitespace().count() as u32,
                },
                ProviderStreamEvent::Finished,
            ])
        }
        401 | 403 => Err(ProviderFailureClass::Auth),
        408 | 504 => Err(ProviderFailureClass::Timeout),
        429 => Err(ProviderFailureClass::RateLimited),
        400..=499 => Err(ProviderFailureClass::MalformedResponse),
        _ => Err(ProviderFailureClass::ServerError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RetryProvider {
        responses: Mutex<VecDeque<Result<Vec<ProviderStreamEvent>, ProviderFailureClass>>>,
    }

    impl ProviderAdapter for RetryProvider {
        fn stream(
            &self,
            _request: &NormalizedInferenceRequest,
            cancel: &dyn Fn() -> bool,
        ) -> Result<Vec<ProviderStreamEvent>, ProviderFailureClass> {
            if cancel() {
                return Err(ProviderFailureClass::Cancelled);
            }
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Err(ProviderFailureClass::ServerError))
        }
    }

    struct Phase4Fabric {
        registry: ProviderRegistry,
        free_connection: StableId,
        paid_connection: StableId,
        local_connection: StableId,
    }

    fn successful_events(text: &str) -> Vec<ProviderStreamEvent> {
        vec![
            ProviderStreamEvent::Delta(text.to_string()),
            ProviderStreamEvent::Usage {
                input_tokens: 7,
                output_tokens: 11,
            },
            ProviderStreamEvent::Finished,
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn register_route(
        registry: &mut ProviderRegistry,
        provider_name: &str,
        adapter: Box<dyn ProviderAdapter>,
        family: &str,
        model_name: &str,
        coding_score: u8,
        reasoning_score: u8,
        paid: bool,
        local: bool,
    ) -> (StableId, StableId, StableId) {
        let provider = registry
            .register_provider(
                provider_name,
                Some(format!("secret:{}", provider_name)),
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                provider_name,
            )
            .unwrap();
        registry.register_adapter(&provider, adapter).unwrap();
        let model = registry
            .register_model(
                &provider,
                model_name,
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                16_384,
            )
            .unwrap();
        let connection = registry
            .register_connection(
                &provider,
                format!("account:{}", provider_name),
                Some(format!("env:{}_KEY", provider_name.to_ascii_uppercase())),
                provider_name,
                format!("config:{}.endpoint", provider_name),
                true,
                paid,
                local,
            )
            .unwrap();
        let identity = registry
            .register_model_identity(
                family,
                16_384,
                coding_score,
                reasoning_score,
                false,
                false,
                true,
                if paid { 2 } else { 0 },
                if paid { 4 } else { 0 },
                if local {
                    PrivacyClass::LocalOnly
                } else {
                    PrivacyClass::ExternalAllowed
                },
            )
            .unwrap();
        let route = registry
            .register_model_route(&identity, &model, &connection)
            .unwrap();
        (connection, identity, route)
    }

    fn phase4_fabric(
        free_adapter: Box<dyn ProviderAdapter>,
        paid_adapter: Box<dyn ProviderAdapter>,
    ) -> Phase4Fabric {
        let mut registry = ProviderRegistry::new();
        let (free_connection, _, _) = register_route(
            &mut registry,
            "free-remote",
            free_adapter,
            "qwen-coder-free",
            "free-coder",
            90,
            65,
            false,
            false,
        );
        let (paid_connection, _, _) = register_route(
            &mut registry,
            "paid-remote",
            paid_adapter,
            "deepseek-paid",
            "paid-coder",
            95,
            90,
            true,
            false,
        );
        let (local_connection, _, _) = register_route(
            &mut registry,
            "ollama-local",
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("local"))])),
            "qwen2.5-coder:3b",
            "qwen2.5-coder:3b",
            70,
            60,
            false,
            true,
        );
        Phase4Fabric {
            registry,
            free_connection,
            paid_connection,
            local_connection,
        }
    }

    #[test]
    fn model_selection_uses_capabilities_not_provider_state() {
        let mut registry = ProviderRegistry::new();
        let provider = registry
            .register_provider("local", None, vec![ProviderCapability::LocalModel], "local")
            .unwrap();
        registry
            .register_model(
                &provider,
                "small",
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                8192,
            )
            .unwrap();

        let request = registry
            .normalize_request("hello", vec![ProviderCapability::Streaming], 128)
            .unwrap();
        let attempt = registry.start_attempt(&request).unwrap();
        registry
            .record_event(&attempt, ProviderStreamEvent::Delta("hi".to_string()))
            .unwrap();

        assert_eq!(registry.attempts()[0].events.len(), 1);
    }

    #[test]
    fn unsupported_capability_is_explicit() {
        let registry = ProviderRegistry::new();
        let err = registry
            .select_model(&[ProviderCapability::Vision])
            .unwrap_err();
        assert_eq!(err.code(), "PROVIDER-NO_CAPABLE_MODEL");
    }

    #[test]
    fn provider_streaming_retries_and_records_lifecycle() {
        let mut registry = ProviderRegistry::new();
        let provider = registry
            .register_provider(
                "mock",
                Some("env:AGENTCODE_PROVIDER_KEY".to_string()),
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                "mock",
            )
            .unwrap();
        registry
            .register_adapter(
                &provider,
                Box::new(RetryProvider {
                    responses: Mutex::new(VecDeque::from([
                        Err(ProviderFailureClass::Timeout),
                        Ok(vec![
                            ProviderStreamEvent::Delta("plan".to_string()),
                            ProviderStreamEvent::Usage {
                                input_tokens: 3,
                                output_tokens: 5,
                            },
                            ProviderStreamEvent::Finished,
                        ]),
                    ])),
                }),
            )
            .unwrap();
        registry
            .register_model(
                &provider,
                "mock-model",
                vec![ProviderCapability::Chat, ProviderCapability::Streaming],
                4096,
            )
            .unwrap();
        let request = registry
            .normalize_request("plan task", vec![ProviderCapability::Chat], 128)
            .unwrap();
        let events = registry.stream_with_retry(&request, 2, &|| false).unwrap();
        assert!(matches!(events.last(), Some(ProviderStreamEvent::Finished)));
        assert_eq!(registry.attempts().len(), 2);
        assert_eq!(
            registry.attempts()[0].failure,
            Some(ProviderFailureClass::Timeout)
        );
    }

    #[test]
    fn provider_streaming_obeys_cancellation() {
        let mut registry = ProviderRegistry::new();
        let provider = registry
            .register_provider("mock", None, vec![ProviderCapability::Chat], "mock")
            .unwrap();
        registry
            .register_adapter(&provider, Box::new(ScriptedProvider::default()))
            .unwrap();
        registry
            .register_model(
                &provider,
                "mock-model",
                vec![ProviderCapability::Chat],
                4096,
            )
            .unwrap();
        let request = registry
            .normalize_request("plan task", vec![ProviderCapability::Chat], 128)
            .unwrap();
        let failure = registry
            .stream_with_retry(&request, 1, &|| true)
            .unwrap_err();
        assert_eq!(failure, ProviderFailureClass::Cancelled);
    }

    #[test]
    fn configured_provider_adapter_uses_references_not_secrets() {
        let adapter = ConfiguredProviderAdapter::new(
            "config:provider.endpoint",
            "credential:provider_key_ref",
            "m",
        )
        .unwrap();
        let request = NormalizedInferenceRequest {
            model_id: StableId::new("model"),
            prompt: "hello world".to_string(),
            required: vec![ProviderCapability::Chat],
            max_output_tokens: 16,
        };
        let events = adapter.stream(&request, &|| false).unwrap();
        assert!(matches!(events.last(), Some(ProviderStreamEvent::Finished)));
        assert!(matches!(
            &events[0],
            ProviderStreamEvent::Delta(text)
                if text.contains("config:provider.endpoint") && !text.contains("provider.key=")
        ));
    }

    #[test]
    fn http_provider_adapter_normalizes_mocked_response() {
        let adapter = HttpProviderAdapter::new(
            "http://provider.example/v1/chat",
            Some("AGENTCODE_TEST_PROVIDER_KEY".to_string()),
            "fixture-model",
            1000,
        )
        .unwrap();
        let request = NormalizedInferenceRequest {
            model_id: StableId::new("model"),
            prompt: "plan".to_string(),
            required: vec![ProviderCapability::Chat],
            max_output_tokens: 32,
        };
        let endpoint = parse_http_endpoint(&adapter.endpoint).unwrap();
        assert_eq!(endpoint.path, "/v1/chat");
        let events =
            events_from_http_parts(200, "{\"delta\":\"goal=fix\\nverify=status:0\"}", &request)
                .unwrap();
        assert!(matches!(events.last(), Some(ProviderStreamEvent::Finished)));
        assert!(
            matches!(&events[0], ProviderStreamEvent::Delta(text) if text.contains("goal=fix"))
        );
        assert_eq!(
            events_from_http_parts(429, "rate limited", &request).unwrap_err(),
            ProviderFailureClass::RateLimited
        );
    }

    #[test]
    fn structured_agent_response_validates_and_rejects_invalid_output() {
        let raw = r#"{
            "plan": ["inspect task"],
            "assumptions": ["workspace is available"],
            "actions": ["edit file"],
            "verification_requirements": ["run tests"]
        }"#;
        let parsed = parse_structured_agent_response(raw).unwrap();
        assert_eq!(parsed.plan, vec!["inspect task"]);
        assert_eq!(parsed.actions, vec!["edit file"]);
        assert!(parse_structured_agent_response(
            r#"{"plan":["inspect"],"assumptions":[],"actions":[],"verification_requirements":[]}"#
        )
        .is_err());
    }

    #[test]
    fn provider_events_accept_legacy_fallback_without_breaking_scripted_provider() {
        let events = successful_events("plan=fix bug\naction=patch file\nverify=cargo test");
        let validated = validate_provider_events(&events, true).unwrap();
        assert_eq!(validated.format, ProviderResponseFormat::LegacyFallback);
        assert_eq!(validated.response.plan, vec!["fix bug"]);
        assert!(validate_provider_events(&events, false).is_err());
    }

    #[test]
    fn model_broker_ranks_candidates_independent_of_connections() {
        let fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("free"))])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let candidates = fabric.registry.ranked_candidates(&TaskProfile::coding(
            StableId::new("task"),
            RoutingProfile::FreeFirst,
        ));
        assert!(candidates.len() >= 2);
        assert_eq!(candidates[0].connection_id, fabric.free_connection);
        assert!(candidates[0]
            .reasons
            .iter()
            .any(|reason| reason == "paid:false"));
    }

    #[test]
    fn rate_limit_triggers_fallback_cooldown_and_persists_decision_data() {
        let mut fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::RateLimited,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let execution = fabric
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeFirst),
                "fix bug",
                128,
                &|| false,
            )
            .unwrap();
        assert_eq!(
            execution.decision.selected.as_ref().unwrap().connection_id,
            fabric.paid_connection
        );
        assert!(execution
            .decision
            .fallback_reason
            .as_ref()
            .unwrap()
            .contains("RateLimit"));
        assert!(fabric
            .registry
            .circuit_breaker(&fabric.free_connection)
            .unwrap()
            .is_open());
        assert_eq!(fabric.registry.routing_decisions().len(), 1);
        assert!(format!("{:?}", execution.decision).contains("connection-"));
        assert!(!format!("{:?}", execution.decision).contains("_KEY"));
    }

    #[test]
    fn timeout_triggers_recovery_without_manual_task_restart() {
        let mut fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::Timeout,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events(
                "recovered",
            ))])),
        );
        let execution = fabric
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeFirst),
                "recover",
                128,
                &|| false,
            )
            .unwrap();
        assert!(matches!(
            &execution.events[0],
            ProviderStreamEvent::Delta(text) if text == "recovered"
        ));
        assert_eq!(fabric.registry.attempts().len(), 2);
    }

    #[test]
    fn paid_policy_blocks_and_paid_fallback_requires_explicit_allowance() {
        let mut free_unavailable = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::ServerError,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        free_unavailable
            .registry
            .connections
            .get_mut(&free_unavailable.local_connection)
            .unwrap()
            .enabled = false;
        let blocked = free_unavailable
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeOnly),
                "free only",
                128,
                &|| false,
            )
            .unwrap_err();
        assert_eq!(blocked, ProviderFailureClass::ServerError);
        let paid_rejected = free_unavailable
            .registry
            .routing_decisions()
            .last()
            .unwrap()
            .rejected
            .iter()
            .any(|reason| reason.contains("paid_disallowed"));
        assert!(paid_rejected);

        let mut paid_allowed = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Err(
                ProviderFailureClass::ServerError,
            )])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        paid_allowed
            .registry
            .connections
            .get_mut(&paid_allowed.local_connection)
            .unwrap()
            .enabled = false;
        let execution = paid_allowed
            .registry
            .request_model(
                &TaskProfile::coding(StableId::new("task"), RoutingProfile::FreeFirst),
                "paid fallback",
                128,
                &|| false,
            )
            .unwrap();
        assert_eq!(
            execution.decision.selected.as_ref().unwrap().connection_id,
            paid_allowed.paid_connection
        );
    }

    #[test]
    fn local_ollama_route_is_selected_for_offline_profile() {
        let mut fabric = phase4_fabric(
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("free"))])),
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("paid"))])),
        );
        let mut profile = TaskProfile::coding(StableId::new("task"), RoutingProfile::Offline);
        profile.privacy = PrivacyClass::LocalOnly;
        let execution = fabric
            .registry
            .request_model(&profile, "local", 128, &|| false)
            .unwrap();
        assert_eq!(
            execution.decision.selected.as_ref().unwrap().connection_id,
            fabric.local_connection
        );
    }

    #[test]
    fn provider_failure_categories_normalize_to_phase4_contract() {
        assert_eq!(
            normalize_failure(ProviderFailureClass::RateLimited),
            NormalizedProviderFailure::RateLimit
        );
        assert_eq!(
            normalize_failure(ProviderFailureClass::Auth),
            NormalizedProviderFailure::AuthFailed
        );
        assert_eq!(
            normalize_failure(ProviderFailureClass::ContextOverflow),
            NormalizedProviderFailure::ContextLimit
        );
        assert_eq!(
            normalize_failure(ProviderFailureClass::MalformedResponse),
            NormalizedProviderFailure::BadResponse
        );
    }

    #[test]
    fn diversity_gate_can_select_different_model_families_by_role() {
        let mut registry = ProviderRegistry::new();
        let (_, impl_identity, _) = register_route(
            &mut registry,
            "impl-remote",
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("impl"))])),
            "implementation-family-a",
            "impl-model",
            95,
            40,
            false,
            false,
        );
        let (_, verify_identity, _) = register_route(
            &mut registry,
            "verify-remote",
            Box::new(ScriptedProvider::new(vec![Ok(successful_events("verify"))])),
            "verification-family-b",
            "verify-model",
            50,
            95,
            false,
            false,
        );
        let implementation = registry.ranked_candidates(&TaskProfile::coding(
            StableId::new("task"),
            RoutingProfile::QualityFirst,
        ));
        let mut verification =
            TaskProfile::coding(StableId::new("task"), RoutingProfile::QualityFirst);
        verification.role = "verification".to_string();
        let verification = registry.ranked_candidates(&verification);
        assert_eq!(implementation[0].model_identity_id, impl_identity);
        assert_eq!(verification[0].model_identity_id, verify_identity);
    }
}
