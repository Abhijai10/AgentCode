use std::time::Duration;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

use crate::{
    map_reqwest_error, normalize_failure, provider_error_class_from_value,
    NormalizedProviderFailure, ProviderFailureClass,
};

/// A provider catalog entry (backend-owned source of truth).  Never
/// constructed from frontend input without validation; the daemon persists
/// these rows and the agent builds runtime routes from them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCatalogEntry {
    pub id: StableId,
    pub display_name: String,
    pub description: String,
    pub website_url: String,
    pub logo_url: String,
    pub credential_url: String,
    pub pricing_classification: String,
    pub capabilities: Vec<String>,
}

impl ProviderCatalogEntry {
    pub fn pricing_is_paid(&self) -> bool {
        matches!(
            self.pricing_classification.as_str(),
            "paid" | "freemium" | "tiered"
        )
    }

    pub fn pricing_is_free(&self) -> bool {
        self.pricing_classification == "free"
            || self.pricing_classification == "free-tier"
            || self.pricing_classification == "local"
    }
}

/// A provider account.  The credential reference must never be resolved to a
/// raw secret anywhere a UI/IPC response can observe it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderAccount {
    pub id: StableId,
    pub provider_id: StableId,
    pub label: String,
    pub credential_ref: String,
    pub credential_region: String,
    pub organization: String,
    pub project: String,
    pub workspace: String,
    pub enabled: bool,
    pub health_state: String,
    pub quota_rate_limit: Option<u64>,
    pub quota_remaining: Option<u64>,
    pub quota_reset_at_ms: Option<u64>,
    pub last_success_at_ms: Option<u64>,
    pub last_failure_at_ms: Option<u64>,
    pub failure_reason: String,
}

impl ProviderAccount {
    pub fn masked_credential(&self) -> String {
        // The account only ever exposes a masked form of the credential
        // reference plus a stable fingerprint.  The raw value is resolved
        // exclusively inside the daemon request path.
        let ref_part = self
            .credential_ref
            .rsplit([':', '/'])
            .next()
            .unwrap_or("ref")
            .to_string();
        if ref_part.is_empty() {
            "ref:****".to_string()
        } else {
            format!("ref:*****{}", last_n(&ref_part, 4))
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.health_state == "ok" || self.health_state == "healthy"
    }
}

fn last_n(value: &str, n: usize) -> String {
    value
        .chars()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// Normalized, non-secret account status returned to the UI after a
/// connection test or health refresh.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderAccountStatus {
    pub account_id: StableId,
    pub provider_id: StableId,
    pub ok: bool,
    pub health_state: String,
    pub latency_ms: u64,
    pub failure: Option<NormalizedProviderFailure>,
    pub masked_credential: String,
    pub detail: String,
}

/// Result of a model-discovery operation for one provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredModel {
    pub id: StableId,
    pub provider_id: StableId,
    pub model_name: String,
    pub capabilities: Vec<String>,
    pub context_window: Option<u32>,
    pub parameters: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderConnectionTest {
    pub account: ProviderAccount,
    pub endpoint: String,
    pub model_name: String,
    pub kind: ProviderConnectionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderConnectionKind {
    OpenAiCompatible,
    OpenAiChatCompletions,
    AnthropicMessages,
    GeminiGenerateContent,
    OllamaChat,
    OllamaTags,
}

impl ProviderConnectionTest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account: ProviderAccount,
        endpoint: impl Into<String>,
        model_name: impl Into<String>,
        kind: ProviderConnectionKind,
    ) -> AcResult<Self> {
        let endpoint = endpoint.into();
        let model_name = model_name.into();
        if endpoint.trim().is_empty() || model_name.trim().is_empty() {
            return Err(AcError::validation(
                "PROVIDER-INVALID_CONNECTION_TEST",
                "endpoint and model name are required for a provider connection test",
            ));
        }
        if account.credential_ref.is_empty() {
            return Err(AcError::validation(
                "PROVIDER-ACCOUNT_NO_CREDENTIAL",
                "provider account has no credential reference",
            ));
        }
        Ok(Self {
            account,
            endpoint,
            model_name,
            kind,
        })
    }
}

/// Resolve a credential reference into a secret value.
///
/// Supported reference shapes:
/// - `env:NAME` -> read the environment variable
/// - `secret:NAME` -> read the `NAME` secret file from the AgentCode runtime
///   secret directory
/// - anything else -> error (never treat an unknown reference as a
///   credential value)
pub fn resolve_credential_ref(credential_ref: &str) -> AcResult<String> {
    let (kind, name) = credential_ref.split_once(':').ok_or_else(|| {
        AcError::validation(
            "PROVIDER-INVALID_CREDENTIAL_REF",
            "credential references must be env:NAME or secret:NAME",
        )
    })?;
    match kind {
        "env" => {
            if name.trim().is_empty() {
                return Err(AcError::validation(
                    "PROVIDER-INVALID_CREDENTIAL_REF",
                    "env credential reference must name a variable",
                ));
            }
            std::env::var(name).map_err(|_| {
                AcError::validation(
                    "PROVIDER-CREDENTIAL_UNAVAILABLE",
                    format!("environment credential {name} is not set"),
                )
            })
        }
        "secret" => {
            if name.trim().is_empty() {
                return Err(AcError::validation(
                    "PROVIDER-INVALID_CREDENTIAL_REF",
                    "secret credential reference must name a secret",
                ));
            }
            let dir = std::env::var("AGENTCODE_SECRET_DIR").map_err(|_| {
                AcError::validation(
                    "PROVIDER-SECRET_DIR_UNAVAILABLE",
                    "secret storage requires AGENTCODE_SECRET_DIR",
                )
            })?;
            let path = std::path::Path::new(&dir).join(name);
            std::fs::read_to_string(&path)
                .map(|value| value.trim_end_matches(['\n', '\r']).to_string())
                .map_err(|error| {
                    AcError::validation(
                        "PROVIDER-CREDENTIAL_UNAVAILABLE",
                        format!("secret {name} cannot be read: {error}"),
                    )
                })
        }
        _ => Err(AcError::validation(
            "PROVIDER-INVALID_CREDENTIAL_REF",
            "credential references must be env:NAME or secret:NAME",
        )),
    }
}

/// A real provider connection test.  Resolves the credential reference,
/// constructs the provider-specific request, authenticates, contacts the
/// provider, validates the response, and returns a normalized status.
/// The raw credential is never returned and never logged.
pub fn test_provider_account(
    test: &ProviderConnectionTest,
    cancel: &dyn Fn() -> bool,
) -> AcResult<ProviderAccountStatus> {
    let started = TimestampMillis::now();
    let account = &test.account;
    if !account.enabled {
        return Ok(ProviderAccountStatus {
            account_id: account.id.clone(),
            provider_id: account.provider_id.clone(),
            ok: false,
            health_state: "disabled".to_string(),
            latency_ms: 0,
            failure: None,
            masked_credential: account.masked_credential(),
            detail: "account is disabled".to_string(),
        });
    }
    if cancel() {
        return Err(AcError::new(
            "PROVIDER-CONNECTION_CANCELLED",
            "provider connection test cancelled",
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::NotRetryable,
        ));
    }
    let secret = match resolve_credential_ref(&account.credential_ref) {
        Ok(secret) => secret,
        Err(error) => {
            return Ok(ProviderAccountStatus {
                account_id: account.id.clone(),
                provider_id: account.provider_id.clone(),
                ok: false,
                health_state: "auth_unavailable".to_string(),
                latency_ms: 0,
                failure: Some(NormalizedProviderFailure::AuthFailed),
                masked_credential: account.masked_credential(),
                detail: error.to_string(),
            });
        }
    };
    let result = perform_connection_probe(test, &secret, cancel);
    let latency_ms = TimestampMillis::now()
        .as_millis()
        .saturating_sub(started.as_millis()) as u64;
    match result {
        Ok(()) => Ok(ProviderAccountStatus {
            account_id: account.id.clone(),
            provider_id: account.provider_id.clone(),
            ok: true,
            health_state: "ok".to_string(),
            latency_ms,
            failure: None,
            masked_credential: account.masked_credential(),
            detail: "provider connection verified".to_string(),
        }),
        Err(failure) => {
            let normalized = normalize_failure(failure);
            let detail = format!("provider connection failed: {normalized:?}");
            Ok(ProviderAccountStatus {
                account_id: account.id.clone(),
                provider_id: account.provider_id.clone(),
                ok: false,
                health_state: "failed".to_string(),
                latency_ms,
                failure: Some(normalized),
                masked_credential: account.masked_credential(),
                detail,
            })
        }
    }
}

fn perform_connection_probe(
    test: &ProviderConnectionTest,
    secret: &str,
    cancel: &dyn Fn() -> bool,
) -> Result<(), ProviderFailureClass> {
    if cancel() {
        return Err(ProviderFailureClass::Cancelled);
    }
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| ProviderFailureClass::ServerError)?;
    let request = connection_probe_request(test, secret);
    let response = client
        .post(&test.endpoint)
        .headers(connection_probe_headers(test, secret)?)
        .json(&request)
        .send()
        .map_err(map_reqwest_error)?;
    if cancel() {
        return Err(ProviderFailureClass::Cancelled);
    }
    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|_| ProviderFailureClass::MalformedResponse)?;
    validate_probe_response(test.kind, status, &body)
}

fn connection_probe_headers(
    test: &ProviderConnectionTest,
    secret: &str,
) -> Result<HeaderMap, ProviderFailureClass> {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    match test.kind {
        ProviderConnectionKind::AnthropicMessages => {
            headers.insert(
                "x-api-key",
                reqwest::header::HeaderValue::from_str(secret)
                    .map_err(|_| ProviderFailureClass::Auth)?,
            );
            headers.insert(
                "anthropic-version",
                reqwest::header::HeaderValue::from_static("2023-06-01"),
            );
        }
        ProviderConnectionKind::GeminiGenerateContent => {
            headers.insert(
                "x-goog-api-key",
                reqwest::header::HeaderValue::from_str(secret)
                    .map_err(|_| ProviderFailureClass::Auth)?,
            );
        }
        _ => {
            let value = format!("Bearer {secret}");
            headers.insert(
                AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&value)
                    .map_err(|_| ProviderFailureClass::Auth)?,
            );
        }
    }
    Ok(headers)
}

fn connection_probe_request(test: &ProviderConnectionTest, _secret: &str) -> Value {
    match test.kind {
        ProviderConnectionKind::OllamaChat | ProviderConnectionKind::OllamaTags => json!({
            "model": test.model_name,
            "messages": [{"role": "user", "content": "Reply with the single word OK."}],
            "stream": false
        }),
        ProviderConnectionKind::GeminiGenerateContent => json!({
            "contents": [{"role": "user", "parts": [{"text": "Reply with the single word OK."}]}],
            "generationConfig": {"maxOutputTokens": 4}
        }),
        _ => json!({
            "model": test.model_name,
            "messages": [{"role": "user", "content": "Reply with the single word OK."}],
            "max_tokens": 4,
            "stream": false
        }),
    }
}

fn validate_probe_response(
    _kind: ProviderConnectionKind,
    status: u16,
    body: &str,
) -> Result<(), ProviderFailureClass> {
    match status {
        200..=299 => {
            let value: Value =
                serde_json::from_str(body).map_err(|_| ProviderFailureClass::MalformedResponse)?;
            if let Some(failure) = provider_error_class_from_value(&value) {
                return Err(failure);
            }
            // Contact and authentication succeeded even if the model answers
            // "OK" or an equivalent.  A connection test proves the endpoint
            // and credential work; it does not assert on model content.
            Ok(())
        }
        401 | 403 => Err(ProviderFailureClass::Auth),
        408 | 504 => Err(ProviderFailureClass::Timeout),
        429 => Err(ProviderFailureClass::RateLimited),
        400..=499 => Err(ProviderFailureClass::InvalidRequest),
        _ => Err(ProviderFailureClass::ServerError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration as StdDuration;

    fn with_credential_env_lock<T>(f: impl FnOnce() -> T) -> T {
        let _guard = crate::PROVIDER_TEST_ENV_LOCK.lock().unwrap();
        f()
    }

    fn test_account() -> ProviderAccount {
        ProviderAccount {
            id: StableId::new("acct"),
            provider_id: StableId::from_existing("ollama").unwrap(),
            label: "test".to_string(),
            credential_ref: "env:TEST_PROVIDER_KEY".to_string(),
            credential_region: String::new(),
            organization: String::new(),
            project: String::new(),
            workspace: String::new(),
            enabled: true,
            health_state: "unknown".to_string(),
            quota_rate_limit: None,
            quota_remaining: None,
            quota_reset_at_ms: None,
            last_success_at_ms: None,
            last_failure_at_ms: None,
            failure_reason: String::new(),
        }
    }

    #[test]
    fn masked_credential_never_exposes_full_ref() {
        let account = test_account();
        let masked = account.masked_credential();
        assert!(!masked.contains("TEST_PROVIDER_KEY"));
        assert!(!masked.contains("env:"));
        assert!(masked.contains("****"));
        assert!(masked.ends_with("KEY"));
    }

    #[test]
    fn masked_credential_with_empty_ref_returns_fallback() {
        let mut account = test_account();
        account.credential_ref = String::new();
        assert_eq!(account.masked_credential(), "ref:****");
    }

    #[test]
    fn resolve_credential_env_works() {
        with_credential_env_lock(|| {
            std::env::set_var("AGENTCODE_TEST_CRED", "test-secret-value");
            let result = resolve_credential_ref("env:AGENTCODE_TEST_CRED").unwrap();
            assert_eq!(result, "test-secret-value");
            std::env::remove_var("AGENTCODE_TEST_CRED");
        });
    }

    #[test]
    fn resolve_credential_unknown_ref_is_error() {
        let err = resolve_credential_ref("plain:key").unwrap_err();
        assert_eq!(err.code(), "PROVIDER-INVALID_CREDENTIAL_REF");
    }

    #[test]
    fn resolve_credential_unset_env_is_error() {
        let err = resolve_credential_ref("env:UNSET_VAR_XYZ").unwrap_err();
        assert_eq!(err.code(), "PROVIDER-CREDENTIAL_UNAVAILABLE");
    }

    #[test]
    fn resolve_credential_secret_round_trip() {
        with_credential_env_lock(|| {
            let tmp =
                std::env::temp_dir().join(format!("ac-catalog-secret-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp).unwrap();
            std::env::set_var("AGENTCODE_SECRET_DIR", &tmp);
            std::fs::write(tmp.join("live-key"), "sk-live-value\n").unwrap();
            let resolved = resolve_credential_ref("secret:live-key").unwrap();
            assert_eq!(resolved, "sk-live-value");
            std::env::remove_var("AGENTCODE_SECRET_DIR");
            let _ = std::fs::remove_dir_all(tmp);
        });
    }

    #[test]
    fn resolve_credential_missing_secret_file_is_honest_error() {
        with_credential_env_lock(|| {
            let tmp =
                std::env::temp_dir().join(format!("ac-catalog-missing-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp).unwrap();
            std::env::set_var("AGENTCODE_SECRET_DIR", &tmp);
            let err = resolve_credential_ref("secret:no-such-secret").unwrap_err();
            assert_eq!(err.code(), "PROVIDER-CREDENTIAL_UNAVAILABLE");
            std::env::remove_var("AGENTCODE_SECRET_DIR");
            let _ = std::fs::remove_dir_all(tmp);
        });
    }

    #[test]
    fn account_is_healthy_returns_true_for_ok_state() {
        let mut account = test_account();
        account.health_state = "ok".to_string();
        assert!(account.is_healthy());
        account.health_state = "healthy".to_string();
        assert!(account.is_healthy());
        account.health_state = "failed".to_string();
        assert!(!account.is_healthy());
    }

    #[test]
    fn disabled_account_returns_disconnected_status() {
        let mut account = test_account();
        account.enabled = false;
        let test = ProviderConnectionTest::new(
            account.clone(),
            "http://127.0.0.1:11434/api/chat",
            "test-model",
            ProviderConnectionKind::OllamaChat,
        )
        .unwrap();
        let status = test_provider_account(&test, &|| false).unwrap();
        assert!(!status.ok);
        assert_eq!(status.health_state, "disabled");
    }

    #[test]
    fn test_provider_account_returns_auth_failure_for_unreachable_endpoint() {
        with_credential_env_lock(|| {
            std::env::set_var("TEST_PROVIDER_KEY", "test-key");
            let mut account = test_account();
            account.credential_ref = "env:TEST_PROVIDER_KEY".to_string();
            let test = ProviderConnectionTest::new(
                account.clone(),
                "http://127.0.0.1:1/nonexistent",
                "test",
                ProviderConnectionKind::OpenAiChatCompletions,
            )
            .unwrap();
            let status = test_provider_account(&test, &|| false).unwrap();
            // Connection failure should be a non-ok status, not a crash
            assert!(!status.ok);
            assert!(status.failure.is_some());
            assert!(status.masked_credential.contains("****"));
            std::env::remove_var("TEST_PROVIDER_KEY");
        });
    }

    #[test]
    fn test_provider_account_returns_auth_failure_on_401() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/chat", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            tx.send(()).unwrap();
            let response =
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
        });
        std::env::set_var("TEST_PROVIDER_KEY_401", "bad-key");
        let mut account = test_account();
        account.credential_ref = "env:TEST_PROVIDER_KEY_401".to_string();
        let test = ProviderConnectionTest::new(
            account.clone(),
            endpoint,
            "test-model",
            ProviderConnectionKind::OpenAiChatCompletions,
        )
        .unwrap();
        let status = test_provider_account(&test, &|| false).unwrap();
        rx.recv_timeout(StdDuration::from_secs(2)).unwrap();
        assert!(!status.ok);
        assert_eq!(status.failure, Some(NormalizedProviderFailure::AuthFailed));
        assert!(status.masked_credential.contains("****"));
        std::env::remove_var("TEST_PROVIDER_KEY_401");
    }

    #[test]
    fn test_provider_account_succeeds_on_200() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/chat", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            tx.send(()).unwrap();
            let body = r#"{"id":"ok","object":"chat.completion","choices":[{"message":{"content":"OK"},"finish_reason":"stop"}],"usage":{"prompt_tokens":4,"completion_tokens":1}}"#;
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            let _ = stream.write_all(response.as_bytes());
        });
        std::env::set_var("TEST_PROVIDER_KEY_OK", "good-key");
        let mut account = test_account();
        account.credential_ref = "env:TEST_PROVIDER_KEY_OK".to_string();
        let test = ProviderConnectionTest::new(
            account.clone(),
            endpoint,
            "test-model",
            ProviderConnectionKind::OpenAiChatCompletions,
        )
        .unwrap();
        let status = test_provider_account(&test, &|| false).unwrap();
        rx.recv_timeout(StdDuration::from_secs(2)).unwrap();
        assert!(status.ok, "connection test should succeed: {status:?}");
        assert_eq!(status.health_state, "ok");
        assert!(status.failure.is_none());
        assert!(status.masked_credential.contains("****"));
        std::env::remove_var("TEST_PROVIDER_KEY_OK");
    }

    #[test]
    fn provider_connection_test_rejects_empty_endpoint() {
        let account = test_account();
        let err =
            ProviderConnectionTest::new(account, "", "model", ProviderConnectionKind::OllamaChat)
                .unwrap_err();
        assert_eq!(err.code(), "PROVIDER-INVALID_CONNECTION_TEST");
    }

    #[test]
    fn provider_connection_test_rejects_empty_model_name() {
        let account = test_account();
        let err = ProviderConnectionTest::new(
            account,
            "http://127.0.0.1:11434",
            "",
            ProviderConnectionKind::OllamaChat,
        )
        .unwrap_err();
        assert_eq!(err.code(), "PROVIDER-INVALID_CONNECTION_TEST");
    }

    #[test]
    fn provider_catalog_entry_pricing() {
        let free = ProviderCatalogEntry {
            id: StableId::new("test"),
            display_name: "free".to_string(),
            description: String::new(),
            website_url: String::new(),
            logo_url: String::new(),
            credential_url: String::new(),
            pricing_classification: "free".to_string(),
            capabilities: Vec::new(),
        };
        assert!(free.pricing_is_free());
        assert!(!free.pricing_is_paid());

        let paid = ProviderCatalogEntry {
            pricing_classification: "paid".to_string(),
            ..free.clone()
        };
        assert!(paid.pricing_is_paid());
        assert!(!paid.pricing_is_free());
    }
}
