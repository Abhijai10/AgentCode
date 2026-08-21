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
}
