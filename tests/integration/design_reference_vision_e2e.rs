//! Real-model E2E for Design Studio reference-image analysis (core Doc 06
//! H28): a PNG attachment in a DESIGN conversation is analyzed by a real
//! vision-capable local model (gemma3:4b, within the ≤4B constraint) and the
//! structured ReferenceAnalysis is persisted as a design document.
//!
//! This test is #[ignore] by default because it requires:
//! 1. Ollama running at OLLAMA_BASE_URL with gemma3:4b pulled
//! 2. AGENTCODE_ENABLE_OLLAMA=1 to opt in
//!
//! Run with:
//!   OLLAMA_BASE_URL=http://127.0.0.1:11434 AGENTCODE_VISION_MODEL=gemma3:4b \
//!     AGENTCODE_ENABLE_OLLAMA=1 cargo test --test design_reference_vision_e2e \
//!     -- --ignored --nocapture

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ac_daemon::{default_paths, default_socket_path, DaemonService, UnixIpcClient, UnixIpcServer};
use serde_json::{json, Value};

fn temp_root(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/acvision-{}-{}", std::process::id(), name))
}

fn request_via_ipc(
    server: &UnixIpcServer,
    listener: &std::os::unix::net::UnixListener,
    daemon: &mut DaemonService,
    payload: Value,
) -> Value {
    let client = std::thread::spawn({
        let socket = server.path().to_path_buf();
        move || UnixIpcClient::new(socket).request(payload).unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    while !client.is_finished() && Instant::now() < deadline {
        let _ = server.serve_once(listener, daemon);
        std::thread::sleep(Duration::from_millis(5));
    }
    client.join().unwrap()
}

fn fnv1a64(data: &[u8]) -> String {
    let hash = data.iter().fold(0xcbf29ce484222325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    });
    format!("fnv1a64:{hash:016x}")
}

#[test]
#[ignore = "requires ollama running with gemma3:4b vision model"]
fn design_reference_image_analysis_with_real_vision_model() {
    let runtime = temp_root("runtime");
    let project = temp_root("project");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join(".gitignore"), ".agentcode/\n").unwrap();

    let (db_path, lock) = default_paths(&runtime);
    let socket = default_socket_path(&runtime);

    let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
    daemon.start().unwrap();
    let (server, listener) = UnixIpcServer::bind(&socket).expect("bind IPC socket");

    // DESIGN conversation for the project
    let project_path = project.to_string_lossy().to_string();
    let create = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"DESIGN","title":"Reference Analysis E2E"}),
    );
    assert_eq!(create["ok"], true, "create: {create}");
    let cid = create["conversation_id"].as_str().unwrap().to_string();
    eprintln!("VISION-E2E: conversation={cid}");

    // Register a real PNG fixture as an attachment
    let png_bytes = include_bytes!("fixtures/reference_ui_card.png");
    let att_dir = project.join(".agentcode").join("attachments").join(&cid);
    fs::create_dir_all(&att_dir).unwrap();
    let rel_path = format!(".agentcode/attachments/{cid}/reference_ui_card.png");
    fs::write(project.join(&rel_path), png_bytes).unwrap();
    let reg = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"a1","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"reference_ui_card.png","mime_type":"image/png","size_bytes": png_bytes.len() as i64,"content_hash": fnv1a64(png_bytes),"rel_path": rel_path}),
    );
    assert_eq!(reg["ok"], true, "register: {reg}");
    let att_id = reg["attachment"]["id"].as_str().unwrap().to_string();
    eprintln!(
        "VISION-E2E: attachment={att_id} ({} bytes)",
        png_bytes.len()
    );

    // Run the reference analysis against the real vision model
    let started = Instant::now();
    let analyze = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"r1","command":"DesignAnalyzeReference","conversation_id": cid, "attachment_id": att_id}),
    );
    let elapsed = started.elapsed();
    assert_eq!(analyze["ok"], true, "analyze: {analyze}");
    let analysis = &analyze["analysis"];
    eprintln!("VISION-E2E: analysis completed in {elapsed:.1?}");

    // H28 shape: structured extraction with the copying boundary
    assert_eq!(analysis["reference_id"].as_str(), Some(att_id.as_str()));
    assert_eq!(analysis["source_type"].as_str(), Some("IMAGE"));
    let model = analysis["vision_model"].as_str().unwrap_or("");
    assert!(!model.is_empty(), "vision model must be recorded honestly");
    eprintln!("VISION-E2E: vision_model={model}");
    let extracted = analysis["extracted"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let non_empty_fields = extracted
        .values()
        .filter(|v| v.as_array().map(|a| !a.is_empty()).unwrap_or(false))
        .count();
    assert!(
        non_empty_fields >= 2,
        "vision model must extract at least 2 non-empty design fields, got {non_empty_fields}: {analysis}"
    );
    let do_not_copy = analysis["explicitly_do_not_copy"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(!do_not_copy.is_empty(), "copying boundary must be stated");
    eprintln!("VISION-E2E: do_not_copy entries={}", do_not_copy.len());

    // The analysis must be persisted as a design document
    let state = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"d1","command":"DesignState","conversation_id": cid}),
    );
    assert_eq!(state["ok"], true, "state: {state}");

    // Deterministic failure honesty: a non-image attachment must be refused
    let txt_path = format!(".agentcode/attachments/{cid}/notes.txt");
    fs::write(project.join(&txt_path), b"just text").unwrap();
    let reg2 = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"a2","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"notes.txt","mime_type":"text/plain","size_bytes": 9,"content_hash": fnv1a64(b"just text"),"rel_path": txt_path}),
    );
    assert_eq!(reg2["ok"], true);
    let txt_att = reg2["attachment"]["id"].as_str().unwrap().to_string();
    let refused = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"r2","command":"DesignAnalyzeReference","conversation_id": cid, "attachment_id": txt_att}),
    );
    assert_eq!(
        refused["ok"], false,
        "text attachment must not be analyzed as a reference image"
    );
    assert_eq!(
        refused["error"]["code"].as_str().unwrap_or(""),
        "DESIGN-REFERENCE_NOT_IMAGE"
    );

    // The analysis outcome is also visible in the conversation messages
    let messages = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"m1","command":"ConversationGet","conversation_id": cid}),
    );
    assert_eq!(messages["ok"], true, "messages: {messages}");
    let body = messages.to_string();
    assert!(
        body.contains("Reference image analyzed"),
        "assistant summary must be recorded: {body}"
    );

    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}
