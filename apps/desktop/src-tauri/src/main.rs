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
    // Enrich each task with the real backend task details (title, state,
    // dependencies, attempts) so the plan view reflects authoritative state.
    let task_details = request(
        &state.0,
        "ui",
        "GetTaskDetails",
        json!({ "mission_id": mission_id }),
    )
    .ok()
    .and_then(|r| r.get("tasks").and_then(Value::as_array).cloned())
    .unwrap_or_default();
    Ok(json!({
        "state": response.get("state").cloned().unwrap_or(Value::Null),
        "tasks": task_details,
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
fn daemon_terminal_start(
    state: tauri::State<DaemonClient>,
    mission_id: Option<String>,
    argv: Vec<String>,
    cwd: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "TerminalStart",
        json!({ "mission_id": mission_id, "argv": argv, "cwd": cwd }),
    )
}

#[tauri::command]
fn daemon_terminal_tail(
    state: tauri::State<DaemonClient>,
    session_id: String,
    cursor: Option<u64>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "TerminalTail",
        json!({ "session_id": session_id, "cursor": cursor.unwrap_or(0) }),
    )
}

#[tauri::command]
fn daemon_terminal_cancel(
    state: tauri::State<DaemonClient>,
    session_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "TerminalCancel",
        json!({ "session_id": session_id }),
    )
}

#[tauri::command]
fn daemon_terminal_list(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    request(&state.0, "ui", "TerminalList", json!({}))
}

#[tauri::command]
fn daemon_mission_export(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "MissionExport",
        json!({ "mission_id": mission_id }),
    )
}

#[tauri::command]
fn daemon_readiness_get(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    request(&state.0, "ui", "ReadinessGet", json!({}))
}

#[tauri::command]
fn daemon_list_providers(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    provider_catalog(&state.0)
}

/// Batch N1: persisted provider/routing preferences (display + set; all
/// routing logic remains backend-owned — the UI never routes).
#[tauri::command]
fn daemon_provider_preferences_get(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    request(&state.0, "ui", "ProviderPreferencesGet", json!({}))
}

#[tauri::command]
fn daemon_provider_preferences_set(
    state: tauri::State<DaemonClient>,
    routing_profile: String,
    preferred_model: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ProviderPreferencesSet",
        json!({ "routing_profile": routing_profile, "preferred_model": preferred_model }),
    )
}

/// Batch N1 (G2): real failover/health evidence for the Providers panel.
#[tauri::command]
fn daemon_provider_health_get(state: tauri::State<DaemonClient>) -> Result<Value, String> {
    request(&state.0, "ui", "ProviderHealthGet", json!({}))
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
    // Desktop-appearance and notification settings are backed by the daemon's
    // SetDesktopSettings/GetDesktopSettings IPC.  Model routing and preferred
    // model are NOT exposed by the backend IPC in this build — the daemon
    // determines routing from its own provider config and per-provider env
    // vars (OLLAMA_MODEL, OPENAI_MODEL, etc.).  Return them as null rather
    // than inventing a local preference the daemon silently ignores.
    let response = request(&state.0, "ui", "GetDesktopSettings", json!({}))?;
    Ok(json!({
        "appearance": response.get("appearance").and_then(Value::as_str).unwrap_or("light"),
        "notifications_enabled": response.get("notifications_enabled").and_then(Value::as_bool).unwrap_or(true),
        "completion_sound": response.get("completion_sound_enabled").and_then(Value::as_bool).unwrap_or(true),
        "reduced_motion": response.get("reduced_motion").and_then(Value::as_bool).unwrap_or(false),
        "budget_limit_micros": response.get("budget_limit_micros").and_then(Value::as_u64),
        "routing_profile": null,
        "preferred_model": null,
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
    Ok(())
}

#[tauri::command]
fn daemon_mission_progress(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    // Use the real mission snapshot so progress/counts/current task come from
    // authoritative backend state rather than frontend derivation.
    let response = request(
        &state.0,
        "ui",
        "GetMissionDetails",
        json!({ "mission_id": mission_id }),
    )?;
    let state_val = response
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let task_count = response
        .get("task_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let progress = response
        .get("progress")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let progress_pct = if task_count > 0 {
        (progress * 100.0).round() as u32
    } else {
        0
    };
    Ok(json!({
        "mission_id": mission_id,
        "goal": response.get("goal").cloned().unwrap_or_default(),
        "state": state_val,
        "progress_pct": progress_pct,
        "active_task": response.get("current_task").cloned(),
        "terminal": response.get("terminal").cloned().unwrap_or(json!(false)),
        "failure_code": response.get("failure_code").cloned(),
    }))
}

#[tauri::command]
fn daemon_get_changeset(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    let response = request(
        &state.0,
        "ui",
        "GetChangeSetSummary",
        json!({ "mission_id": mission_id }),
    )?;
    Ok(response)
}

#[tauri::command]
fn daemon_get_mission_details(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "GetMissionDetails",
        json!({ "mission_id": mission_id }),
    )
}

#[tauri::command]
fn daemon_get_task_details(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "GetTaskDetails",
        json!({ "mission_id": mission_id }),
    )
}

#[tauri::command]
fn daemon_get_mission_events(
    state: tauri::State<DaemonClient>,
    mission_id: String,
    limit: Option<u64>,
) -> Result<Value, String> {
    let mut payload = json!({ "mission_id": mission_id });
    if let Some(l) = limit {
        payload["limit"] = json!(l);
    }
    request(&state.0, "ui", "GetMissionEvents", payload)
}

#[tauri::command]
fn daemon_get_evidence_summary(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "GetEvidenceSummary",
        json!({ "mission_id": mission_id }),
    )
}

#[tauri::command]
fn daemon_project_memory_get(
    state: tauri::State<DaemonClient>,
    project_path: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ProjectMemoryGet",
        json!({ "project_path": project_path }),
    )
}

#[tauri::command]
fn daemon_get_verification_summary(
    state: tauri::State<DaemonClient>,
    mission_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "GetVerificationSummary",
        json!({ "mission_id": mission_id }),
    )
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
fn daemon_security_findings(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityFindings",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_security_send(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: String,
    attachment_ids: Option<Vec<String>>,
) -> Result<Value, String> {
    let mut payload = json!({
        "conversation_id": conversation_id,
        "content": content,
        "attachment_ids": attachment_ids.unwrap_or_default(),
    });
    if payload["attachment_ids"] == Value::Null {
        payload["attachment_ids"] = json!([]);
    }
    request(&state.0, "ui", "SecuritySend", payload)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn daemon_security_scope_set(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    target: String,
    scope_kind: String,
    auth_state: String,
    allowed_hosts: Vec<String>,
    allowed_ports: Vec<u16>,
    allowed_techniques: Vec<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityScopeSet",
        json!({
            "conversation_id": conversation_id,
            "target": target,
            "scope_kind": scope_kind,
            "auth_state": auth_state,
            "allowed_hosts": allowed_hosts,
            "allowed_ports": allowed_ports,
            "allowed_techniques": allowed_techniques,
        }),
    )
}

#[tauri::command]
fn daemon_security_audit(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    depth: Option<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityAudit",
        json!({
            "conversation_id": conversation_id,
            "depth": depth.unwrap_or_else(|| "quick".to_string()),
        }),
    )
}

#[tauri::command]
fn daemon_security_finding_detail(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    finding_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityFindingDetail",
        json!({ "conversation_id": conversation_id, "finding_id": finding_id }),
    )
}

#[tauri::command]
fn daemon_security_finding_transition(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    finding_id: String,
    target_state: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityFindingTransition",
        json!({ "conversation_id": conversation_id, "finding_id": finding_id, "target_state": target_state }),
    )
}

#[tauri::command]
fn daemon_security_validate(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    finding_id: String,
    canary: Option<String>,
) -> Result<Value, String> {
    let mut payload = json!({
        "conversation_id": conversation_id,
        "finding_id": finding_id,
    });
    if let Some(canary) = canary {
        payload["canary"] = json!(canary);
    }
    request(&state.0, "ui", "SecurityValidate", payload)
}

#[tauri::command]
fn daemon_security_attack_paths(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityAttackPaths",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_security_remediate(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    finding_id: String,
    approved: bool,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityRemediate",
        json!({ "conversation_id": conversation_id, "finding_id": finding_id, "approved": approved }),
    )
}

#[tauri::command]
fn daemon_security_retest(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityRetest",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_security_suppress(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    finding_id: String,
    reason: String,
    expires_at_ms: Option<i64>,
    applicability: Option<String>,
    compensating_controls: Option<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecuritySuppress",
        json!({
            "conversation_id": conversation_id,
            "finding_id": finding_id,
            "reason": reason,
            "expires_at_ms": expires_at_ms,
            "applicability": applicability.unwrap_or_default(),
            "compensating_controls": compensating_controls.unwrap_or_default(),
        }),
    )
}

#[tauri::command]
fn daemon_security_accept_risk(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    finding_id: String,
    rationale: String,
    approver: String,
    expires_at_ms: Option<i64>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityAcceptRisk",
        json!({
            "conversation_id": conversation_id,
            "finding_id": finding_id,
            "rationale": rationale,
            "approver": approver,
            "expires_at_ms": expires_at_ms,
        }),
    )
}

#[tauri::command]
fn daemon_security_report(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityReport",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_security_status(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityStatus",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_security_secret_lifecycle(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    finding_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecuritySecretLifecycle",
        json!({ "conversation_id": conversation_id, "finding_id": finding_id }),
    )
}

#[tauri::command]
fn daemon_security_quality_metrics(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "SecurityQualityMetrics",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_conversation_create(
    state: tauri::State<DaemonClient>,
    project_path: String,
    mode: Option<String>,
    title: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ConversationCreate",
        json!({
            "project_path": project_path,
            "mode": mode.unwrap_or_else(|| "GOAL".to_string()),
            "title": title,
        }),
    )
}

#[tauri::command]
fn daemon_conversation_list(
    state: tauri::State<DaemonClient>,
    project_path: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ConversationList",
        json!({ "project_path": project_path }),
    )
}

#[tauri::command]
fn daemon_conversation_get(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ConversationGet",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_conversation_rename(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    title: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ConversationRename",
        json!({ "conversation_id": conversation_id, "title": title }),
    )
}

#[tauri::command]
fn daemon_conversation_archive(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ConversationArchive",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_conversation_delete(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ConversationDelete",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_message_append(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    role: String,
    content: String,
    mission_ref: Option<String>,
) -> Result<Value, String> {
    let mut payload = json!({
        "conversation_id": conversation_id,
        "role": role,
        "content": content,
        "mission_ref": mission_ref,
    });
    if payload["mission_ref"] == Value::Null {
        payload["mission_ref"] = Value::Null;
    }
    request(&state.0, "ui", "MessageAppend", payload)
}

#[tauri::command]
fn daemon_goal_submit(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    goal: String,
    attachment_ids: Option<Vec<String>>,
) -> Result<Value, String> {
    let mut payload = json!({
        "conversation_id": conversation_id,
        "goal": goal,
        "attachment_ids": attachment_ids.unwrap_or_default(),
    });
    if payload["attachment_ids"] == Value::Null {
        payload["attachment_ids"] = json!([]);
    }
    request(&state.0, "ui", "GoalSubmit", payload)
}

/// Pick a file via the native dialog, persist it inside the project workspace
/// under `.agentcode/attachments/`, and register its metadata with the daemon.
/// The raw file bytes never enter localStorage or frontend state; only safe
/// metadata is returned.  Path validation (workspace boundary) is enforced by
/// the daemon before the row is stored.
#[tauri::command]
fn daemon_add_attachment(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    project_path: String,
) -> Result<Value, String> {
    let picked = rfd::FileDialog::new()
        .set_title("Attach a File")
        .pick_file()
        .ok_or_else(|| "ATTACH-CANCELLED: no file selected".to_string())?;
    let source_path = std::path::PathBuf::from(&picked);
    let filename = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "ATTACH-INVALID_PATH: invalid file path".to_string())?;
    let bytes =
        std::fs::read(&source_path).map_err(|error| format!("ATTACH-READ_FAILED: {error}"))?;
    if bytes.is_empty() {
        return Err("ATTACH-EMPTY: file is empty".to_string());
    }
    if bytes.len() > 25 * 1024 * 1024 {
        return Err("ATTACH-TOO_LARGE: attachment exceeds 25MB limit".to_string());
    }
    let mime = guess_mime(filename);
    // Persist inside the project workspace under .agentcode/attachments/
    let attachments_dir = std::path::Path::new(&project_path)
        .join(".agentcode")
        .join("attachments")
        .join(&conversation_id);
    std::fs::create_dir_all(&attachments_dir)
        .map_err(|error| format!("ATTACH-MKDIR_FAILED: {error}"))?;
    let unique = format!("{}-{}", chrono_like_id(), filename);
    let rel_path = format!(".agentcode/attachments/{conversation_id}/{unique}");
    let target = std::path::Path::new(&project_path).join(&rel_path);
    std::fs::write(&target, &bytes).map_err(|error| format!("ATTACH-WRITE_FAILED: {error}"))?;
    // Compute FNV-1a 64 hash (matches daemon content hash format)
    let content_hash = fnv1a64_hex(&bytes);
    request(
        &state.0,
        "ui",
        "AttachmentRegister",
        json!({
            "conversation_id": conversation_id,
            "project_path": project_path,
            "filename": filename,
            "mime_type": mime,
            "size_bytes": bytes.len() as i64,
            "content_hash": content_hash,
            "rel_path": rel_path,
        }),
    )
}

#[tauri::command]
fn daemon_attachment_path(
    state: tauri::State<DaemonClient>,
    attachment_id: String,
    project_path: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "AttachmentPath",
        json!({ "attachment_id": attachment_id, "project_path": project_path }),
    )
}

#[tauri::command]
fn daemon_attachment_list(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "AttachmentList",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_attachment_remove(
    state: tauri::State<DaemonClient>,
    attachment_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "AttachmentRemove",
        json!({ "attachment_id": attachment_id }),
    )
}

#[tauri::command]
fn daemon_conversation_activity(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "ConversationActivity",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_discuss_send(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: String,
    attachment_ids: Option<Vec<String>>,
) -> Result<Value, String> {
    let mut payload = json!({
        "conversation_id": conversation_id,
        "content": content,
        "attachment_ids": attachment_ids.unwrap_or_default(),
    });
    if payload["attachment_ids"] == Value::Null {
        payload["attachment_ids"] = json!([]);
    }
    request(&state.0, "ui", "DiscussSend", payload)
}

#[tauri::command]
fn daemon_discuss_turn_into_plan(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DiscussTurnIntoPlan",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_discuss_execute_plan(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DiscussExecutePlan",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_discuss_accept_decision(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    message_id: String,
    decision: String,
    rationale: Option<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DiscussAcceptDecision",
        json!({
            "conversation_id": conversation_id,
            "message_id": message_id,
            "decision": decision,
            "rationale": rationale.unwrap_or_default(),
        }),
    )
}

#[tauri::command]
fn daemon_discuss_plan_get(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DiscussPlanGet",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_discuss_decisions_get(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DiscussDecisionsGet",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_send(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: String,
    attachment_ids: Option<Vec<String>>,
) -> Result<Value, String> {
    let mut payload = json!({
        "conversation_id": conversation_id,
        "content": content,
        "attachment_ids": attachment_ids.unwrap_or_default(),
    });
    if payload["attachment_ids"] == Value::Null {
        payload["attachment_ids"] = json!([]);
    }
    request(&state.0, "ui", "DesignSend", payload)
}

#[tauri::command]
fn daemon_design_understand(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignUnderstand",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_analyze_reference(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    attachment_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignAnalyzeReference",
        json!({ "conversation_id": conversation_id, "attachment_id": attachment_id }),
    )
}

#[tauri::command]
fn daemon_design_brief(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    audience: String,
    workflow: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignBrief",
        json!({ "conversation_id": conversation_id, "audience": audience, "workflow": workflow }),
    )
}

#[tauri::command]
fn daemon_design_grammar(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignGrammar",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_state(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignState",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_critique(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: String,
    doc_type: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignCritique",
        json!({ "conversation_id": conversation_id, "content": content, "doc_type": doc_type }),
    )
}

#[tauri::command]
fn daemon_design_repair(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: String,
    doc_type: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignRepair",
        json!({ "conversation_id": conversation_id, "content": content, "doc_type": doc_type }),
    )
}

#[tauri::command]
fn daemon_design_preview_start(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignPreviewStart",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_preview_status(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignPreviewStatus",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_preview_stop(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignPreviewStop",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_browser(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    url: String,
    html: String,
    deterministic: bool,
    viewport_hint: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignBrowser",
        json!({
            "conversation_id": conversation_id,
            "url": url,
            "html": html,
            "deterministic": deterministic,
            "viewport_hint": viewport_hint,
        }),
    )
}

#[tauri::command]
fn daemon_design_qa_responsive(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: Option<String>,
    url: Option<String>,
    html: Option<String>,
    deterministic: Option<bool>,
    viewport_hint: Option<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignQAResponsive",
        json!({
            "conversation_id": conversation_id,
            "content": content.unwrap_or_default(),
            "url": url.unwrap_or_default(),
            "html": html.unwrap_or_default(),
            "deterministic": deterministic.unwrap_or(false),
            "viewport_hint": viewport_hint.unwrap_or_else(|| "desktop".to_string()),
        }),
    )
}

#[tauri::command]
fn daemon_design_qa_accessibility(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: Option<String>,
    url: Option<String>,
    html: Option<String>,
    deterministic: Option<bool>,
    viewport_hint: Option<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignQAAccessibility",
        json!({
            "conversation_id": conversation_id,
            "content": content.unwrap_or_default(),
            "url": url.unwrap_or_default(),
            "html": html.unwrap_or_default(),
            "deterministic": deterministic.unwrap_or(false),
            "viewport_hint": viewport_hint.unwrap_or_else(|| "desktop".to_string()),
        }),
    )
}

#[tauri::command]
fn daemon_design_qa_functional(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    content: Option<String>,
    url: Option<String>,
    html: Option<String>,
    deterministic: Option<bool>,
    viewport_hint: Option<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignQAFunctional",
        json!({
            "conversation_id": conversation_id,
            "content": content.unwrap_or_default(),
            "url": url.unwrap_or_default(),
            "html": html.unwrap_or_default(),
            "deterministic": deterministic.unwrap_or(false),
            "viewport_hint": viewport_hint.unwrap_or_else(|| "desktop".to_string()),
        }),
    )
}

#[tauri::command]
fn daemon_design_qa_report(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    url: Option<String>,
    html: Option<String>,
    deterministic: Option<bool>,
    viewport_hint: Option<String>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignQAReport",
        json!({
            "conversation_id": conversation_id,
            "url": url.unwrap_or_default(),
            "html": html.unwrap_or_default(),
            "deterministic": deterministic.unwrap_or(false),
            "viewport_hint": viewport_hint.unwrap_or_else(|| "desktop".to_string()),
        }),
    )
}

#[tauri::command]
fn daemon_design_visual_critique(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    url: Option<String>,
    deterministic: Option<bool>,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignVisualCritique",
        json!({
            "conversation_id": conversation_id,
            "url": url.unwrap_or_default(),
            "deterministic": deterministic.unwrap_or(false),
        }),
    )
}

#[tauri::command]
fn daemon_design_contract(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignContract",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_execute_contract(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignExecuteContract",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_constraints_set(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
    constraints: Value,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignConstraintsSet",
        json!({
            "conversation_id": conversation_id,
            "constraints": constraints,
        }),
    )
}

#[tauri::command]
fn daemon_design_constraints_get(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignConstraintsGet",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_materialize_state(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignMaterializeState",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_memory_get(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignMemoryGet",
        json!({ "conversation_id": conversation_id }),
    )
}

#[tauri::command]
fn daemon_design_iterations(
    state: tauri::State<DaemonClient>,
    conversation_id: String,
) -> Result<Value, String> {
    request(
        &state.0,
        "ui",
        "DesignIterations",
        json!({ "conversation_id": conversation_id }),
    )
}

fn guess_mime(filename: &str) -> String {
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if lower.ends_with(".pdf") {
        "application/pdf".to_string()
    } else if lower.ends_with(".md") {
        "text/markdown".to_string()
    } else if lower.ends_with(".json") {
        "application/json".to_string()
    } else if lower.ends_with(".txt")
        || lower.ends_with(".rs")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".py")
        || lower.ends_with(".go")
        || lower.ends_with(".c")
        || lower.ends_with(".h")
        || lower.ends_with(".cpp")
        || lower.ends_with(".toml")
        || lower.ends_with(".yml")
        || lower.ends_with(".yaml")
        || lower.ends_with(".sh")
        || lower.ends_with(".css")
        || lower.ends_with(".html")
    {
        "text/plain".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

/// FNV-1a 64-bit hash hex string, matching the daemon's attachment hash.
fn fnv1a64_hex(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("fnv1a64:{hash:016x}")
}

/// Micro timestamp used to keep staged attachment filenames unique.
fn chrono_like_id() -> String {
    format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0)
    )
}

/// F7 (final audit): the app must not require a terminal running the
/// daemon.  If the socket is dead, spawn a daemon as a detached child:
///  1. the binary bundled beside the app (packaged builds),
///  2. the workspace target dir (dev runs: `cargo tauri dev` runs from
///     apps/desktop/src-tauri, the workspace target/ is four levels up).
///
/// Honest on failure: log to stderr and continue — the UI already shows a
/// clean daemon-down state and the user can start one manually; the shell
/// never crashes on a spawn problem.
fn ensure_daemon_running() {
    let socket = match socket_path() {
        Ok(path) => path,
        Err(_) => return,
    };
    // Alive probe: a successful connect means a daemon is serving.
    if std::os::unix::net::UnixStream::connect(&socket).is_ok() {
        return;
    }
    // Stale socket file from a dead daemon: remove it so the new daemon can
    // bind (mirrors the daemon's own stale-socket recovery).
    let _ = std::fs::remove_file(&socket);
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
    let Some(exe_dir) = exe_dir else {
        return;
    };
    let candidates = [
        exe_dir.join("ac-daemon"),
        exe_dir
            .join("../../../../..")
            .join("target")
            .join("debug")
            .join("ac-daemon"),
        exe_dir
            .join("../../../../..")
            .join("target")
            .join("release")
            .join("ac-daemon"),
    ];
    let binary = candidates
        .into_iter()
        .find(|p| p.exists())
        .map(|p| p.canonicalize().unwrap_or(p));
    let Some(binary) = binary else {
        eprintln!(
            "AgentCode: no daemon serving at {} and no ac-daemon binary found beside the app or in the workspace target dir",
            socket.display()
        );
        return;
    };
    match std::process::Command::new(&binary)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            // Hold the handle on a parked thread so the daemon is not reaped
            // into a zombie while the app runs; never wait() — the app must
            // outlive daemon restarts.
            std::thread::spawn(move || {
                let _ = child;
                std::thread::park();
            });
            // Give the daemon a moment to bind before the first health poll.
            for _ in 0..50 {
                if std::os::unix::net::UnixStream::connect(&socket).is_ok() {
                    eprintln!("AgentCode: spawned daemon at {}", binary.display());
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            eprintln!(
                "AgentCode: daemon spawned but socket {} did not become ready",
                socket.display()
            );
        }
        Err(error) => {
            eprintln!(
                "AgentCode: could not spawn daemon {}: {error}",
                binary.display()
            );
        }
    }
}

fn main() {
    // F7 (final audit): sidecar daemon lifecycle — if no daemon is serving
    // the socket, spawn one (dev builds: the cargo-built binary next to the
    // app; packaged builds: the bundled binary).  This closes the launch
    // gap where users had to run `cargo run -p ac-daemon` in a terminal.
    ensure_daemon_running();
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
            daemon_terminal_start,
            daemon_terminal_tail,
            daemon_terminal_cancel,
            daemon_terminal_list,
            daemon_mission_export,
            daemon_readiness_get,
            daemon_provider_preferences_get,
            daemon_provider_preferences_set,
            daemon_provider_health_get,
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
            daemon_get_mission_details,
            daemon_get_task_details,
            daemon_get_mission_events,
            daemon_get_evidence_summary,
            daemon_project_memory_get,
            daemon_get_verification_summary,
            daemon_list_tools,
            daemon_list_scanners,
            daemon_list_memory,
            daemon_security_findings,
            daemon_conversation_create,
            daemon_conversation_list,
            daemon_conversation_get,
            daemon_conversation_rename,
            daemon_conversation_archive,
            daemon_conversation_delete,
            daemon_message_append,
            daemon_goal_submit,
            daemon_add_attachment,
            daemon_attachment_path,
            daemon_attachment_list,
            daemon_attachment_remove,
            daemon_conversation_activity,
            daemon_discuss_send,
            daemon_discuss_turn_into_plan,
            daemon_discuss_execute_plan,
            daemon_discuss_accept_decision,
            daemon_discuss_plan_get,
            daemon_discuss_decisions_get,
            daemon_design_send,
            daemon_design_understand,
            daemon_design_analyze_reference,
            daemon_design_brief,
            daemon_design_grammar,
            daemon_design_state,
            daemon_design_critique,
            daemon_design_repair,
            daemon_design_preview_start,
            daemon_design_preview_status,
            daemon_design_preview_stop,
            daemon_design_browser,
            daemon_design_qa_responsive,
            daemon_design_qa_report,
            daemon_design_visual_critique,
            daemon_design_contract,
            daemon_design_execute_contract,
            daemon_design_constraints_set,
            daemon_design_constraints_get,
            daemon_design_materialize_state,
            daemon_design_memory_get,
            daemon_design_iterations,
            daemon_design_qa_accessibility,
            daemon_design_qa_functional,
            daemon_security_send,
            daemon_security_scope_set,
            daemon_security_audit,
            daemon_security_finding_detail,
            daemon_security_finding_transition,
            daemon_security_validate,
            daemon_security_attack_paths,
            daemon_security_remediate,
            daemon_security_retest,
            daemon_security_suppress,
            daemon_security_accept_risk,
            daemon_security_report,
            daemon_security_status,
            daemon_security_secret_lifecycle,
            daemon_security_quality_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AgentCode desktop application");
}
