#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

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

fn provider_catalog(client: &Mutex<UnixIpcClient>) -> Result<Value, String> {
    // The backend daemon is the single source of truth for the provider
    // catalog, accounts, health, and model discovery.  The UI never decides
    // routing; it only renders what the backend reports.
    let response = request(client, "ui", "ListProviders", json!({}))?;
    let providers = response
        .get("providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut result = Vec::new();
    for provider in providers {
        let provider_id = provider
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let accounts_response = request(
            client,
            "ui",
            "ListProviderAccounts",
            json!({ "provider_id": provider_id }),
        )
        .ok();
        let accounts = accounts_response
            .and_then(|r| r.get("accounts").and_then(Value::as_array).cloned())
            .unwrap_or_default();
        let account_values = accounts
            .into_iter()
            .map(provider_account_json_from_daemon)
            .collect::<Vec<_>>();
        let category = provider
            .get("pricing_classification")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let local = category == "local";
        let paid = matches!(category.as_str(), "paid" | "freemium" | "tiered");
        let enabled_accounts = account_values
            .iter()
            .filter(|account| {
                account
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .count();
        let health = if local {
            let discovery = daemon_local_discovery(client, &provider_id);
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
        } else if !account_values.is_empty() {
            "disabled"
        } else {
            "unavailable"
        };
        let model_count = if local {
            daemon_local_discovery(client, &provider_id)
                .get("models")
                .and_then(Value::as_array)
                .map(|models| models.len())
                .unwrap_or(0)
        } else {
            1
        };
        result.push(json!({
            "id": provider_id,
            "name": provider.get("display_name").and_then(Value::as_str).unwrap_or(&provider_id),
            "description": provider.get("description").and_then(Value::as_str).unwrap_or(""),
            "category": category,
            "health": health,
            "connected_accounts": account_values.len(),
            "model_count": model_count,
            "website": provider.get("website_url").and_then(Value::as_str).unwrap_or(""),
            "credential_url": provider.get("credential_url").and_then(Value::as_str).unwrap_or(""),
            "local": local,
            "paid": paid,
            "accounts": account_values,
        }));
    }
    Ok(json!(result))
}

/// Map a backend `ListProviderAccounts`/`GetProviderAccount` account row to the
/// React `ProviderAccount` shape.  `credential_ref` from the daemon is already
/// masked; only safe metadata is forwarded.
fn provider_account_json_from_daemon(account: Value) -> Value {
    let health_state = account
        .get("health_state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let health = match health_state {
        "ok" | "healthy" => "healthy",
        "failed" | "auth_unavailable" => "auth_failed",
        "rate_limited" => "rate_limited",
        "disabled" => "disabled",
        _ => "unknown",
    };
    json!({
        "id": account.get("id").and_then(Value::as_str).unwrap_or(""),
        "provider_id": account.get("provider_id").and_then(Value::as_str),
        "label": account.get("label").and_then(Value::as_str).unwrap_or(""),
        "credential_masked": account.get("credential_ref").and_then(Value::as_str).unwrap_or(""),
        "organization": account.get("organization").and_then(Value::as_str),
        "project": account.get("project").and_then(Value::as_str),
        "workspace": account.get("workspace").and_then(Value::as_str),
        "health": health,
        "quota_state": account.get("quota_rate_limit").cloned().unwrap_or(Value::Null),
        "quota_remaining": account.get("quota_remaining").cloned(),
        "quota_reset_at_ms": account.get("quota_reset_at_ms").cloned(),
        "last_connected": account.get("last_success_at_ms").cloned(),
        "last_failure": account.get("last_failure_at_ms").cloned(),
        "enabled": account.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        "paid": !matches!(account.get("provider_id").and_then(Value::as_str), Some("ollama") | Some("lm-studio")),
        "local": matches!(account.get("provider_id").and_then(Value::as_str), Some("ollama") | Some("lm-studio")),
    })
}

/// Local model discovery delegated to the backend daemon (real `/api/tags` or
/// `/v1/models`).  Returns the React `OllamaStatus`/discovery shape.
fn daemon_local_discovery(client: &Mutex<UnixIpcClient>, provider_id: &str) -> Value {
    let endpoint = match provider_id {
        "ollama" => std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
        "lm-studio" => std::env::var("LMSTUDIO_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1234/v1".to_string()),
        _ => return json!({ "running": false, "models": [], "error": "not a local provider" }),
    };
    let kind = if provider_id == "ollama" {
        "ollama"
    } else {
        "openai"
    };
    match request(
        client,
        "ui",
        "DiscoverProviderModels",
        json!({ "endpoint": endpoint, "kind": kind }),
    ) {
        Ok(response) => {
            let models = response
                .get("models")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|m| {
                    let name = m
                        .get("model_name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let parameters = m.get("parameters").and_then(Value::as_str);
                    let capabilities = m
                        .get("capabilities")
                        .and_then(Value::as_array)
                        .map(|cap| {
                            cap.iter()
                                .filter_map(Value::as_str)
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_else(|| vec!["chat".to_string()]);
                    json!({
                        "name": name,
                        "size": parameters,
                        "capabilities": capabilities,
                        "loaded": true,
                        "discovered_at": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0),
                    })
                })
                .collect::<Vec<_>>();
            json!({ "running": !models.is_empty(), "models": models })
        }
        Err(error) => json!({
            "running": false,
            "models": [],
            "error": error,
        }),
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
fn daemon_submit_mission(
    state: tauri::State<DaemonClient>,
    goal: String,
    workspace_root: Option<String>,
) -> Result<Value, String> {
    let mut payload = json!({ "goal": goal });
    if let Some(root) = workspace_root {
        if root.trim().is_empty() {
            return Err(AcError::validation(
                "PROJECT-EMPTY_WORKSPACE",
                "workspace root is required",
            )
            .to_string());
        }
        payload["workspace_root"] = json!(root);
    }
    let response = request(&state.0, "ui", "SubmitMission", payload)?;
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
        "workspace_root": response.get("workspace_root").cloned().unwrap_or(Value::Null),
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
fn daemon_list_providers(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    provider_catalog(&state.0)
}

/// Store a credential in the backend-owned secret store and create a provider
/// account that references it.  The raw key transits only to the daemon's
/// secret store; the UI never receives or persists the raw secret after save.
#[tauri::command]
fn daemon_add_account(
    state: tauri::State<DaemonClient>,
    provider_id: String,
    label: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    workspace: Option<String>,
) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err(AcError::validation("ACCOUNT-EMPTY_KEY", "API key is required").to_string());
    }
    // 1. Persist the credential into the backend secret store, receiving a
    //    `secret:NAME` reference.  The raw key is not returned.
    let store_name = format!("{}/{}", provider_id, label);
    let store = request(
        &state.0,
        "ui",
        "StoreCredential",
        json!({ "name": store_name, "value": api_key }),
    )?;
    let credential_ref = store
        .get("credential_ref")
        .and_then(Value::as_str)
        .ok_or_else(|| "daemon did not return a credential reference".to_string())?
        .to_string();
    // 2. Real connection test through the backend before saving.
    let test = request(
        &state.0,
        "ui",
        "TestCredential",
        json!({
            "provider_id": provider_id,
            "credential_ref": credential_ref,
            "organization": organization.clone().unwrap_or_default(),
            "project": project.clone().unwrap_or_default(),
            "workspace": workspace.clone().unwrap_or_default(),
        }),
    )?;
    let healthy = test
        .get("healthy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !healthy {
        let _ = request(
            &state.0,
            "ui",
            "DeleteCredential",
            json!({ "name": store_name }),
        );
        let detail = test
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or("connection test failed");
        return Err(format!("ACCOUNT-AUTH_FAILED: {detail}"));
    }
    // 3. Create the account in the backend provider_accounts table.
    request(
        &state.0,
        "ui",
        "CreateProviderAccount",
        json!({
            "provider_id": provider_id,
            "label": label,
            "credential_ref": credential_ref,
            "credential_region": "",
            "organization": organization.unwrap_or_default(),
            "project": project.unwrap_or_default(),
            "workspace": workspace.unwrap_or_default(),
            "enabled": true,
        }),
    )?;
    Ok(())
}

/// Standalone real connection test through the backend daemon. Never saves
/// anything and never returns the raw secret: on failure it returns only the
/// normalized backend error.
#[tauri::command]
fn daemon_test_connection(
    state: tauri::State<DaemonClient>,
    provider_id: String,
    api_key: String,
    organization: Option<String>,
    project: Option<String>,
    workspace: Option<String>,
) -> Result<Value, String> {
    if api_key.trim().is_empty() {
        return Err(AcError::validation("ACCOUNT-EMPTY_KEY", "API key is required").to_string());
    }
    // The credential is stored in the backend secret store solely for the
    // duration of the real connection probe, then removed.  Nothing is saved.
    let temp_name = format!("tmp-{}-{}", provider_id, std::process::id());
    let store = request(
        &state.0,
        "ui",
        "StoreCredential",
        json!({ "name": temp_name, "value": api_key }),
    )?;
    let credential_ref = store
        .get("credential_ref")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let result = request(
        &state.0,
        "ui",
        "TestCredential",
        json!({
            "provider_id": provider_id,
            "credential_ref": credential_ref,
            "organization": organization.unwrap_or_default(),
            "project": project.unwrap_or_default(),
            "workspace": workspace.unwrap_or_default(),
        }),
    );
    let _ = request(
        &state.0,
        "ui",
        "DeleteCredential",
        json!({ "name": temp_name }),
    );
    match result {
        Ok(response) => {
            let healthy = response
                .get("healthy")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if healthy {
                Ok(json!({ "ok": true }))
            } else {
                let detail = response
                    .get("detail")
                    .and_then(Value::as_str)
                    .unwrap_or("connection test failed");
                Err(format!("ACCOUNT-AUTH_FAILED: {detail}"))
            }
        }
        Err(error) => Err(error),
    }
}

/// Toggle whether a saved account is eligible for routing.  Delegated to the
/// backend so the credential reference is never round-tripped through the UI.
#[tauri::command]
fn daemon_set_account_enabled(
    state: tauri::State<DaemonClient>,
    account_id: String,
    enabled: bool,
) -> Result<(), String> {
    request(
        &state.0,
        "ui",
        "SetProviderAccountEnabled",
        json!({ "account_id": account_id, "enabled": enabled }),
    )
    .map(|_| ())
}

/// Rotate a saved account's credential through the backend. The new key is
/// stored in the backend secret store and validated against the real provider
/// before the account's credential reference is updated; the raw key is never
/// returned or persisted by the UI.
#[tauri::command]
fn daemon_rotate_account(
    state: tauri::State<DaemonClient>,
    account_id: String,
    api_key: String,
) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err(AcError::validation("ACCOUNT-EMPTY_KEY", "API key is required").to_string());
    }
    let account = request(
        &state.0,
        "ui",
        "GetProviderAccount",
        json!({ "account_id": account_id }),
    )
    .map_err(|_| "ACCOUNT-NOT_FOUND: account is not saved".to_string())?;
    let account = account.get("account").cloned().unwrap_or(Value::Null);
    let provider_id = account
        .get("provider_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let store_name = format!(
        "{provider_id}/{}",
        account
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("rotated")
    );
    let store = request(
        &state.0,
        "ui",
        "StoreCredential",
        json!({ "name": store_name, "value": api_key }),
    )?;
    let credential_ref = store
        .get("credential_ref")
        .and_then(Value::as_str)
        .ok_or_else(|| "daemon did not return a credential reference".to_string())?
        .to_string();
    let test = request(
        &state.0,
        "ui",
        "TestCredential",
        json!({
            "provider_id": provider_id,
            "credential_ref": credential_ref,
            "organization": account.get("organization").and_then(Value::as_str).unwrap_or(""),
            "project": account.get("project").and_then(Value::as_str).unwrap_or(""),
            "workspace": account.get("workspace").and_then(Value::as_str).unwrap_or(""),
        }),
    )?;
    let healthy = test
        .get("healthy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !healthy {
        let _ = request(
            &state.0,
            "ui",
            "DeleteCredential",
            json!({ "name": store_name }),
        );
        return Err(
            "ACCOUNT-AUTH_FAILED: The API key was rejected. Your key has not been exposed."
                .to_string(),
        );
    }
    request(
        &state.0,
        "ui",
        "RotateProviderAccount",
        json!({ "account_id": account_id, "credential_ref": credential_ref }),
    )?;
    Ok(())
}

#[tauri::command]
fn daemon_remove_account(
    state: tauri::State<DaemonClient>,
    account_id: String,
) -> Result<(), String> {
    request(
        &state.0,
        "ui",
        "DeleteProviderAccount",
        json!({ "account_id": account_id }),
    )?;
    Ok(())
}

#[tauri::command]
fn daemon_discover_ollama(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    let endpoint =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let response = request(
        &state.0,
        "ui",
        "DiscoverProviderModels",
        json!({ "endpoint": endpoint, "kind": "ollama" }),
    )?;
    let models = response
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            json!({
                "name": m.get("model_name").and_then(Value::as_str).unwrap_or(""),
                "size": m.get("parameters").and_then(Value::as_str),
                "capabilities": m.get("capabilities").and_then(Value::as_array).cloned().unwrap_or_default(),
                "loaded": true,
                "discovered_at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "running": !models.is_empty(), "models": models }))
}

#[tauri::command]
fn daemon_refresh_ollama(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    daemon_discover_ollama(state)
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
