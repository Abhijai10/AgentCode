use std::time::Duration;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};
use serde_json::Value;

use crate::catalog::DiscoveredModel;

/// Discover models from a provider's real discovery API.
///
/// Supported sources:
/// - Ollama: `GET {base}/api/tags` -> `{"models": [{"name": "...", ...}]}`
/// - LM Studio / OpenAI-compatible: `GET {base}/v1/models` -> `{"data": [{"id": "..."}]}`
///
/// The list is never fabricated from installed names; it comes from the
/// provider endpoint.  When discovery is unsupported or unreachable, the
/// caller must decide whether to treat the provider as unavailable rather
/// than inventing model data.
pub fn discover_models(
    endpoint_base: &str,
    discovery_kind: ModelDiscoveryKind,
    connect_timeout_ms: u64,
    read_timeout_ms: u64,
) -> AcResult<Vec<DiscoveredModel>> {
    if endpoint_base.trim().is_empty() {
        return Err(AcError::validation(
            "PROVIDER-DISCOVERY_NO_ENDPOINT",
            "model discovery requires a provider endpoint",
        ));
    }
    let url = match discovery_kind {
        ModelDiscoveryKind::Ollama => join_url(endpoint_base, "api/tags"),
        ModelDiscoveryKind::OpenAiCompatible => join_url(endpoint_base, "v1/models"),
    };
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_millis(connect_timeout_ms.max(1000)))
        .timeout(Duration::from_millis(read_timeout_ms.max(5000)))
        .build()
        .map_err(|error| AcError::validation("PROVIDER-DISCOVERY_CLIENT", error.to_string()))?;
    let response = client.get(&url).send().map_err(|error| {
        AcError::new(
            "PROVIDER-DISCOVERY_UNREACHABLE",
            format!("model discovery failed: {error}"),
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::Retryable,
        )
    })?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|error| AcError::validation("PROVIDER-DISCOVERY_READ", error.to_string()))?;
    if !(200..=299).contains(&status) {
        return Err(AcError::new(
            "PROVIDER-DISCOVERY_HTTP",
            format!("model discovery returned HTTP {status}"),
            ac_common::ErrorKind::Unavailable,
            ac_common::Retryability::Retryable,
        ));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| AcError::validation("PROVIDER-DISCOVERY_MALFORMED", error.to_string()))?;
    let provider_id = StableId::new("provider");
    match discovery_kind {
        ModelDiscoveryKind::Ollama => parse_ollama_models(&value, provider_id),
        ModelDiscoveryKind::OpenAiCompatible => parse_openai_models(&value, provider_id),
    }
}

fn join_url(base: &str, suffix: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    let suffix = suffix.trim_start_matches('/');
    if trimmed.ends_with(suffix) {
        trimmed.to_string()
    } else if trimmed.is_empty() {
        suffix.to_string()
    } else {
        format!("{trimmed}/{suffix}")
    }
}

fn parse_ollama_models(value: &Value, provider_id: StableId) -> AcResult<Vec<DiscoveredModel>> {
    let models = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AcError::validation(
                "PROVIDER-DISCOVERY_MALFORMED",
                "Ollama model discovery response is missing models array",
            )
        })?;
    if models.is_empty() {
        return Err(AcError::validation(
            "PROVIDER-DISCOVERY_EMPTY",
            "Ollama reports no loaded models",
        ));
    }
    models
        .iter()
        .map(|model| {
            let name = model.get("name").and_then(Value::as_str).ok_or_else(|| {
                AcError::validation(
                    "PROVIDER-DISCOVERY_MALFORMED",
                    "Ollama model entry is missing name",
                )
            })?;
            let details = model.get("details");
            let context_window = details
                .and_then(|details| details.get("context_length"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let parameters = details
                .and_then(|details| details.get("parameter_size"))
                .and_then(Value::as_str)
                .map(ToString::to_string);
            Ok(DiscoveredModel {
                id: StableId::new("model"),
                provider_id: provider_id.clone(),
                model_name: name.to_string(),
                capabilities: model_capabilities_from_name(name),
                context_window,
                parameters,
            })
        })
        .collect()
}

fn parse_openai_models(value: &Value, provider_id: StableId) -> AcResult<Vec<DiscoveredModel>> {
    let data = value.get("data").and_then(Value::as_array).ok_or_else(|| {
        AcError::validation(
            "PROVIDER-DISCOVERY_MALFORMED",
            "OpenAI-compatible model discovery response is missing data array",
        )
    })?;
    if data.is_empty() {
        return Err(AcError::validation(
            "PROVIDER-DISCOVERY_EMPTY",
            "provider reports no models",
        ));
    }
    data.iter()
        .map(|model| {
            let id = model.get("id").and_then(Value::as_str).ok_or_else(|| {
                AcError::validation("PROVIDER-DISCOVERY_MALFORMED", "model entry is missing id")
            })?;
            Ok(DiscoveredModel {
                id: StableId::new("model"),
                provider_id: provider_id.clone(),
                model_name: id.to_string(),
                capabilities: model_capabilities_from_name(id),
                context_window: None,
                parameters: None,
            })
        })
        .collect()
}

/// Rough capability inference from the model name.  Capabilities are
/// normalized hints only; the runtime still requires the model to actually
/// support the requested behavior and never fabricates a working model list.
fn model_capabilities_from_name(name: &str) -> Vec<String> {
    let lower = name.to_ascii_lowercase();
    let mut capabilities = vec!["chat".to_string()];
    if lower.contains("coder") || lower.contains("code") {
        capabilities.push("tools".to_string());
    }
    if lower.contains("vision") || lower.contains("vl") {
        capabilities.push("vision".to_string());
    }
    capabilities
}

/// Timestamp helper kept explicit so discovery results can be recorded with a
/// stable observation time by callers.
pub fn now_millis() -> u128 {
    TimestampMillis::now().as_millis()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDiscoveryKind {
    Ollama,
    OpenAiCompatible,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ollama_discovery_parses_real_api_shape() {
        let payload = json!({
            "models": [
                {"name": "qwen3:4b", "details": {"context_length": 32768, "parameter_size": "4.0B"}},
                {"name": "qwen2.5-coder:3b", "details": {"context_length": 32768, "parameter_size": "3.1B"}}
            ]
        });
        let models = parse_ollama_models(&payload, StableId::new("provider")).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_name, "qwen3:4b");
        assert_eq!(models[0].context_window, Some(32768));
        assert_eq!(models[0].parameters.as_deref(), Some("4.0B"));
        assert!(models[0].capabilities.contains(&"chat".to_string()));
    }

    #[test]
    fn ollama_discovery_rejects_missing_models_array() {
        let err =
            parse_ollama_models(&json!({"not_models": []}), StableId::new("provider")).unwrap_err();
        assert_eq!(err.code(), "PROVIDER-DISCOVERY_MALFORMED");
    }

    #[test]
    fn ollama_discovery_rejects_empty_model_list() {
        let err =
            parse_ollama_models(&json!({"models": []}), StableId::new("provider")).unwrap_err();
        assert_eq!(err.code(), "PROVIDER-DISCOVERY_EMPTY");
    }

    #[test]
    fn openai_compatible_discovery_parses_id_list() {
        let payload = json!({"data": [{"id": "gpt-4o-mini"}, {"id": "custom-model"}]});
        let models = parse_openai_models(&payload, StableId::new("provider")).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[1].model_name, "custom-model");
    }

    #[test]
    fn join_url_handles_trailing_slashes_and_existing_suffix() {
        assert_eq!(
            join_url("http://127.0.0.1:11434/", "api/tags"),
            "http://127.0.0.1:11434/api/tags"
        );
        assert_eq!(
            join_url("http://127.0.0.1:11434/api/tags", "api/tags"),
            "http://127.0.0.1:11434/api/tags"
        );
    }
}
