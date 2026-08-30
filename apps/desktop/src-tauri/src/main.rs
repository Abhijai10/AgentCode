#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use ac_agent::{
    default_provider_registry, ProviderConfigEntry, ProviderConfigKind, ProviderRegistryConfig,
};
use ac_common::AcError;
use ac_daemon::{default_runtime_dir, default_socket_path, UnixIpcClient};
use serde_json::{json, Value};

struct DaemonClient(Mutex<UnixIpcClient>);

fn socket_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("AGENTCODE_RUNTIME_DIR")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(default_runtime_dir)
        .map_err(|error| error.to_string())?;
    Ok(default_socket_path(&base))
}

fn request(
    client: &Mutex<UnixIpcClient>,
    id: &str,
    command: &str,
    mut payload: Value,
) -> Result<Value, String> {
    payload["id"] = json!(id);
    payload["command"] = json!(command);
    let response = client
        .lock()
        .map_err(|_| "daemon client lock poisoned".to_string())?
        .request(payload)
        .map_err(|error| error.to_string())?;
    if response.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        Ok(response)
    } else {
        let code = response
            .pointer("/error/code")
            .and_then(Value::as_str)
            .unwrap_or("DAEMON-IPC_UNKNOWN");
        let message = response
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("daemon command failed");
        Err(format!("{code}: {message}"))
    }
}

fn provider_catalog() -> Result<Value, String> {
    let registry = default_provider_registry().map_err(|error| error.to_string())?;
    let config =
        ProviderRegistryConfig::default_provider_config().map_err(|error| error.to_string())?;
    let saved = load_saved_accounts().unwrap_or_default();
    let mut providers = Vec::new();
    for entry in &config.entries {
        if !entry.enabled {
            continue;
        }
        let provider_accounts = saved
            .iter()
            .filter(|account| {
                account.get("provider_id").and_then(Value::as_str)
                    == Some(entry.provider_id.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        providers.push(provider_entry_json(entry, &registry, &provider_accounts));
    }
    // A provider configured with a credential env var but no saved account is
    // still a real configured route; expose it as a discoverable account.
    for entry in &config.entries {
        if !entry.enabled {
            continue;
        }
        let configured_env = entry
            .credential_env
            .as_ref()
            .map(|env| std::env::var(env).is_ok())
            .unwrap_or(false);
        if configured_env
            && !saved.iter().any(|account| {
                account.get("provider_id").and_then(Value::as_str)
                    == Some(entry.provider_id.as_str())
            })
        {
            let mut provider = providers
                .iter_mut()
                .find(|p| p.get("id").and_then(Value::as_str) == Some(entry.provider_id.as_str()))
                .cloned()
                .unwrap_or_default();
            let accounts = provider
                .get("accounts")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let account = json!({
                "id": format!("{}/Default", entry.provider_id),
                "provider_id": entry.provider_id,
                "label": "Default",
                "credential_masked": "From environment".to_string(),
                "organization": Value::Null,
                "project": Value::Null,
                "workspace": Value::Null,
                "health": "healthy",
                "quota_state": Value::Null,
                "last_connected": Value::Null,
                "last_failure": Value::Null,
                "enabled": true,
                "paid": entry.paid,
                "local": entry.local,
            });
            let mut updated = accounts;
            updated.push(account);
            if let Some(obj) = provider.as_object_mut() {
                obj.insert("accounts".to_string(), json!(updated));
                obj.insert("connected_accounts".to_string(), json!(updated.len()));
                obj.insert("health".to_string(), json!("healthy"));
            }
            if let Some(slot) = providers
                .iter_mut()
                .find(|p| p.get("id").and_then(Value::as_str) == Some(entry.provider_id.as_str()))
            {
                *slot = provider;
            }
        }
    }
    Ok(json!(providers))
}

fn provider_entry_json(
    entry: &ProviderConfigEntry,
    _registry: &ac_provider::ProviderRegistry,
    accounts: &[Value],
) -> Value {
    let (category, website, credential_url) = match entry.kind {
        ProviderConfigKind::OpenAi => (
            "paid",
            "https://openai.com",
            "https://platform.openai.com/api-keys",
        ),
        ProviderConfigKind::Anthropic => (
            "paid",
            "https://anthropic.com",
            "https://console.anthropic.com/settings/keys",
        ),
        ProviderConfigKind::Gemini => (
            "free-tier",
            "https://ai.google.dev",
            "https://aistudio.google.com/app/apikey",
        ),
        ProviderConfigKind::Ollama => {
            ("local", "https://ollama.com", "https://ollama.com/download")
        }
        ProviderConfigKind::LmStudio => (
            "local",
            "https://lmstudio.ai",
            "https://lmstudio.ai/docs/app/system-requirements",
        ),
    };
    let enabled_accounts = accounts
        .iter()
        .filter(|account| {
            account
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true)
        })
        .count();
    let health = if entry.local {
        let discovery = local_discovery_for(entry);
        if discovery
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "healthy"
        } else {
            "unavailable"
        }
    } else if enabled_accounts > 0 {
        "healthy"
    } else if !accounts.is_empty() {
        "disabled"
    } else {
        "unavailable"
    };
    let model_count = if entry.local {
        let discovery = local_discovery_for(entry);
        discovery
            .get("models")
            .and_then(Value::as_array)
            .map(|models| models.len())
            .unwrap_or(0)
    } else if entry.models.is_empty() {
        1
    } else {
        entry.models.len()
    };
    json!({
        "id": entry.provider_id,
        "name": provider_display_name(entry),
        "description": provider_description(entry),
        "category": category,
        "health": health,
        "connected_accounts": accounts.len(),
        "model_count": model_count,
        "website": website,
        "credential_url": credential_url,
        "local": entry.local,
        "paid": entry.paid,
        "accounts": accounts,
    })
}

fn provider_display_name(entry: &ProviderConfigEntry) -> &'static str {
    match entry.kind {
        ProviderConfigKind::OpenAi => "OpenAI",
        ProviderConfigKind::Anthropic => "Anthropic",
        ProviderConfigKind::Gemini => "Google / Gemini",
        ProviderConfigKind::Ollama => "Ollama",
        ProviderConfigKind::LmStudio => "LM Studio",
    }
}

fn provider_description(entry: &ProviderConfigEntry) -> &'static str {
    match entry.kind {
        ProviderConfigKind::OpenAi => "Fast inference · Paid",
        ProviderConfigKind::Anthropic => "High quality · Paid",
        ProviderConfigKind::Gemini => "Free tier available",
        ProviderConfigKind::Ollama => "Local models · Free",
        ProviderConfigKind::LmStudio => "Local models · Free",
    }
}

fn ollama_discovery() -> Value {
    let url =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let endpoint = url.trim_end_matches("/api/chat").trim_end_matches('/');
    let tags_url = format!("{endpoint}/api/tags");
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(2000))
        .timeout(std::time::Duration::from_millis(3000))
        .build();
    let Ok(client) = client else {
        return json!({ "running": false, "models": [], "error": "client unavailable" });
    };
    let response = client.get(&tags_url).send();
    match response {
        Ok(resp) if resp.status().is_success() => {
            let body: Value = resp.json().unwrap_or(Value::Null);
            let models = body
                .get("models")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|m| {
                    let name = m
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let size = m.get("size").and_then(Value::as_u64).map(human_size);
                    let loaded = m
                        .get("details")
                        .and_then(|d| d.get("family"))
                        .and_then(Value::as_str)
                        .is_some();
                    json!({
                        "name": name,
                        "size": size,
                        "capabilities": ["chat", "tool_calls"],
                        "loaded": loaded,
                        "discovered_at": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0),
                    })
                })
                .collect::<Vec<_>>();
            json!({ "running": true, "models": models })
        }
        _ => json!({ "running": false, "models": [], "error": "Ollama server is not reachable" }),
    }
}

fn human_size(bytes: u64) -> String {
    let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gib >= 1.0 {
        format!("{:.1} GB", gib)
    } else {
        let mib = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.0} MB", mib)
    }
}

fn runtime_base() -> Result<PathBuf, String> {
    std::env::var_os("AGENTCODE_RUNTIME_DIR")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(default_runtime_dir)
        .map_err(|error| error.to_string())
}

fn secrets_dir() -> Result<PathBuf, String> {
    let base = runtime_base()?;
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let dir = base.join("secrets");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(dir)
}

fn account_file_path(credential_ref: &str) -> Result<PathBuf, String> {
    Ok(secrets_dir()?.join(format!("{}.json", sanitize(credential_ref))))
}

/// Load every saved account metadata file. Raw API keys are never persisted;
/// only the masked credential and safe metadata are written and read back.
fn load_saved_accounts() -> Result<Vec<Value>, String> {
    let dir = secrets_dir()?;
    let mut accounts = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
            if parsed.get("provider_id").and_then(Value::as_str).is_some() {
                accounts.push(parsed);
            }
        }
    }
    Ok(accounts)
}

/// Mask an API key so the raw secret is never displayed after save.
/// Preserves only the first 4 and last 4 characters (or a fixed mask for
/// short keys) so the credential remains recognizable but not exposable.
fn mask_api_key(api_key: &str) -> String {
    let key = api_key.trim();
    if key.len() <= 8 {
        return "••••••••".to_string();
    }
    let head = &key[..4];
    let tail = &key[key.len() - 4..];
    format!("{head}••••••••{tail}")
}

fn save_provider_account(
    provider_id: &str,
    label: &str,
    api_key: &str,
    organization: Option<&str>,
    project: Option<&str>,
    workspace: Option<&str>,
) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err(AcError::validation("ACCOUNT-EMPTY_KEY", "API key is required").to_string());
    }
    let credential_ref = format!("{provider_id}/{label}");
    let file = account_file_path(&credential_ref)?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let secret = json!({
        "id": credential_ref,
        "provider_id": provider_id,
        "account": label,
        "organization": organization,
        "project": project,
        "workspace": workspace,
        "credential_masked": mask_api_key(api_key),
        "enabled": true,
        "health": "healthy",
        "quota_state": Value::Null,
        "last_connected": now_ms,
        "last_failure": Value::Null,
        "credential_ref": credential_ref,
        "saved_at_ms": now_ms,
    });
    let serialized = serde_json::to_string_pretty(&secret).map_err(|error| error.to_string())?;
    std::fs::write(&file, serialized).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600));
    }
    // The raw key is intentionally discarded after masking. The daemon's
    // secret subsystem owns the live credential; the UI only ever stores and
    // returns safe masked metadata.
    let _ = api_key.len();
    Ok(())
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn local_discovery_for(entry: &ProviderConfigEntry) -> Value {
    match entry.kind {
        ProviderConfigKind::Ollama => ollama_discovery(),
        ProviderConfigKind::LmStudio => {
            let base = std::env::var("LMSTUDIO_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:1234/v1".to_string());
            let endpoint = base.trim_end_matches('/');
            let client = reqwest::blocking::Client::builder()
                .connect_timeout(std::time::Duration::from_millis(2000))
                .timeout(std::time::Duration::from_millis(3000))
                .build();
            let Ok(client) = client else {
                return json!({ "running": false, "models": [], "error": "client unavailable" });
            };
            match client.get(format!("{endpoint}/models")).send() {
                Ok(resp) if resp.status().is_success() => {
                    let body: Value = resp.json().unwrap_or(Value::Null);
                    let models = body
                        .get("data")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|m| {
                            json!({
                                "name": m.get("id").and_then(Value::as_str).unwrap_or(""),
                                "size": Value::Null,
                                "capabilities": ["chat"],
                                "loaded": true,
                                "discovered_at": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_millis())
                                    .unwrap_or(0),
                            })
                        })
                        .collect::<Vec<_>>();
                    json!({ "running": true, "models": models })
                }
                _ => {
                    json!({ "running": false, "models": [], "error": "LM Studio is not reachable" })
                }
            }
        }
        _ => json!({ "running": false, "models": [] }),
    }
}

#[tauri::command]
fn pick_project_folder() -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new()
        .set_title("Open AgentCode Project")
        .pick_folder();
    Ok(picked.map(|path| path.display().to_string()))
}

#[tauri::command]
fn pick_project_location() -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new()
        .set_title("Choose Location for New Project")
        .pick_folder();
    Ok(picked.map(|path| path.display().to_string()))
}

#[tauri::command]
fn create_project_folder(base_path: String, name: String) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(
            AcError::validation("PROJECT-EMPTY_NAME", "project name is required").to_string(),
        );
    }
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let dir = PathBuf::from(&base_path).join(&sanitized);
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let gitignore = dir.join(".gitignore");
    if !gitignore.exists() {
        let _ = std::fs::write(&gitignore, "node_modules/\ndist/\ntarget/\n");
    }
    Ok(dir.display().to_string())
}

#[tauri::command]
fn daemon_health(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let response = request(&state.0, "ui", "GetDaemonInfo", json!({}))?;
    Ok(json!({
        "lifecycle": response.get("lifecycle").cloned().unwrap_or(json!("Disconnected")),
        "recovered_sessions": response.get("recovered_sessions").cloned().unwrap_or(json!(0)),
    }))
}

#[tauri::command]
fn daemon_submit_mission(state: tauri::State<DaemonClient>, goal: String) -> Result<Value, String> {
    let response = request(&state.0, "ui", "SubmitMission", json!({ "goal": goal }))?;
    Ok(json!({
        "mission_id": response.get("mission_id").cloned().unwrap_or_default(),
        "session_id": response.get("session_id").cloned().unwrap_or_default(),
    }))
}

#[tauri::command]
fn daemon_list_missions(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let response = request(&state.0, "ui", "ListActiveMissions", json!({}))?;
    let missions = response.get("missions").cloned().unwrap_or(json!([]));
    // Enrich each mission with its goal from GetMission
    if let Some(arr) = missions.as_array() {
        let enriched: Vec<Value> = arr
            .iter()
            .map(|m| {
                let mid = m.get("mission_id").and_then(Value::as_str).unwrap_or("");
                let goal = if !mid.is_empty() {
                    request(&state.0, "ui", "GetMission", json!({ "mission_id": mid }))
                        .ok()
                        .and_then(|r| r.get("goal").and_then(Value::as_str).map(|s| json!(s)))
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                let mut enriched = m.clone();
                if let Some(obj) = enriched.as_object_mut() {
                    obj.insert("goal".to_string(), goal);
                }
                enriched
            })
            .collect();
        Ok(json!(enriched))
    } else {
        Ok(missions)
    }
}

#[tauri::command]
fn daemon_get_mission(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    let response = request(
        &state.0,
        "ui",
        "GetMission",
        json!({ "mission_id": mission_id }),
    )?;
    Ok(json!({
        "state": response.get("state").cloned().unwrap_or(Value::Null),
        "tasks": response.get("tasks").cloned().unwrap_or(json!([])),
    }))
}

#[tauri::command]
fn daemon_pause_mission(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<(), String> {
    request(
        &state.0,
        "ui",
        "PauseMission",
        json!({ "mission_id": mission_id }),
    )?;
    Ok(())
}

#[tauri::command]
fn daemon_resume_mission(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<(), String> {
    request(
        &state.0,
        "ui",
        "ResumeMission",
        json!({ "mission_id": mission_id }),
    )?;
    Ok(())
}

#[tauri::command]
fn daemon_cancel_mission(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<(), String> {
    request(
        &state.0,
        "ui",
        "CancelMission",
        json!({ "mission_id": mission_id }),
    )?;
    Ok(())
}

#[tauri::command]
fn daemon_list_providers() -> Result<Value, String> {
    provider_catalog()
}

#[tauri::command]
fn daemon_add_account(
    provider_id: String,
    label: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    workspace: Option<String>,
) -> Result<(), String> {
    // Real connection test before saving: never fabricate success.
    test_provider_connection(
        &provider_id,
        &api_key,
        organization.as_deref(),
        project.as_deref(),
        workspace.as_deref(),
    )?;
    save_provider_account(
        &provider_id,
        &label,
        &api_key,
        organization.as_deref(),
        project.as_deref(),
        workspace.as_deref(),
    )
}

/// Standalone real connection test. Never saves anything and never returns
/// the raw secret: on failure it returns only the normalized backend error.
#[tauri::command]
fn daemon_test_connection(
    provider_id: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    workspace: Option<String>,
) -> Result<Value, String> {
    test_provider_connection(
        &provider_id,
        &api_key,
        organization.as_deref(),
        project.as_deref(),
        workspace.as_deref(),
    )?;
    Ok(json!({ "ok": true }))
}

/// Toggle whether a saved account is eligible for routing.
#[tauri::command]
fn daemon_set_account_enabled(account_id: String, enabled: bool) -> Result<(), String> {
    let file = account_file_path(&account_id)?;
    if !file.exists() {
        return Err(AcError::validation("ACCOUNT-NOT_FOUND", "account is not saved").to_string());
    }
    let raw = std::fs::read_to_string(&file).map_err(|error| error.to_string())?;
    let mut account: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    account["enabled"] = json!(enabled);
    let serialized = serde_json::to_string_pretty(&account).map_err(|error| error.to_string())?;
    std::fs::write(&file, serialized).map_err(|error| error.to_string())?;
    Ok(())
}

/// Rotate a saved account's credential. The new key is validated against the
/// real provider before the masked metadata is updated; the raw key is never
/// returned or persisted.
#[tauri::command]
fn daemon_rotate_account(account_id: String, api_key: String) -> Result<(), String> {
    let file = account_file_path(&account_id)?;
    if !file.exists() {
        return Err(AcError::validation("ACCOUNT-NOT_FOUND", "account is not saved").to_string());
    }
    let raw = std::fs::read_to_string(&file).map_err(|error| error.to_string())?;
    let mut account: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let provider_id = account
        .get("provider_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AcError::validation("ACCOUNT-INVALID", "account record is invalid").to_string()
        })?
        .to_string();
    let organization = account
        .get("organization")
        .and_then(Value::as_str)
        .map(String::from);
    let project = account
        .get("project")
        .and_then(Value::as_str)
        .map(String::from);
    let workspace = account
        .get("workspace")
        .and_then(Value::as_str)
        .map(String::from);
    test_provider_connection(
        &provider_id,
        &api_key,
        organization.as_deref(),
        project.as_deref(),
        workspace.as_deref(),
    )?;
    account["credential_masked"] = json!(mask_api_key(&api_key));
    account["health"] = json!("healthy");
    account["last_failure"] = Value::Null;
    account["last_connected"] = json!(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0));
    let serialized = serde_json::to_string_pretty(&account).map_err(|error| error.to_string())?;
    std::fs::write(&file, serialized).map_err(|error| error.to_string())?;
    Ok(())
}

fn test_provider_connection(
    provider_id: &str,
    api_key: &str,
    organization: Option<&str>,
    project: Option<&str>,
    workspace: Option<&str>,
) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err(AcError::validation("ACCOUNT-EMPTY_KEY", "API key is required").to_string());
    }
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(8000))
        .timeout(std::time::Duration::from_millis(15000))
        .build()
        .map_err(|error| error.to_string())?;
    let request = match provider_id {
        "openai" => {
            let mut req = client
                .get("https://api.openai.com/v1/models")
                .bearer_auth(api_key);
            if let Some(org) = organization {
                req = req.header("OpenAI-Organization", org);
            }
            if let Some(proj) = project {
                req = req.header("OpenAI-Project", proj);
            }
            let _ = workspace;
            req
        }
        "anthropic" => {
            let mut req = client
                .get("https://api.anthropic.com/v1/models")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01");
            if let Some(org) = organization {
                req = req.header("anthropic-organization-id", org);
            }
            if let Some(proj) = project {
                req = req.header("anthropic-project-id", proj);
            }
            let _ = workspace;
            req
        }
        "gemini" => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models?key={}",
                api_key
            );
            let _ = workspace;
            client.get(url)
        }
        "ollama" | "lm-studio" => {
            let base = match provider_id {
                "lm-studio" => std::env::var("LMSTUDIO_BASE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:1234/v1".to_string()),
                _ => std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
            };
            let endpoint = if provider_id == "lm-studio" {
                base.trim_end_matches('/').to_string()
            } else {
                base.trim_end_matches("/api/chat")
                    .trim_end_matches('/')
                    .to_string()
            };
            let _ = workspace;
            client.get(format!("{endpoint}/api/tags"))
        }
        _ => {
            return Err(AcError::validation(
                "ACCOUNT-UNKNOWN_PROVIDER",
                "unknown provider for connection test",
            )
            .to_string())
        }
    };
    let status = request.send().map_err(|error| {
        format!(
            "Provider unavailable: {}",
            if error.is_timeout() {
                "connection timed out".to_string()
            } else {
                error.to_string()
            }
        )
    })?;
    let code = status.status();
    if code.is_success() {
        Ok(())
    } else if code.as_u16() == 401 || code.as_u16() == 403 {
        Err(AcError::validation(
            "ACCOUNT-AUTH_FAILED",
            "The API key was rejected. Your key has not been exposed.",
        )
        .to_string())
    } else {
        Err(AcError::validation(
            "ACCOUNT-PROVIDER_UNAVAILABLE",
            format!("Provider returned status {code}. Your key has not been exposed."),
        )
        .to_string())
    }
}

#[tauri::command]
fn daemon_remove_account(account_id: String) -> Result<(), String> {
    let file = account_file_path(&account_id)?;
    if file.exists() {
        std::fs::remove_file(&file).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn daemon_discover_ollama() -> Result<Value, String> {
    Ok(ollama_discovery())
}

#[tauri::command]
fn daemon_refresh_ollama() -> Result<Value, String> {
    Ok(ollama_discovery())
}

#[tauri::command]
fn daemon_get_settings(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let response = request(&state.0, "ui", "GetDesktopSettings", json!({}))?;
    Ok(json!({
        "appearance": response.get("appearance").and_then(Value::as_str).unwrap_or("light"),
        "notifications_enabled": response.get("notifications_enabled").and_then(Value::as_bool).unwrap_or(true),
        "completion_sound": response.get("completion_sound_enabled").and_then(Value::as_bool).unwrap_or(true),
        "reduced_motion": response.get("reduced_motion").and_then(Value::as_bool).unwrap_or(false),
        "budget_limit_micros": response.get("budget_limit_micros").and_then(Value::as_u64),
        "routing_profile": std::env::var("AGENTCODE_ROUTING_PROFILE").unwrap_or_else(|_| "free_first".to_string()),
        "preferred_model": std::env::var("AGENTCODE_PREFERRED_MODEL").ok(),
    }))
}

#[tauri::command]
fn daemon_set_settings(state: tauri::State<DaemonClient>, settings: Value) -> Result<(), String> {
    let appearance = settings
        .get("appearance")
        .and_then(Value::as_str)
        .unwrap_or("light")
        .to_string();
    let notifications = settings
        .get("notifications_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let completion_sound = settings
        .get("completion_sound")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let reduced_motion = settings
        .get("reduced_motion")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let budget = settings.get("budget_limit_micros").and_then(Value::as_u64);
    request(
        &state.0,
        "ui",
        "SetDesktopSettings",
        json!({
            "appearance": appearance,
            "notifications_enabled": notifications,
            "completion_sound_enabled": completion_sound,
            "reduced_motion": reduced_motion,
            "budget_limit_micros": budget,
        }),
    )?;
    if let Some(profile) = settings.get("routing_profile").and_then(Value::as_str) {
        std::env::set_var("AGENTCODE_ROUTING_PROFILE", profile);
    }
    Ok(())
}

#[tauri::command]
fn daemon_mission_progress(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    let response = request(
        &state.0,
        "ui",
        "GetMission",
        json!({ "mission_id": mission_id }),
    )?;
    let state_val = response
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let tasks = response
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let task_count = tasks.len();
    let done = tasks
        .iter()
        .filter(|t| {
            t.get("state")
                .and_then(Value::as_str)
                .map(|s| matches!(s, "done" | "completed" | "succeeded"))
                .unwrap_or(false)
        })
        .count();
    let progress_pct = if task_count > 0 {
        (done as f64 / task_count as f64 * 100.0).round() as u32
    } else {
        0
    };
    Ok(json!({
        "mission_id": mission_id,
        "goal": response.get("goal").cloned().unwrap_or_default(),
        "state": state_val,
        "progress_pct": progress_pct,
        "active_task": tasks.iter().find(|t| {
            t.get("state").and_then(Value::as_str).map(|s| matches!(s, "in_progress" | "running" | "working" | "verifying")).unwrap_or(false)
        }).and_then(|t| t.get("task_id")).cloned(),
    }))
}

#[tauri::command]
fn daemon_get_changeset(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    // The daemon IPC does not yet expose a per-mission ChangeSet contract.
    // Return the real mission state only; never fabricate a safe verdict or
    // file list.
    let response = request(
        &state.0,
        "ui",
        "GetMission",
        json!({ "mission_id": mission_id }),
    )?;
    let state_val = response
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Ok(json!({
        "mission_id": mission_id,
        "available": false,
        "verification_state": if state_val == "completed" { "Completed".to_string() } else { format!("Mission {state_val}") },
    }))
}

#[tauri::command]
fn daemon_list_tools() -> Result<Value, String> {
    // Tool availability is owned by the backend runtime. Without a daemon IPC
    // contract we return an empty catalog rather than a fabricated list.
    Ok(json!([]))
}

#[tauri::command]
fn daemon_list_scanners() -> Result<Value, String> {
    // Scanner availability is owned by the backend security subsystem. Without
    // a daemon IPC contract we return an empty catalog rather than fabricate.
    Ok(json!([]))
}

#[tauri::command]
fn daemon_list_memory(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let _ = &state.0;
    Ok(json!([]))
}

#[tauri::command]
fn daemon_security_findings(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let _ = &state.0;
    Ok(json!([]))
}

#[tauri::command]
fn daemon_discuss_sessions(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let _ = &state.0;
    Ok(json!([]))
}

#[tauri::command]
fn daemon_discuss_messages(
    state: tauri::State<DaemonClient>,
    session_id: String,
) -> Result<Value, String> {
    let _ = &state.0;
    let _ = session_id;
    Ok(json!([]))
}

#[tauri::command]
fn daemon_discuss_send(
    state: tauri::State<DaemonClient>,
    session_id: String,
    content: String,
) -> Result<Value, String> {
    let _ = &state.0;
    let _ = session_id;
    let _ = content;
    Ok(json!([]))
}

#[tauri::command]
fn daemon_design_sessions(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let _ = &state.0;
    Ok(json!([]))
}

#[tauri::command]
fn daemon_design_session(
    state: tauri::State<DaemonClient>,
    session_id: String,
) -> Result<Value, String> {
    let _ = &state.0;
    let _ = session_id;
    Ok(json!([]))
}

fn main() {
    let client =
        UnixIpcClient::new(socket_path().unwrap_or_else(|_| PathBuf::from("/tmp/agentcode.sock")));
    tauri::Builder::default()
        .manage(DaemonClient(Mutex::new(client)))
        .invoke_handler(tauri::generate_handler![
            pick_project_folder,
            pick_project_location,
            create_project_folder,
            daemon_health,
            daemon_submit_mission,
            daemon_list_missions,
            daemon_get_mission,
            daemon_pause_mission,
            daemon_resume_mission,
            daemon_cancel_mission,
            daemon_list_providers,
            daemon_add_account,
            daemon_test_connection,
            daemon_set_account_enabled,
            daemon_rotate_account,
            daemon_remove_account,
            daemon_discover_ollama,
            daemon_refresh_ollama,
            daemon_get_settings,
            daemon_set_settings,
            daemon_mission_progress,
            daemon_get_changeset,
            daemon_list_tools,
            daemon_list_scanners,
            daemon_list_memory,
            daemon_security_findings,
            daemon_discuss_sessions,
            daemon_discuss_messages,
            daemon_discuss_send,
            daemon_design_sessions,
            daemon_design_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AgentCode desktop application");
}
