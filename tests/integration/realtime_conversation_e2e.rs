use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ac_daemon::{default_paths, default_socket_path, DaemonService, UnixIpcClient, UnixIpcServer};
use serde_json::{json, Value};

fn run_git<const N: usize>(cwd: &PathBuf, args: [&str; N]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:{}\nstderr:{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_root(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/ace2e-{}-{}", std::process::id(), name))
}

fn create_fixture_project(root: &PathBuf) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(root.join(".gitignore"), ".agentcode/\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"ace2e-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn answer() -> u32 { 41 }\n").unwrap();
    fs::write(
        root.join("tests/fixture.rs"),
        "use ace2e_fixture::answer;\n#[test]\nfn answer_is_correct() {\n    assert_eq!(answer(), 42);\n}\n",
    )
    .unwrap();
    run_git(root, ["init"]);
    run_git(root, ["add", "."]);
    run_git(
        root,
        [
            "-c",
            "user.name=AgentCode E2E",
            "-c",
            "user.email=e2e@agentcode.test",
            "commit",
            "-m",
            "initial",
        ],
    );
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

fn poll_mission(
    server: &UnixIpcServer,
    listener: &std::os::unix::net::UnixListener,
    daemon: &mut DaemonService,
    mission_id: &str,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(900);
    loop {
        let resp = request_via_ipc(
            server,
            listener,
            daemon,
            json!({"id": "poll", "command": "GetMission", "mission_id": mission_id}),
        );
        let state = resp["state"].as_str().unwrap_or("");
        match state {
            "completed" => return resp,
            s if s == "failed" || s.starts_with("failed:") => {
                eprintln!("E2E: mission reached terminal state: {s}");
                return resp;
            }
            "cancelled" => {
                return resp;
            }
            _ => {
                assert!(
                    Instant::now() < deadline,
                    "mission did not reach terminal within timeout\nlast: {resp}"
                );
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

/// Real E2E: conversation → goal → mission → planner → provider → tool →
/// changeset → evidence → verification → final result → activity projection.
/// Uses the real daemon in-process with a local ollama model (≤4B).
///
/// This test is #[ignore] because it requires:
/// 1. The real daemon binary (built via `cargo build -p ac-daemon`)
/// 2. Ollama running with a suitable model
/// 3. Gemma3:1b (815MB, ≤4B) installed
///
/// Run with: OLLAMA_BASE_URL=http://127.0.0.1:11434 OLLAMA_MODEL=gemma3:1b \
///   AGENTCODE_ENABLE_OLLAMA=1 cargo test --test realtime_conversation_e2e
#[test]
#[ignore = "requires ollama running with gemma3:1b model"]
fn realtime_conversation_and_mission_activity_projection() {
    // ── Setup ─────────────────────────────────────────────────────────────────
    let runtime = temp_root("runtime");
    let project = temp_root("project");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_fixture_project(&project);

    let (db_path, lock) = default_paths(&runtime);
    let socket = default_socket_path(&runtime);

    let mut daemon = DaemonService::open(&db_path, &lock).unwrap();
    daemon.start().unwrap();

    let (server, listener) = UnixIpcServer::bind(&socket).expect("bind IPC socket");

    // ── Conversation: create ──────────────────────────────────────────────────
    let project_path = project.to_string_lossy().to_string();
    let create = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"c1","command":"ConversationCreate","project_path": project_path, "mode":"GOAL","title":"E2E Test"}),
    );
    assert_eq!(create["ok"], true);
    let cid = create["conversation_id"].as_str().unwrap().to_string();
    eprintln!("E2E: conversation={cid}");

    // ── Attachment: register a text file ──────────────────────────────────────
    let att_dir = project.join(".agentcode").join("attachments").join(&cid);
    fs::create_dir_all(&att_dir).unwrap();
    let att_content = b"fn answer() -> u32 { 41 }";
    let rel_path = format!(".agentcode/attachments/{cid}/src.rs");
    fs::write(project.join(&rel_path), att_content).unwrap();
    let hash = format!(
        "fnv1a64:{:016x}",
        att_content
            .iter()
            .fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(*b))
                .wrapping_mul(0x100000001b3))
    );
    let reg = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"a1","command":"AttachmentRegister","conversation_id": cid, "project_path": project_path, "filename":"src.rs","mime_type":"text/plain","size_bytes": att_content.len() as i64,"content_hash": hash,"rel_path": rel_path}),
    );
    assert_eq!(reg["ok"], true, "register: {reg}");
    let att_id = reg["attachment"]["id"].as_str().unwrap().to_string();
    eprintln!("E2E: attachment={att_id}");

    // ── Goal submission → creates mission ────────────────────────────────────
    let goal = "Fix the answer function to return 42 instead of 41";
    let submit = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"s1","command":"GoalSubmit","conversation_id": cid, "goal": goal, "attachment_ids": [att_id]}),
    );
    assert_eq!(submit["ok"], true, "submit: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    eprintln!("E2E: mission={mission_id}");

    // ── Wait for terminal state ───────────────────────────────────────────────
    let final_state = poll_mission(&server, &listener, &mut daemon, &mission_id);
    eprintln!("E2E: mission completed: state={}", final_state["state"]);
    // Real-model E2E must actually COMPLETE the mission: a real provider
    // (qwen2.5-coder:3b, ≤4B constraint) plans, edits, and verifies the fix.
    // A failure here is a regression, not a tolerated outcome.
    assert_eq!(
        final_state["state"].as_str().unwrap_or(""),
        "completed",
        "real-model mission must complete: {}",
        final_state
    );
    // The goal requires the fix to be verified: the answer function must now
    // be 42 in the mission worktree (the kernel only completes missions whose
    // verification passed).
    let worktree_root = project.join(".agentcode-worktrees");
    let mut fixed_src = None;
    if let Ok(read) = fs::read_dir(&worktree_root) {
        for entry in read.flatten() {
            let lib = entry.path().join("src").join("lib.rs");
            if let Ok(content) = fs::read_to_string(&lib) {
                if content.contains("42") {
                    fixed_src = Some(content);
                }
            }
        }
    }
    assert!(
        fixed_src.is_some(),
        "the worktree must contain the fixed answer function (42)"
    );

    // ── Verify conversation activity projection ───────────────────────────────
    let activity = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"a1","command":"ConversationActivity","conversation_id": cid}),
    );
    assert_eq!(activity["ok"], true, "activity: {activity}");
    let missions = activity["missions"].as_array().unwrap();
    assert!(!missions.is_empty(), "should have at least 1 mission");
    assert_eq!(missions[0]["mission_id"], mission_id);
    assert!(missions[0].get("details").is_some(), "details present");
    assert!(missions[0].get("tasks").is_some(), "tasks present");
    assert!(missions[0].get("events").is_some(), "events present");
    assert!(missions[0].get("evidence").is_some(), "evidence present");
    assert!(
        missions[0].get("verification").is_some(),
        "verification present"
    );
    eprintln!("E2E: activity projection verified");

    // ── Verify the conversation get also works ────────────────────────────────
    let get = request_via_ipc(
        &server,
        &listener,
        &mut daemon,
        json!({"id":"g1","command":"ConversationGet","conversation_id": cid}),
    );
    assert_eq!(get["ok"], true);
    let msgs = get["conversation"]["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["mission_ref"].as_str().unwrap(), mission_id);

    // ── Verify the attachment is linked to the message ─────────────────────────
    let atts = get["conversation"]["attachments"].as_array().unwrap();
    assert!(!atts.is_empty(), "should have attachments");
    assert!(atts[0].get("message_id").and_then(|v| v.as_str()).is_some());
    eprintln!("E2E: attachment linked to message");

    // ── Cleanup ───────────────────────────────────────────────────────────────
    server.cleanup();
    daemon.shutdown().unwrap();
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    eprintln!("E2E: PASSED");
}
