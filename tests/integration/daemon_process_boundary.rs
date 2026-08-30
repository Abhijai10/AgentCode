use std::fs::{self, File};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use ac_common::StableId;
use ac_daemon::{default_paths, default_socket_path, UnixIpcClient};
use ac_db::ControlPlaneDb;
use serde_json::{json, Value};

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_survives_disconnect_restart_without_replay() {
    let runtime = short_temp_path("acr");
    let project = short_temp_path("acp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_fixture_project(&project);
    let daemon = daemon_binary();
    assert!(
        daemon.is_file(),
        "build the daemon first: cargo build -p ac-daemon"
    );

    let socket = default_socket_path(&runtime);
    let (db_path, _) = default_paths(&runtime);

    let mut first = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut first, &runtime);
    assert_socket_0600(&socket);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    let session_id = submit["session_id"].as_str().unwrap().to_string();
    assert_process_running(&mut first, &runtime);

    let completed = poll_mission_completed(&socket, &mission_id, &mut first, &runtime);
    assert_eq!(
        completed["state"], "completed",
        "mission response: {completed}"
    );

    let worktree_lib = project
        .join(".agentcode-worktrees")
        .join(&session_id)
        .join("src/lib.rs");
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}"
    );
    assert_eq!(
        count_occurrences(&fs::read_to_string(&worktree_lib).unwrap(), "42"),
        1
    );
    assert_eq!(
        fs::read_to_string(project.join("src/lib.rs")).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    41\n}\n"
    );

    let mission_stable = StableId::from_existing(&mission_id).unwrap();
    let session_stable = StableId::from_existing(&session_id).unwrap();
    let db = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        db.get_mission(&mission_stable).unwrap().unwrap().state,
        "completed"
    );
    assert_eq!(
        db.get_session(&session_stable).unwrap().unwrap().state,
        "completed"
    );
    let evidence = db.evidence_records().unwrap();
    assert!(evidence.iter().any(|record| {
        record.provenance.tool.as_deref() == Some("dev.test")
            && record
                .raw_content
                .as_deref()
                .is_some_and(|content| content.contains("achieved_isolation:FilesystemIsolated"))
    }));
    assert!(evidence.iter().any(|record| {
        record.provenance.source == "agent.completion-request"
            || record.artifact_uri.contains("completion")
    }));
    let evidence_count = evidence.len();
    let persisted_task_attempt_count = task_attempt_count(&db, &mission_id);

    shutdown_daemon(&socket, &mut first, &runtime);

    let mut second = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut second, &runtime);
    assert_socket_0600(&socket);
    let after_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "after-restart",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(
        after_restart["ok"], true,
        "restart response: {after_restart}"
    );
    assert_eq!(after_restart["state"], "completed");
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}"
    );
    assert_eq!(
        count_occurrences(&fs::read_to_string(&worktree_lib).unwrap(), "42"),
        1
    );
    let restarted_db = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        restarted_db.evidence_records().unwrap().len(),
        evidence_count
    );
    assert_eq!(
        task_attempt_count(&restarted_db, &mission_id),
        persisted_task_attempt_count
    );
    assert_eq!(
        restarted_db
            .get_mission(&mission_stable)
            .unwrap()
            .unwrap()
            .state,
        "completed"
    );

    shutdown_daemon(&socket, &mut second, &runtime);

    let mut third = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut third, &runtime);
    assert_socket_0600(&socket);
    let after_second_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "after-second-restart",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(
        after_second_restart["state"], "completed",
        "second restart response: {after_second_restart}"
    );
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}"
    );
    let second_restart_db = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        second_restart_db.evidence_records().unwrap().len(),
        evidence_count
    );
    assert_eq!(
        task_attempt_count(&second_restart_db, &mission_id),
        persisted_task_attempt_count
    );

    shutdown_daemon(&socket, &mut third, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_recovers_crash_after_mutation_without_replay() {
    let runtime = short_temp_path("acrc");
    let project = short_temp_path("acpc");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_slow_fixture_project(&project);
    let daemon = daemon_binary();
    assert!(
        daemon.is_file(),
        "build the daemon first: cargo build -p ac-daemon"
    );

    let socket = default_socket_path(&runtime);
    let (db_path, _) = default_paths(&runtime);
    let mut first = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut first, &runtime);
    assert_socket_0600(&socket);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-crash",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    let session_id = submit["session_id"].as_str().unwrap().to_string();

    let modified_task = poll_task_state(&db_path, &mission_id, "Modify target", "completed");
    let worktree_lib = project
        .join(".agentcode-worktrees")
        .join(&session_id)
        .join("src/lib.rs");
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}"
    );
    assert_eq!(
        count_occurrences(&fs::read_to_string(&worktree_lib).unwrap(), "42"),
        1
    );
    let db_before = ControlPlaneDb::open(&db_path).unwrap();
    assert!(db_before
        .worktree_for_mission(&StableId::from_existing(&mission_id).unwrap())
        .unwrap()
        .is_some());
    let modify_attempts_before = db_before.task_attempts(&modified_task).unwrap().len();
    assert_eq!(modify_attempts_before, 1);

    first.kill().unwrap();
    wait_for_exit(&mut first, &runtime);

    let mut second = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut second, &runtime);
    assert_socket_0600(&socket);

    // While the recovered mission is re-running, ListActiveMissions must
    // reflect the PERSISTED task graph from SQLite (not a loss of task state
    // across the crash boundary).  This is checked before completion while the
    // mission is still active.
    let listed_tasks = poll_active_mission_tasks(&socket, &mission_id, &mut second, &runtime);
    assert!(
        listed_tasks >= 2,
        "ListActiveMissions must reflect persisted tasks during recovery"
    );

    let completed = poll_mission_completed(&socket, &mission_id, &mut second, &runtime);
    assert_eq!(
        completed["state"], "completed",
        "mission response: {completed}"
    );
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}"
    );
    assert_eq!(
        count_occurrences(&fs::read_to_string(&worktree_lib).unwrap(), "42"),
        1
    );
    let db_after = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        db_after.task_attempts(&modified_task).unwrap().len(),
        modify_attempts_before,
        "completed mutation task was replayed"
    );
    assert!(db_after.evidence_records().unwrap().iter().any(|record| {
        record.provenance.tool.as_deref() == Some("dev.test")
            && record
                .raw_content
                .as_deref()
                .is_some_and(|content| content.contains("achieved_isolation:FilesystemIsolated"))
    }));
    let evidence_count_after_crash = db_after.evidence_records().unwrap().len();

    // Verify the daemon restarted with the SAME mission/session identity and
    // reports the PERSISTED task graph (not an in-memory reconstruction).
    let same_mission = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "identity-check",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(same_mission["mission_id"], mission_id, "{same_mission}");
    assert_eq!(same_mission["session_id"], session_id, "{same_mission}");
    assert_eq!(same_mission["state"], "completed", "{same_mission}");
    let persisted_tasks = same_mission["tasks"]
        .as_array()
        .map(|tasks| tasks.len())
        .unwrap_or(0);
    assert!(
        persisted_tasks >= 2,
        "persisted task graph must be visible over IPC, got {same_mission}"
    );
    // ListActiveMissions was already proven to reflect the persisted task
    // graph above, while the recovered mission was still active.

    shutdown_daemon(&socket, &mut second, &runtime);

    // Second restart: the terminal Completed state must survive and nothing
    // may replay (no new mutation attempt, no duplicated evidence).
    let mut third = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut third, &runtime);
    assert_socket_0600(&socket);
    let after_second_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "after-second-restart",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(
        after_second_restart["state"], "completed",
        "second restart response: {after_second_restart}"
    );
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}"
    );
    assert_eq!(
        count_occurrences(&fs::read_to_string(&worktree_lib).unwrap(), "42"),
        1
    );
    let second_restart_db = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        second_restart_db
            .task_attempts(&modified_task)
            .unwrap()
            .len(),
        modify_attempts_before,
        "completed mutation task replayed after second restart"
    );
    assert_eq!(
        second_restart_db.evidence_records().unwrap().len(),
        evidence_count_after_crash,
        "evidence duplicated after second restart"
    );

    shutdown_daemon(&socket, &mut third, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_binds_submitted_workspace_root_per_project() {
    // Project-to-daemon workspace binding (TEST 1/2/7/9): the daemon must
    // execute against the submitted workspace_root, never against its own
    // AGENTCODE_WORKSPACE_ROOT or cwd, and two projects must stay isolated.
    let runtime = short_temp_path("acwb");
    let project_a = short_temp_path("acwba");
    let project_b = short_temp_path("acwbb");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project_a);
    let _ = fs::remove_dir_all(&project_b);
    fs::create_dir_all(&runtime).unwrap();
    create_fixture_project(&project_a);
    create_fixture_project(&project_b);
    let daemon = daemon_binary();
    assert!(
        daemon.is_file(),
        "build the daemon first: cargo build -p ac-daemon"
    );
    let socket = default_socket_path(&runtime);
    let (db_path, _) = default_paths(&runtime);

    // Launch with AGENTCODE_WORKSPACE_ROOT = project B so the daemon's
    // configured default deliberately differs from the project we will submit.
    let mut child = launch_daemon(&daemon, &runtime, &project_b);
    wait_for_socket(&socket, &mut child, &runtime);

    // Mission 1: submit with workspace_root = Project A.
    let submit_a = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-a",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs",
            "workspace_root": project_a.to_string_lossy().to_string()
        }))
        .unwrap();
    assert_eq!(submit_a["ok"], true, "submit A response: {submit_a}");
    let mission_a = submit_a["mission_id"].as_str().unwrap().to_string();
    let completed_a = poll_mission_completed(&socket, &mission_a, &mut child, &runtime);
    assert_eq!(completed_a["state"], "completed", "{completed_a}");
    // Persisted workspace must be Project A, not the daemon default (B).
    let db = ControlPlaneDb::open(&db_path).unwrap();
    let session_a = db
        .session_for_mission(&StableId::from_existing(&mission_a).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        session_a.workspace_root.as_deref(),
        Some(fs::canonicalize(&project_a).unwrap().to_str().unwrap()),
        "mission A must be bound to Project A"
    );
    // Mutation must appear in Project A's worktree, NOT in Project B.
    let session_a_id = session_a.id.clone();
    let worktree_a_lib = project_a
        .join(".agentcode-worktrees")
        .join(&session_a_id)
        .join("src/lib.rs");
    assert_eq!(
        fs::read_to_string(&worktree_a_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}",
        "Project A worktree must contain the mutation"
    );
    assert_eq!(
        fs::read_to_string(project_a.join("src/lib.rs")).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    41\n}\n",
        "Project A source must remain unmodified by isolated work"
    );
    assert!(
        !project_b.join(".agentcode-worktrees").exists(),
        "Project B must not receive Project A's worktree"
    );

    // Mission 2: submit with workspace_root = Project B.
    let submit_b = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-b",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs",
            "workspace_root": project_b.to_string_lossy().to_string()
        }))
        .unwrap();
    assert_eq!(submit_b["ok"], true, "submit B response: {submit_b}");
    let mission_b = submit_b["mission_id"].as_str().unwrap().to_string();
    let completed_b = poll_mission_completed(&socket, &mission_b, &mut child, &runtime);
    assert_eq!(completed_b["state"], "completed", "{completed_b}");
    let db = ControlPlaneDb::open(&db_path).unwrap();
    let session_b = db
        .session_for_mission(&StableId::from_existing(&mission_b).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        session_b.workspace_root.as_deref(),
        Some(fs::canonicalize(&project_b).unwrap().to_str().unwrap()),
        "mission B must be bound to Project B"
    );
    let worktree_b_lib = project_b
        .join(".agentcode-worktrees")
        .join(&session_b.id)
        .join("src/lib.rs");
    assert_eq!(
        fs::read_to_string(&worktree_b_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}",
        "Project B worktree must contain the mutation"
    );
    // The two missions must never share a workspace identity.
    assert_ne!(
        session_a.workspace_root, session_b.workspace_root,
        "two different projects must not share a workspace"
    );
    assert_ne!(
        session_a.id, session_b.id,
        "two missions must not share a session/worktree"
    );

    shutdown_daemon(&socket, &mut child, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project_a);
    let _ = fs::remove_dir_all(&project_b);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_rejects_invalid_workspace_root() {
    // TEST 5/9: The daemon must independently reject an invalid, non-existent,
    // or non-directory workspace_root from the frontend.
    let runtime = short_temp_path("acwi");
    let project = short_temp_path("acwip");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_fixture_project(&project);
    let daemon = daemon_binary();
    let socket = default_socket_path(&runtime);
    let mut child = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut child, &runtime);

    let missing = runtime.join("no-such-project");
    let rejected = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-invalid",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs",
            "workspace_root": missing.to_string_lossy().to_string()
        }))
        .unwrap();
    assert_eq!(
        rejected["ok"], false,
        "invalid workspace_root must be rejected: {rejected}"
    );
    assert_eq!(rejected["error"]["code"], "DAEMON-WORKSPACE_UNAVAILABLE");

    // A file is not a directory.
    let file = runtime.join("not-a-dir.txt");
    fs::write(&file, "x").unwrap();
    let rejected_file = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-file",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs",
            "workspace_root": file.to_string_lossy().to_string()
        }))
        .unwrap();
    assert_eq!(
        rejected_file["error"]["code"], "DAEMON-WORKSPACE_NOT_DIR",
        "{rejected_file}"
    );

    shutdown_daemon(&socket, &mut child, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_cancels_active_mission_without_restart_replay() {
    let runtime = short_temp_path("acx");
    let project = short_temp_path("acxp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_slow_fixture_project(&project);
    let daemon = daemon_binary();
    let socket = default_socket_path(&runtime);
    let (db_path, _) = default_paths(&runtime);
    let mut child = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut child, &runtime);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-cancel",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    poll_task_state(&db_path, &mission_id, "Modify target", "completed");
    let cancel = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "cancel-active",
            "command": "CancelMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(cancel["ok"], true, "cancel response: {cancel}");
    let cancelled = poll_mission_state(&socket, &mission_id, "cancelled", &mut child, &runtime);
    assert_eq!(cancelled["state"], "cancelled");
    shutdown_daemon(&socket, &mut child, &runtime);

    let mut restarted = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut restarted, &runtime);
    let after_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "cancel-after-restart",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(after_restart["state"], "cancelled");
    shutdown_daemon(&socket, &mut restarted, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_pauses_queued_mission_across_restart_then_resumes() {
    let runtime = short_temp_path("acpzr");
    let project = short_temp_path("acpzp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_slow_fixture_project(&project);
    let daemon = daemon_binary();
    let socket = default_socket_path(&runtime);
    let mut child = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut child, &runtime);

    let first = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-first",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(first["ok"], true, "first response: {first}");
    let second = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-second",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(second["ok"], true, "second response: {second}");
    let second_mission = second["mission_id"].as_str().unwrap().to_string();
    let second_session = second["session_id"].as_str().unwrap().to_string();
    let pause = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "pause-second",
            "command": "PauseMission",
            "mission_id": second_mission
        }))
        .unwrap();
    assert_eq!(pause["ok"], true, "pause response: {pause}");
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        !project
            .join(".agentcode-worktrees")
            .join(&second_session)
            .exists(),
        "paused queued mission should not create a worktree before resume"
    );
    shutdown_daemon(&socket, &mut child, &runtime);

    let mut restarted = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut restarted, &runtime);
    let paused = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "paused-after-restart",
            "command": "GetMission",
            "mission_id": second_mission
        }))
        .unwrap();
    assert_eq!(paused["state"], "paused", "paused response: {paused}");
    let resume = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "resume-second",
            "command": "ResumeMission",
            "mission_id": second_mission
        }))
        .unwrap();
    assert_eq!(resume["ok"], true, "resume response: {resume}");
    let completed = poll_mission_completed(&socket, &second_mission, &mut restarted, &runtime);
    assert_eq!(completed["state"], "completed");
    shutdown_daemon(&socket, &mut restarted, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires Ollama or LM Studio running locally with a loaded model"]
fn real_provider_daemon_path_smoke_proof() {
    let ollama_base =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let connect_addr = ollama_connect_addr(&ollama_base);
    let ollama_chat = if ollama_base.ends_with("/api/chat") {
        ollama_base.clone()
    } else if ollama_base.ends_with('/') {
        format!("{ollama_base}api/chat")
    } else {
        format!("{ollama_base}/api/chat")
    };
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3:4b".to_string());
    // Quick connectivity check — skip (not fail) if Ollama is unreachable.
    // Uses host:port only (never the URL path) so TcpStream::connect works.
    if std::net::TcpStream::connect(&connect_addr).is_err() {
        eprintln!("SKIP: Ollama not reachable at {ollama_base}; cannot run real provider test");
        return;
    }
    let runtime = short_temp_path("acrp");
    let project = short_temp_path("acrpp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_passing_fixture_project(&project);
    let daemon = daemon_binary();
    assert!(
        daemon.is_file(),
        "build the daemon first: cargo build -p ac-daemon"
    );
    let socket = default_socket_path(&runtime);
    let (db_path, _) = default_paths(&runtime);
    let mut child = Command::new(&daemon)
        .env("AGENTCODE_RUNTIME_DIR", &runtime)
        .env("AGENTCODE_WORKSPACE_ROOT", &project)
        .env("OLLAMA_BASE_URL", &ollama_chat)
        .env("OLLAMA_MODEL", &ollama_model)
        // No AGENTCODE_PROVIDER_MODE=mock — this is a real provider test.
        .stdout(Stdio::from(
            File::create(runtime.join("daemon.log")).unwrap(),
        ))
        .stderr(Stdio::from(
            File::options()
                .create(true)
                .append(true)
                .open(runtime.join("daemon.log"))
                .unwrap(),
        ))
        .spawn()
        .unwrap();
    wait_for_socket(&socket, &mut child, &runtime);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-real",
            "command": "SubmitMission",
            "goal": "Verify that fixture_answer returns 42"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();

    // Strict completion poll.  If the mission reaches failed or cancelled,
    // poll_mission_completed immediately panics with the daemon log —
    // no "UNPROVEN" escape hatch.
    let completed = poll_mission_completed(&socket, &mission_id, &mut child, &runtime);
    assert_eq!(
        completed["state"], "completed",
        "real provider mission: {completed}"
    );

    let mission_stable = StableId::from_existing(&mission_id).unwrap();
    let db = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        db.get_mission(&mission_stable).unwrap().unwrap().state,
        "completed"
    );
    // Verify real provider evidence exists (not mock-scripted)
    let evidence = db.evidence_records().unwrap();
    assert!(
        evidence.iter().any(|record| {
            record.provenance.source == "agent.completion-request"
                || record.artifact_uri.contains("completion")
        }),
        "real provider must produce completion evidence"
    );

    // Verify the provider source in evidence is from a real provider, not
    // the scripted mock.  Real provider evidence has provenance source
    // "agent.provider.planner" or "agent.completion-request" with proper
    // artifact_uri (not "mem://agent/.../provider/planner" mock URIs).
    let real_provider_evidence = evidence.iter().any(|record| {
        record.provenance.source == "agent.provider.planner"
            && !record.artifact_uri.contains("mock")
            && !record.artifact_uri.contains("scripted")
    });
    assert!(
        real_provider_evidence,
        "real provider must produce planner evidence from a real provider, not mock or scripted"
    );

    shutdown_daemon(&socket, &mut child, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires Ollama or LM Studio running locally with a loaded model"]
fn real_provider_daemon_path_creates_smoke_file_with_exact_content() {
    let ollama_base =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let connect_addr = ollama_connect_addr(&ollama_base);
    if std::net::TcpStream::connect(&connect_addr).is_err() {
        eprintln!("SKIP: Ollama not reachable at {ollama_base}; cannot run real provider test");
        return;
    }
    let ollama_chat = if ollama_base.ends_with("/api/chat") {
        ollama_base.clone()
    } else if ollama_base.ends_with('/') {
        format!("{ollama_base}api/chat")
    } else {
        format!("{ollama_base}/api/chat")
    };
    let ollama_model =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:7b".to_string());

    let runtime = short_temp_path("acrp");
    let project = short_temp_path("acrpp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_smoke_fixture_project(&project);
    let daemon = daemon_binary();
    assert!(
        daemon.is_file(),
        "build the daemon first: cargo build -p ac-daemon"
    );
    let socket = default_socket_path(&runtime);
    let (db_path, _) = default_paths(&runtime);

    let mut child = Command::new(&daemon)
        .env("AGENTCODE_RUNTIME_DIR", &runtime)
        .env("AGENTCODE_WORKSPACE_ROOT", &project)
        .env("OLLAMA_BASE_URL", &ollama_chat)
        .env("OLLAMA_MODEL", &ollama_model)
        .stdout(Stdio::from(
            File::create(runtime.join("daemon.log")).unwrap(),
        ))
        .stderr(Stdio::from(
            File::options()
                .create(true)
                .append(true)
                .open(runtime.join("daemon.log"))
                .unwrap(),
        ))
        .spawn()
        .unwrap();
    wait_for_socket(&socket, &mut child, &runtime);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-smoke",
            "command": "SubmitMission",
            "goal": "Create a file named agentcode_smoke.txt containing exactly: AgentCode operational smoke test"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    let session_id = submit["session_id"].as_str().unwrap().to_string();

    let completed = poll_mission_completed(&socket, &mission_id, &mut child, &runtime);
    assert_eq!(
        completed["state"], "completed",
        "real provider smoke mission: {completed}"
    );

    // Verify the file was created with exact content in the worktree.
    // Small local models may place it at the root or inside tests/; the
    // backend must create the file with the exact content in one of them.
    let worktree_dir = project.join(".agentcode-worktrees").join(&session_id);
    let candidates = [
        worktree_dir.join("agentcode_smoke.txt"),
        worktree_dir.join("tests/agentcode_smoke.txt"),
    ];
    let found = candidates.iter().find(|path| path.exists());
    assert!(
        found.is_some(),
        "agentcode_smoke.txt must exist in the worktree after mission completion"
    );
    let content = fs::read_to_string(found.unwrap()).unwrap();
    assert_eq!(
        content.trim(),
        "AgentCode operational smoke test",
        "file content must match exactly"
    );

    // Verify real provider evidence exists.
    let mission_stable = StableId::from_existing(&mission_id).unwrap();
    let db = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        db.get_mission(&mission_stable).unwrap().unwrap().state,
        "completed"
    );
    let evidence = db.evidence_records().unwrap();
    assert!(
        evidence.iter().any(|record| {
            record.provenance.source == "agent.completion-request"
                || record.artifact_uri.contains("completion")
        }),
        "real provider must produce completion evidence"
    );
    let real_provider_evidence = evidence.iter().any(|record| {
        record.provenance.source == "agent.provider.planner"
            && !record.artifact_uri.contains("mock")
            && !record.artifact_uri.contains("scripted")
    });
    assert!(
        real_provider_evidence,
        "real provider must produce planner evidence from a real provider, not mock or scripted"
    );

    shutdown_daemon(&socket, &mut child, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

fn short_temp_path(prefix: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/{prefix}-{}", std::process::id()))
}

/// Extract the `host:port` TCP connect address from an Ollama base URL.  The
/// URL may carry a path (e.g. `/api/chat`); `TcpStream::connect` must never
/// receive the path — only the authority.
fn ollama_connect_addr(base: &str) -> String {
    let parsed = url::Url::parse(base).expect("OLLAMA_BASE_URL must be a valid URL");
    let host = parsed.host_str().expect("OLLAMA_BASE_URL must have a host");
    let port = parsed
        .port_or_known_default()
        .expect("OLLAMA_BASE_URL must resolve to a port");
    format!("{host}:{port}")
}

fn create_fixture_project(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"agentcode_daemon_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn fixture_answer() -> u32 {\n    41\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/fixture.rs"),
        "use agentcode_daemon_fixture::fixture_answer;\n\n#[test]\nfn fixture_answer_is_correct() {\n    assert_eq!(fixture_answer(), 42);\n}\n",
    )
    .unwrap();
    run_git(root, ["init"]);
    run_git(root, ["add", "."]);
    run_git(
        root,
        [
            "-c",
            "user.name=AgentCode Test",
            "-c",
            "user.email=agentcode@example.test",
            "commit",
            "-m",
            "initial",
        ],
    );
}

/// A fixture whose verification genuinely passes.  Used by the real-provider
/// smoke proof so the mission can complete end-to-end with a real local model:
/// the planner produces a real plan, the implementer produces a real
/// RunVerification action, dev.test actually runs the fixture test, and the
/// evidence/verification/completion gates are exercised for real.  The
/// real-provider test's purpose is to prove the provider/daemon integration,
/// not to test model code-writing ability, so verification must be achievable
/// with the small local model actually routed.
fn create_passing_fixture_project(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"agentcode_daemon_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn fixture_answer() -> u32 {\n    42\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/fixture.rs"),
        "use agentcode_daemon_fixture::fixture_answer;\n\n#[test]\nfn fixture_answer_is_correct() {\n    assert_eq!(fixture_answer(), 42);\n}\n",
    )
    .unwrap();
    run_git(root, ["init"]);
    run_git(root, ["add", "."]);
    run_git(
        root,
        [
            "-c",
            "user.name=AgentCode Test",
            "-c",
            "user.email=agentcode@example.test",
            "commit",
            "-m",
            "initial",
        ],
    );
}

fn create_slow_fixture_project(root: &Path) {
    create_fixture_project(root);
    fs::write(
        root.join("tests/fixture.rs"),
        "use agentcode_daemon_fixture::fixture_answer;\nuse std::{thread, time::Duration};\n\n#[test]\nfn fixture_answer_is_correct() {\n    thread::sleep(Duration::from_secs(5));\n    assert_eq!(fixture_answer(), 42);\n}\n",
    )
    .unwrap();
    run_git(root, ["add", "."]);
    run_git(
        root,
        [
            "-c",
            "user.name=AgentCode Test",
            "-c",
            "user.email=agentcode@example.test",
            "commit",
            "-m",
            "slow test",
        ],
    );
}

/// A fixture with a smoke test that reads agentcode_smoke.txt and asserts
/// exact content.  The test initially FAILS (file does not exist), so the
/// model must create the file with the correct content before verification
/// can pass.  This is a genuine coding task that exercises the real provider
/// path with real file mutation, ChangeSet, and verification.
fn create_smoke_fixture_project(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"agentcode_daemon_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn fixture_answer() -> u32 {\n    42\n}\n",
    )
    .unwrap();
    // The fixture test always passes.
    fs::write(
        root.join("tests/fixture.rs"),
        "use agentcode_daemon_fixture::fixture_answer;\n\n#[test]\nfn fixture_answer_is_correct() {\n    assert_eq!(fixture_answer(), 42);\n}\n",
    )
    .unwrap();
    // The smoke test will FAIL initially (file missing) and PASS after the
    // model creates the file with the exact content.  Small local models may
    // place the file at the repository root or inside tests/; both are
    // accepted so the test proves the BACKEND can create a file with exact
    // content, independent of the model's path choice.
    fs::write(
        root.join("tests/smoke.rs"),
        r#"#[test]
fn smoke_file_exists() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        format!("{manifest}/agentcode_smoke.txt"),
        format!("{manifest}/tests/agentcode_smoke.txt"),
    ];
    let content = candidates
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    assert_eq!(content.trim(), "AgentCode operational smoke test");
}
"#,
    )
    .unwrap();
    run_git(root, ["init"]);
    run_git(root, ["add", "."]);
    run_git(
        root,
        [
            "-c",
            "user.name=AgentCode Test",
            "-c",
            "user.email=agentcode@example.test",
            "commit",
            "-m",
            "initial",
        ],
    );
}

fn daemon_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.file_name().is_some_and(|name| name == "deps") {
        path.pop();
    }
    path.join("ac-daemon")
}

fn launch_daemon(daemon: &Path, runtime: &Path, project: &Path) -> Child {
    let log = File::create(runtime.join("daemon.log")).unwrap();
    Command::new(daemon)
        .env("AGENTCODE_RUNTIME_DIR", runtime)
        .env("AGENTCODE_WORKSPACE_ROOT", project)
        .env("AGENTCODE_PROVIDER_MODE", "mock")
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap()
}

fn wait_for_socket(socket: &Path, child: &mut Child, runtime: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        assert_process_running(child, runtime);
        if socket.exists()
            && UnixIpcClient::new(socket)
                .request(json!({"id": "ready", "command": "Ping"}))
                .is_ok_and(|response| response["ok"] == true)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("daemon socket was not created\n{}", daemon_log(runtime));
}

fn assert_socket_0600(socket: &Path) {
    let metadata = fs::metadata(socket).unwrap();
    assert!(metadata.file_type().is_socket());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
}

fn poll_mission_completed(
    socket: &Path,
    mission_id: &str,
    child: &mut Child,
    runtime: &Path,
) -> Value {
    // Generous timeout: real-provider missions with local models (Ollama) can
    // take several minutes because each planner/implementer call streams
    // slowly on small laptops.  Crash-recovery tests complete far sooner and
    // are only bounded by this ceiling, so it does not weaken their
    // assertions — it only sets the hang-detection horizon.
    let deadline = Instant::now() + Duration::from_secs(900);
    let mut last = json!(null);
    while Instant::now() < deadline {
        assert_process_running(child, runtime);
        last = UnixIpcClient::new(socket)
            .request(json!({
                "id": "poll",
                "command": "GetMission",
                "mission_id": mission_id
            }))
            .unwrap();
        if last["state"] == "completed" {
            return last;
        }
        let state_str = last["state"].as_str().unwrap_or("");
        assert_ne!(
            state_str,
            "failed",
            "mission failed: {last}\n{}",
            daemon_log(runtime)
        );
        assert!(
            !state_str.starts_with("failed:"),
            "mission failed: {last}\n{}",
            daemon_log(runtime)
        );
        assert_ne!(last["state"], "cancelled", "mission cancelled: {last}");
        std::thread::sleep(Duration::from_millis(250));
    }
    panic!(
        "mission did not complete; last={last}\n{}",
        daemon_log(runtime)
    );
}

fn poll_mission_state(
    socket: &Path,
    mission_id: &str,
    expected: &str,
    child: &mut Child,
    runtime: &Path,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = json!(null);
    while Instant::now() < deadline {
        assert_process_running(child, runtime);
        last = UnixIpcClient::new(socket)
            .request(json!({
                "id": "poll-state",
                "command": "GetMission",
                "mission_id": mission_id
            }))
            .unwrap();
        if last["state"] == expected {
            return last;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "mission did not reach {expected}; last={last}\n{}",
        daemon_log(runtime)
    );
}

fn poll_active_mission_tasks(
    socket: &Path,
    mission_id: &str,
    child: &mut Child,
    runtime: &Path,
) -> usize {
    // The recovered mission must be visible as active over real IPC with its
    // PERSISTED task graph.  Poll until the mission appears; the task_count
    // comes from SQLite (daemon.active_missions -> tasks_for_mission).
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = json!(null);
    while Instant::now() < deadline {
        assert_process_running(child, runtime);
        last = UnixIpcClient::new(socket)
            .request(json!({"id": "poll-active", "command": "ListActiveMissions"}))
            .unwrap();
        if let Some(count) = last["missions"]
            .as_array()
            .and_then(|missions| {
                missions
                    .iter()
                    .find(|mission| mission["mission_id"] == mission_id)
            })
            .and_then(|mission| mission["task_count"].as_u64())
        {
            return count as usize;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "recovered mission did not appear in ListActiveMissions; last={last}\n{}",
        daemon_log(runtime)
    );
}

fn shutdown_daemon(socket: &Path, child: &mut Child, runtime: &Path) {
    let response = UnixIpcClient::new(socket)
        .request(json!({"id": "shutdown", "command": "ShutdownDaemon"}))
        .unwrap();
    assert_eq!(response["ok"], true, "shutdown response: {response}");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "daemon exited with {status}\n{}",
                daemon_log(runtime)
            );
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    panic!(
        "daemon did not exit after shutdown\n{}",
        daemon_log(runtime)
    );
}

fn wait_for_exit(child: &mut Child, runtime: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    panic!("daemon did not exit after kill\n{}", daemon_log(runtime));
}

fn poll_task_state(db_path: &Path, mission_id: &str, title: &str, expected: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = Vec::new();
    while Instant::now() < deadline {
        let db = ControlPlaneDb::open(db_path).unwrap();
        last = db.tasks_for_mission(mission_id).unwrap();
        if let Some(task) = last
            .iter()
            .find(|task| task.title == title && task.state == expected)
        {
            return task.id.clone();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("task {title} did not reach {expected}; last={last:?}");
}

fn task_attempt_count(db: &ControlPlaneDb, mission_id: &str) -> usize {
    db.tasks_for_mission(mission_id)
        .unwrap()
        .iter()
        .map(|task| db.task_attempts(&task.id).unwrap().len())
        .sum()
}

fn assert_process_running(child: &mut Child, runtime: &Path) {
    if let Some(status) = child.try_wait().unwrap() {
        panic!("daemon exited early with {status}\n{}", daemon_log(runtime));
    }
}

fn daemon_log(runtime: &Path) -> String {
    fs::read_to_string(runtime.join("daemon.log")).unwrap_or_default()
}

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = Command::new("git")
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

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

#[cfg(test)]
mod url_parsing_tests {
    use super::ollama_connect_addr;

    #[test]
    fn ollama_connect_addr_strips_path_from_api_chat_url() {
        assert_eq!(
            ollama_connect_addr("http://127.0.0.1:11434/api/chat"),
            "127.0.0.1:11434"
        );
    }

    #[test]
    fn ollama_connect_addr_handles_base_url_with_trailing_slash() {
        assert_eq!(
            ollama_connect_addr("http://127.0.0.1:11434/"),
            "127.0.0.1:11434"
        );
    }

    #[test]
    fn ollama_connect_addr_handles_plain_base_url() {
        assert_eq!(
            ollama_connect_addr("http://127.0.0.1:11434"),
            "127.0.0.1:11434"
        );
    }

    #[test]
    fn ollama_connect_addr_uses_known_default_port_when_absent() {
        assert_eq!(ollama_connect_addr("http://localhost"), "localhost:80");
    }
}
