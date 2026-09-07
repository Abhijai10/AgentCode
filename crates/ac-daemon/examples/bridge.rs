// Diagnostics bridge: reproduces the Tauri webview exactly — serves the
// production dist with a `window.__TAURI_INTERNALS__` shim whose invoke
// forwards over HTTP to the REAL daemon's framed-IPC socket.  Loading this
// in a real browser exercises the true open-path (real data, real shapes)
// where a renderer crash becomes visible in the console diagnostics.
use ac_daemon::UnixIpcClient;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Mutex;

fn handle_invoke(client: &Mutex<UnixIpcClient>, payload: Value) -> Value {
    let mut payload = payload;
    payload["id"] = json!("web-bridge");
    let Ok(guard) = client.lock() else {
        return json!({"ok": false, "error": {"code": "BRIDGE-LOCK", "message": "poisoned"}});
    };
    match guard.request(payload.clone()) {
        Ok(response) => {
            println!(
                "[bridge] {} -> ok={}",
                payload["command"],
                response
                    .get("ok")
                    .cloned()
                    .unwrap_or(serde_json::json!(null))
            );
            response
        }
        Err(error) => {
            println!("[bridge] {} IPC-ERR {error}", payload["command"]);
            json!({"ok": false, "error": {"code": "BRIDGE-IPC", "message": error.to_string()}})
        }
    }
}

fn main() {
    let runtime = std::env::var("AGENTCODE_RUNTIME_DIR").unwrap_or_else(|_| {
        format!(
            "{}/Library/Application Support/AgentCode/runtime",
            std::env::var("HOME").expect("HOME")
        )
    });
    let socket = PathBuf::from(runtime).join("agentcode.sock");
    let dist_root =
        PathBuf::from(std::env::var("AGENTCODE_DIST").expect("AGENTCODE_DIST (apps/desktop/dist)"));
    let client = Mutex::new(UnixIpcClient::new(socket));
    let listener = TcpListener::bind("127.0.0.1:4180").expect("bind 4180");
    println!("[bridge] dist {dist_root:?} + daemon on http://127.0.0.1:4180");

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut buffer = [0u8; 16384];
        let mut raw = Vec::new();
        // read headers (+ small bodies) until we have the whole request
        loop {
            let n = match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            raw.extend_from_slice(&buffer[..n]);
            if let Some(header_end) = find_subslice(&raw, b"\r\n\r\n") {
                let text = String::from_utf8_lossy(&raw).to_string();
                let content_length = text
                    .lines()
                    .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
                    .and_then(|line| line.split(':').nth(1))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if raw.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&raw).to_string();
        let first_line = text.lines().next().unwrap_or("").to_string();
        let mut parts = first_line.split(' ');
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("/").to_string();
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .unwrap_or_default();

        let (status, content_type, content): (&str, &str, Vec<u8>) = if method == "POST"
            && path == "/invoke"
        {
            let payload: Value = serde_json::from_str(&body).unwrap_or(json!({}));
            let cmd = payload
                .get("cmd")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let args = payload.get("args").cloned().unwrap_or_else(|| json!({}));
            // Replicate the src-tauri command wrappers 1:1 for the
            // open-path commands (same IPC calls, same transforms).
            let response = match cmd.as_str() {
                "daemon_health" => {
                    let r = handle_invoke(&client, json!({"command": "GetDaemonInfo"}));
                    json!({
                        "lifecycle": r.get("lifecycle").cloned().unwrap_or(json!("Disconnected")),
                        "recovered_sessions": r.get("recovered_sessions").cloned().unwrap_or(json!(0)),
                    })
                }
                "daemon_list_missions" => {
                    let r = handle_invoke(&client, json!({"command": "ListActiveMissions"}));
                    let missions = r.get("missions").cloned().unwrap_or(json!([]));
                    if let Some(arr) = missions.as_array() {
                        let enriched: Vec<Value> = arr
                            .iter()
                            .map(|m| {
                                let mid = m.get("mission_id").and_then(Value::as_str).unwrap_or("");
                                let goal = handle_invoke(
                                    &client,
                                    json!({"command": "GetMission", "mission_id": mid}),
                                )
                                .get("goal")
                                .cloned()
                                .unwrap_or(Value::Null);
                                let mut e = m.clone();
                                if let Some(obj) = e.as_object_mut() {
                                    obj.insert("goal".to_string(), goal);
                                }
                                e
                            })
                            .collect();
                        json!(enriched)
                    } else {
                        missions
                    }
                }
                "daemon_readiness_get" => {
                    let r = handle_invoke(&client, json!({"command": "ReadinessGet"}));
                    json!({ "readiness": r.get("readiness").cloned().unwrap_or(Value::Null) })
                }
                "daemon_project_memory_get" => {
                    let root = args.get("root").and_then(Value::as_str).unwrap_or("");
                    let r = handle_invoke(
                        &client,
                        json!({"command": "ProjectMemoryGet", "repository_root": root}),
                    );
                    json!({ "memory": r.get("memory").cloned().unwrap_or(Value::Null) })
                }
                "daemon_events_subscribe" => {
                    let r = handle_invoke(&client, {
                        let mut f = json!({"command": "EventsSubscribe", "wait_ms": 300});
                        if let Some(v) = args.get("afterCreatedAtMs") {
                            f["after_created_at_ms"] = v.clone();
                        }
                        if let Some(v) = args.get("afterId") {
                            f["after_id"] = v.clone();
                        }
                        f
                    });
                    r
                }
                "daemon_conversation_changes_cursor" => {
                    let mut f = json!({"command": "ConversationChangesCursor"});
                    if let Some(v) = args.get("conversationId") {
                        f["conversation_id"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                // The shim unwraps {ok, result} and the frontend expects the
                // FULL daemon response shape (with session/list/tail/panel
                // fields), so these all pass handle_invoke's response through
                // 1:1 — exactly like the src-tauri wrappers do.
                "daemon_browser_panel" => {
                    let mut f = json!({"command": "BrowserPanel"});
                    if let Some(v) = args.get("action") {
                        f["action"] = v.clone();
                    }
                    if let Some(v) = args.get("url") {
                        f["url"] = v.clone();
                    }
                    if let Some(v) = args.get("viewportHint") {
                        f["viewport_hint"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                "daemon_agent_browse_status" => {
                    handle_invoke(&client, json!({"command": "AgentBrowseStatus"}))
                }
                // Shim convention: plain values pass through unwrapped
                // (matching the daemon_health pattern); daemon-shaped
                // responses keep their own {ok,...} envelope.
                "user_home_dir" => json!(std::env::var("HOME").unwrap_or_default()),
                "daemon_e2e_run" => {
                    let mut f = json!({"command": "E2ERun"});
                    if let Some(v) = args.get("conversationId") {
                        f["conversation_id"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                "daemon_e2e_fix" => {
                    let mut f = json!({"command": "E2EFix"});
                    if let Some(v) = args.get("conversationId") {
                        f["conversation_id"] = v.clone();
                    }
                    if let Some(v) = args.get("reportId") {
                        f["report_id"] = v.clone();
                    }
                    if let Some(v) = args.get("approved") {
                        f["approved"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                "daemon_e2e_reports" => {
                    let mut f = json!({"command": "E2EReports"});
                    if let Some(v) = args.get("conversationId") {
                        f["conversation_id"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                "daemon_terminal_start" => {
                    let mut f = json!({"command": "TerminalStart"});
                    if let Some(v) = args.get("missionId") {
                        f["mission_id"] = v.clone();
                    }
                    if let Some(v) = args.get("argv") {
                        f["argv"] = v.clone();
                    }
                    if let Some(v) = args.get("cwd") {
                        f["cwd"] = v.clone();
                    }
                    if let Some(v) = args.get("mode") {
                        f["mode"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                "daemon_terminal_list" => {
                    handle_invoke(&client, json!({"command": "TerminalList"}))
                }
                "daemon_terminal_tail" => {
                    let mut f = json!({"command": "TerminalTail"});
                    if let Some(v) = args.get("sessionId") {
                        f["session_id"] = v.clone();
                    }
                    if let Some(v) = args.get("cursor") {
                        f["cursor"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                "daemon_terminal_cancel" => {
                    let mut f = json!({"command": "TerminalCancel"});
                    if let Some(v) = args.get("sessionId") {
                        f["session_id"] = v.clone();
                    }
                    handle_invoke(&client, f)
                }
                other => {
                    println!("[bridge] UNMAPPED tauri command: {other}");
                    json!({"ok": false, "error": {"code": "BRIDGE-UNMAPPED", "message": format!("unmapped tauri command {other}")}})
                }
            };
            let ok = response.get("ok").and_then(Value::as_bool).unwrap_or(true);
            let wrapped = if ok {
                json!({"ok": true, "result": response})
            } else {
                response
            };
            (
                "200 OK",
                "application/json",
                serde_json::to_string(&wrapped)
                    .unwrap_or_default()
                    .into_bytes(),
            )
        } else if method == "GET" {
            let rel = path.trim_start_matches('/');
            let rel = if rel.is_empty() { "index.html" } else { rel };
            let candidate = dist_root.join(rel);
            let resolved_root = dist_root
                .canonicalize()
                .unwrap_or_else(|_| dist_root.clone());
            let contents = match candidate.canonicalize() {
                Ok(resolved) if resolved.starts_with(&resolved_root) => {
                    std::fs::read(resolved).ok()
                }
                _ => None,
            };
            match contents {
                Some(bytes) if rel.ends_with(".html") => {
                    let page = String::from_utf8_lossy(&bytes).to_string();
                    let shim = r#"<script>
window.__TAURI_INTERNALS__ = {
  invoke: async (cmd, args) => {
    const res = await fetch('/invoke', {
      method: 'POST',
      headers: {'content-type': 'application/json'},
      body: JSON.stringify({cmd, args: args || {}}),
    });
    const data = await res.json();
    if (data && data.ok === true && data.result !== undefined) {
      return data.result;
    }
    if (data && data.ok === false) {
      const code = (data.error && data.error.code) || 'TAURI-IPC';
      const message = (data.error && data.error.message) || 'invoke failed';
      throw new Error(code + ': ' + message);
    }
                return data;
  }
};
</script>"#;
                    let injected = page.replacen("<head>", &format!("<head>\n{shim}"), 1);
                    ("200 OK", "text/html", injected.into_bytes())
                }
                Some(bytes) => {
                    let content_type = if rel.ends_with(".js") {
                        "text/javascript"
                    } else if rel.ends_with(".css") {
                        "text/css"
                    } else if rel.ends_with(".svg") {
                        "image/svg+xml"
                    } else if rel.ends_with(".png") {
                        "image/png"
                    } else {
                        "application/octet-stream"
                    };
                    ("200 OK", content_type, bytes)
                }
                None => ("404 Not Found", "text/plain", b"not found".to_vec()),
            }
        } else {
            ("405 Method Not Allowed", "text/plain", b"method".to_vec())
        };

        let header = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            content.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(&content);
        let _ = stream.flush();
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
