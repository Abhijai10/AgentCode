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
    assert!(
        evidence_proves_sandboxed_test_execution(&evidence),
        "dev.test evidence must prove sandboxed execution (full isolation on \
         capable hosts, or the honest requested/achieved degraded pair); \
         host_supports_filesystem_isolation={}",
        host_supports_filesystem_isolation()
    );
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
    let crash_evidence = db_after.evidence_records().unwrap();
    assert!(
        evidence_proves_sandboxed_test_execution(&crash_evidence),
        "dev.test evidence must prove sandboxed execution (full isolation on \
         capable hosts, or the honest requested/achieved degraded pair); \
         host_supports_filesystem_isolation={}",
        host_supports_filesystem_isolation()
    );
    let evidence_count_after_crash = crash_evidence.len();

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
fn real_provider_daemon_path_model_unavailable_fails_distinctly() {
    // P1-01 model-unavailable distinction: point the real daemon at a model
    // that does not exist on the local Ollama.  The mission must reach a
    // terminal FAILED state with the provider/model failure recorded — never a
    // silent success, never UNPROVEN, never a fabricated completion.  This is
    // deterministic: a nonexistent model always fails identically.
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
    // A model name that cannot exist on the local server.  <=4B constraint is
    // irrelevant because no model is actually loaded for this probe.
    let ollama_model = std::env::var("OLLAMA_MODEL_MISSING")
        .unwrap_or_else(|_| "agentcode-no-such-model".to_string());

    let runtime = short_temp_path("acrm");
    let project = short_temp_path("acrmp");
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
            "id": "submit-missing",
            "command": "SubmitMission",
            "goal": "Create a file named agentcode_proof.txt"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();

    // The mission must reach a terminal FAILED state with a real provider
    // failure — never completed, never cancelled-by-accident, never UNPROVEN.
    // The coordinator uses "failed: ERROR_CODE" to convey the specific cause.
    let failed = poll_mission_state_prefix(&socket, &mission_id, "failed", &mut child, &runtime);
    assert!(
        failed["state"].as_str().unwrap_or("").starts_with("failed"),
        "mission must reach a failed terminal state: {failed}"
    );

    // The daemon log must name the provider failure (not a mission panic and
    // not a fabricated success).
    let log = daemon_log(&runtime);
    assert!(
        log.contains("MISSION-ERROR") || log.contains("MISSION-NOT-COMPLETED"),
        "daemon log must record the real provider failure: {log}"
    );
    // Persisted mission/session are terminal failed, not completed.
    let mission_stable = StableId::from_existing(&mission_id).unwrap();
    let db = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        db.get_mission(&mission_stable).unwrap().unwrap().state,
        "failed"
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
    // Small-model constraint: <=4B only (8 GB RAM host).  Never default to a
    // 7B/8B model.
    let ollama_model =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3b".to_string());

    // Unique prefix: this suite's tests run in parallel threads and share
    // one process id; a colliding prefix races two git inits in one
    // directory (fatal: cannot copy ... File exists).
    let runtime = short_temp_path("acrps");
    let project = short_temp_path("acrpsp");
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

#[test]
#[ignore = "requires Ollama running locally with a loaded model (<=4B)"]
fn real_provider_daemon_path_conversation_chat_to_mission_e2e() {
    // Phase 16 E2E: Project → New Goal Chat → user message → attachment →
    // submit → real daemon → real mission → execution → verification →
    // conversation remains persisted with messages, attachments, mission ref.
    let ollama_base =
        std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let connect_addr = ollama_connect_addr(&ollama_base);
    if std::net::TcpStream::connect(&connect_addr).is_err() {
        eprintln!("SKIP: Ollama not reachable at {ollama_base}; cannot run E2E conversation test");
        return;
    }
    let ollama_chat = if ollama_base.ends_with("/api/chat") {
        ollama_base.clone()
    } else if ollama_base.ends_with('/') {
        format!("{ollama_base}api/chat")
    } else {
        format!("{ollama_base}/api/chat")
    };
    // Small-model constraint: <=4B only (8 GB RAM host).
    let ollama_model =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3b".to_string());

    let runtime = short_temp_path("acrp");
    let project = short_temp_path("acrpp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_smoke_fixture_project(&project);
    // AgentCode-managed state (attachments) lives under .agentcode/ inside the
    // project.  It must be gitignored so writing an attachment does not dirty
    // the base repository before the mission creates its worktree.
    fs::write(project.join(".gitignore"), ".agentcode/\n").unwrap();
    run_git(&project, ["add", ".gitignore"]);
    run_git(
        &project,
        [
            "-c",
            "user.name=AgentCode Test",
            "-c",
            "user.email=agentcode@example.test",
            "commit",
            "-m",
            "ignore agentcode state",
        ],
    );
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

    // 1. Create a GOAL conversation via the real daemon IPC.
    let project_path = project.to_string_lossy().to_string();
    let create = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "e2e-create",
            "command": "ConversationCreate",
            "project_path": project_path,
            "mode": "GOAL",
            "title": "E2E Goal Chat"
        }))
        .unwrap();
    assert_eq!(create["ok"], true, "create conversation: {create}");
    let cid = create["conversation_id"].as_str().unwrap().to_string();

    // 2. Register an attachment: a reference file that the model can inspect.
    let att_dir = project.join(".agentcode").join("attachments").join(&cid);
    fs::create_dir_all(&att_dir).unwrap();
    let att_content = b"reference constant: 42";
    let att_rel = format!(".agentcode/attachments/{cid}/input.txt");
    fs::write(project.join(&att_rel), att_content).unwrap();
    let att_hash = format!(
        "fnv1a64:{:016x}",
        att_content
            .iter()
            .fold(0xcbf29ce484222325_u64, |h, b| (h ^ u64::from(*b))
                .wrapping_mul(0x100000001b3))
    );
    let attach = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "e2e-attach",
            "command": "AttachmentRegister",
            "conversation_id": cid,
            "project_path": project_path,
            "filename": "input.txt",
            "mime_type": "text/plain",
            "size_bytes": att_content.len() as i64,
            "content_hash": att_hash,
            "rel_path": att_rel
        }))
        .unwrap();
    assert_eq!(attach["ok"], true, "register attachment: {attach}");
    let att_id = attach["attachment"]["id"].as_str().unwrap().to_string();

    // 3. Submit the goal via GoalSubmit — this creates a real mission through
    //    the kernel, persists the user message with mission_ref, and links the
    //    mission to the conversation.  The exact original request is preserved.
    let goal = "Create a file named agentcode_smoke.txt containing exactly: AgentCode operational smoke test";
    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "e2e-submit",
            "command": "GoalSubmit",
            "conversation_id": cid,
            "goal": goal,
            "attachment_ids": [att_id]
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "goal submit: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    let session_id = submit["session_id"].as_str().unwrap().to_string();

    // 4. Poll for mission completion (real model execution).
    let completed = poll_mission_completed(&socket, &mission_id, &mut child, &runtime);
    assert_eq!(
        completed["state"], "completed",
        "mission must complete: {completed}"
    );

    // 5. Verify the file was created by the model (ChangeSet execution).
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

    // 6. Verify the conversation remains persisted with messages, mission_ref,
    //    and attachment.
    let get_conv = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "e2e-get",
            "command": "ConversationGet",
            "conversation_id": cid
        }))
        .unwrap();
    assert_eq!(
        get_conv["ok"], true,
        "get conversation after mission: {get_conv}"
    );
    assert_eq!(
        get_conv["conversation"]["current_mission_id"]
            .as_str()
            .unwrap(),
        mission_id,
        "conversation must still reference the completed mission"
    );
    let msgs = get_conv["conversation"]["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 1, "goal message must be preserved");
    assert_eq!(msgs[0]["role"], "user");
    assert_eq!(msgs[0]["content"], goal);
    assert_eq!(
        msgs[0]["mission_ref"].as_str().unwrap(),
        mission_id,
        "message must reference the mission"
    );
    let conv_atts = get_conv["conversation"]["attachments"].as_array().unwrap();
    assert!(
        conv_atts
            .iter()
            .any(|a| a["id"].as_str().unwrap() == att_id),
        "attachment must still be associated with the conversation"
    );

    // 7. Verify the persisted mission/session are terminal completed.
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
    // Verify real provider evidence (not mock/scripted).
    let real_provider_evidence = evidence.iter().any(|record| {
        record.provenance.source == "agent.provider.planner"
            && !record.artifact_uri.contains("mock")
            && !record.artifact_uri.contains("scripted")
    });
    assert!(
        real_provider_evidence,
        "real provider must produce planner evidence from a real provider, not mock or scripted"
    );

    // 8. Verify the conversation + attachment + message survive a daemon restart.
    shutdown_daemon(&socket, &mut child, &runtime);
    let mut restarted = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut restarted, &runtime);
    let after_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "e2e-restart",
            "command": "ConversationGet",
            "conversation_id": cid
        }))
        .unwrap();
    assert_eq!(after_restart["ok"], true, "after restart: {after_restart}");
    assert_eq!(
        after_restart["conversation"]["current_mission_id"]
            .as_str()
            .unwrap(),
        mission_id,
        "mission ref must survive restart"
    );
    let after_msgs = after_restart["conversation"]["messages"]
        .as_array()
        .unwrap();
    assert_eq!(after_msgs.len(), 1, "messages must survive restart");
    let after_atts = after_restart["conversation"]["attachments"]
        .as_array()
        .unwrap();
    assert!(!after_atts.is_empty(), "attachments must survive restart");

    shutdown_daemon(&socket, &mut restarted, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_pauses_active_mission_blocks_forward_work_then_resumes() {
    // P1-02 active-mission pause over real IPC: a RUNNING mission must pause
    // (no forward work toward completion), remain paused, then resume and
    // complete.  Uses the slow fixture so verification runs a 5s test in the
    // background, giving a deterministic active window.
    let runtime = short_temp_path("acap");
    let project = short_temp_path("acapp");
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
    let mut child = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut child, &runtime);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-pause-active",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();

    // Wait until verification (the 5s slow test) is running — the mission is
    // genuinely active with forward work in flight.
    poll_task_state(&db_path, &mission_id, "Verify target", "running");

    let pause = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "pause-active",
            "command": "PauseMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(pause["ok"], true, "pause response: {pause}");
    assert_eq!(pause["state"], "paused", "pause response: {pause}");
    let paused = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "paused-active",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(paused["state"], "paused", "paused response: {paused}");
    // While paused, the mission must not reach a terminal state even though
    // the in-flight 5s verification would have finished by now.
    std::thread::sleep(Duration::from_secs(7));
    let still_paused = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "still-paused",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(
        still_paused["state"], "paused",
        "mission must stay paused (no forward work): {still_paused}"
    );

    let resume = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "resume-active",
            "command": "ResumeMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(resume["ok"], true, "resume response: {resume}");
    let completed = poll_mission_completed(&socket, &mission_id, &mut child, &runtime);
    assert_eq!(completed["state"], "completed", "{completed}");

    shutdown_daemon(&socket, &mut child, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_cancels_queued_mission_never_executes() {
    // P1-02 queued cancellation over real IPC: a queued mission cancelled
    // before execution must never run — no worktree, no mutation, remains
    // cancelled across restart.
    let runtime = short_temp_path("acqc");
    let project = short_temp_path("acqcp");
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
    let mut child = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut child, &runtime);

    // First mission occupies the single worker slot.
    let first = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-first",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(first["ok"], true, "first response: {first}");
    poll_task_state(
        &db_path,
        first["mission_id"].as_str().unwrap(),
        "Verify target",
        "running",
    );

    // Second mission is queued behind it.
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

    let cancel = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "cancel-queued",
            "command": "CancelMission",
            "mission_id": second_mission
        }))
        .unwrap();
    assert_eq!(cancel["ok"], true, "cancel response: {cancel}");
    assert_eq!(cancel["state"], "cancelled", "cancel response: {cancel}");

    // The cancelled queued mission must never create a worktree or mutate.
    std::thread::sleep(Duration::from_secs(2));
    assert!(
        !project
            .join(".agentcode-worktrees")
            .join(&second_session)
            .exists(),
        "cancelled queued mission must not create a worktree"
    );
    let cancelled = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "cancelled-queued",
            "command": "GetMission",
            "mission_id": second_mission
        }))
        .unwrap();
    assert_eq!(cancelled["state"], "cancelled", "{cancelled}");

    // First mission still completes normally.
    let first_completed = poll_mission_completed(
        &socket,
        first["mission_id"].as_str().unwrap(),
        &mut child,
        &runtime,
    );
    assert_eq!(first_completed["state"], "completed", "{first_completed}");

    // Restart: the cancelled queued mission stays cancelled and never ran.
    shutdown_daemon(&socket, &mut child, &runtime);
    let mut restarted = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut restarted, &runtime);
    let after_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "cancelled-after-restart",
            "command": "GetMission",
            "mission_id": second_mission
        }))
        .unwrap();
    assert_eq!(
        after_restart["state"], "cancelled",
        "cancelled queued mission must stay cancelled: {after_restart}"
    );
    assert!(
        !project
            .join(".agentcode-worktrees")
            .join(&second_session)
            .exists(),
        "cancelled queued mission must never create a worktree after restart"
    );
    shutdown_daemon(&socket, &mut restarted, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_terminates_in_flight_subprocess_on_cancel() {
    // P1-04: an actual subprocess running under the command-execution path
    // must be terminated when cancellation arrives.  The fixture verification
    // spawns a deterministic `sleep 300` probe and writes its PID to a file in
    // the worktree; after cancel, the probe must be dead (no orphan), the
    // mission must become Cancelled (not Failed), and nothing is retried.
    let runtime = short_temp_path("acsp");
    let project = short_temp_path("acspp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_probe_fixture_project(&project);
    let daemon = daemon_binary();
    assert!(
        daemon.is_file(),
        "build the daemon first: cargo build -p ac-daemon"
    );
    let socket = default_socket_path(&runtime);
    let (_, _) = default_paths(&runtime);
    let mut child = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut child, &runtime);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-probe",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    let session_id = submit["session_id"].as_str().unwrap().to_string();

    // Wait for the probe subprocess to be running: its PID file appears in the
    // worktree when the fixture test binary (child of cargo test, itself the
    // command executed by dev.test) has spawned `sleep 300`.
    let probe_pid = poll_probe_pid(&project, &session_id, &mut child, &runtime);
    assert!(
        process_is_alive(probe_pid),
        "probe subprocess must be alive before cancel"
    );

    let cancel = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "cancel-probe",
            "command": "CancelMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(cancel["ok"], true, "cancel response: {cancel}");

    // Mission must become Cancelled, never Failed.
    let cancelled = poll_mission_state(&socket, &mission_id, "cancelled", &mut child, &runtime);
    assert_eq!(cancelled["state"], "cancelled", "{cancelled}");

    // The probe subprocess must be terminated — no orphan process remains.
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if !process_is_alive(probe_pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !process_is_alive(probe_pid),
        "probe subprocess {probe_pid} must be terminated after cancellation"
    );

    // Cancelled command is not retried: the mission stays terminal and the
    // task-attempt count does not grow after cancellation.
    std::thread::sleep(Duration::from_secs(2));
    let after_wait = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "probe-after",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(after_wait["state"], "cancelled", "{after_wait}");

    // Restart: cancelled stays cancelled; no replay, no retry.
    shutdown_daemon(&socket, &mut child, &runtime);
    let mut restarted = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut restarted, &runtime);
    let after_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "probe-after-restart",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(
        after_restart["state"], "cancelled",
        "cancelled probe mission must stay cancelled: {after_restart}"
    );
    assert!(
        !process_is_alive(probe_pid),
        "probe subprocess must stay terminated after restart"
    );
    shutdown_daemon(&socket, &mut restarted, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_restarts_during_verification_without_replay() {
    // P1-03: restart while a completed mutation exists and verification is
    // still running.  The completed mutation must not be replayed, evidence
    // must not be duplicated, and the mission must resume to completion.
    let runtime = short_temp_path("acrv");
    let project = short_temp_path("acrvp");
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

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-verify-restart",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    let session_id = submit["session_id"].as_str().unwrap().to_string();

    // The mutation task completes, then verification starts (5s slow test).
    let modify_task = poll_task_state(&db_path, &mission_id, "Modify target", "completed");
    poll_task_state(&db_path, &mission_id, "Verify target", "running");
    let worktree_lib = project
        .join(".agentcode-worktrees")
        .join(&session_id)
        .join("src/lib.rs");
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}"
    );
    let db_before = ControlPlaneDb::open(&db_path).unwrap();
    let modify_attempts_before = db_before.task_attempts(&modify_task).unwrap().len();
    let evidence_before = db_before.evidence_records().unwrap().len();

    // Kill mid-verification, then restart with the same runtime directory.
    first.kill().unwrap();
    wait_for_exit(&mut first, &runtime);
    let mut second = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut second, &runtime);

    let completed = poll_mission_completed(&socket, &mission_id, &mut second, &runtime);
    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        fs::read_to_string(&worktree_lib).unwrap(),
        "pub fn fixture_answer() -> u32 {\n    42\n}",
        "mutation must not be replayed"
    );
    assert_eq!(
        count_occurrences(&fs::read_to_string(&worktree_lib).unwrap(), "42"),
        1
    );
    let db_after = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        db_after.task_attempts(&modify_task).unwrap().len(),
        modify_attempts_before,
        "completed mutation task was replayed after restart"
    );
    assert!(
        db_after.evidence_records().unwrap().len() >= evidence_before,
        "evidence must not be lost"
    );

    // Identity is stable: same mission/session across the crash boundary.
    let same_mission = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "verify-identity",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(same_mission["mission_id"], mission_id, "{same_mission}");
    assert_eq!(same_mission["session_id"], session_id, "{same_mission}");

    // Second restart: terminal Completed persists, nothing replays.
    shutdown_daemon(&socket, &mut second, &runtime);
    let mut third = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut third, &runtime);
    let after_second_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "verify-second-restart",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(
        after_second_restart["state"], "completed",
        "second restart: {after_second_restart}"
    );
    assert_eq!(
        count_occurrences(&fs::read_to_string(&worktree_lib).unwrap(), "42"),
        1
    );
    let db_third = ControlPlaneDb::open(&db_path).unwrap();
    assert_eq!(
        db_third.task_attempts(&modify_task).unwrap().len(),
        modify_attempts_before,
        "completed mutation replayed after second restart"
    );
    shutdown_daemon(&socket, &mut third, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

#[test]
#[ignore = "requires built ac-daemon binary and macOS production sandbox"]
fn real_daemon_binary_graceful_shutdown_with_in_flight_subprocess() {
    // P1-05: ShutdownDaemon while a mission is actively running a subprocess.
    // Shutdown must reconcile the mission (cancel in-flight work), clean up the
    // child process, remove the lock, and let the daemon exit without a
    // surviving worker thread.
    let runtime = short_temp_path("acsh");
    let project = short_temp_path("acshp");
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&runtime).unwrap();
    create_probe_fixture_project(&project);
    let daemon = daemon_binary();
    assert!(
        daemon.is_file(),
        "build the daemon first: cargo build -p ac-daemon"
    );
    let socket = default_socket_path(&runtime);
    let (db_path, lock) = default_paths(&runtime);
    let mut child = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut child, &runtime);

    let submit = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "submit-shutdown",
            "command": "SubmitMission",
            "goal": "Fix the bug in src/lib.rs"
        }))
        .unwrap();
    assert_eq!(submit["ok"], true, "submit response: {submit}");
    let mission_id = submit["mission_id"].as_str().unwrap().to_string();
    let session_id = submit["session_id"].as_str().unwrap().to_string();

    // Wait until the probe subprocess is actually running.
    let probe_pid = poll_probe_pid(&project, &session_id, &mut child, &runtime);
    assert!(process_is_alive(probe_pid));

    // Graceful shutdown while the subprocess is in flight.
    shutdown_daemon(&socket, &mut child, &runtime);

    // The child process must be cleaned up (no orphan survives shutdown).
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if !process_is_alive(probe_pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !process_is_alive(probe_pid),
        "in-flight subprocess must be terminated during graceful shutdown"
    );
    // Lock and socket are removed by the daemon process exit path.
    assert!(!lock.exists(), "daemon lock must be removed after shutdown");
    assert!(
        !socket.exists(),
        "daemon socket must be removed after shutdown"
    );

    // The in-flight mission is reconciled to a terminal state (cancelled
    // or failed — shutdown cancels in-flight work, so cancelled is expected
    // but a clean reconciliation is what matters).
    let db = ControlPlaneDb::open(&db_path).unwrap();
    let session = db
        .session_for_mission(&StableId::from_existing(&mission_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        session.state,
        "cancelled",
        "in-flight mission must be reconciled on graceful shutdown, got state={} log={}",
        session.state,
        daemon_log(&runtime)
    );
    let mut restarted = launch_daemon(&daemon, &runtime, &project);
    wait_for_socket(&socket, &mut restarted, &runtime);
    let after_restart = UnixIpcClient::new(&socket)
        .request(json!({
            "id": "shutdown-after-restart",
            "command": "GetMission",
            "mission_id": mission_id
        }))
        .unwrap();
    assert_eq!(
        after_restart["state"], "cancelled",
        "reconciled mission must stay terminal: {after_restart}"
    );
    shutdown_daemon(&socket, &mut restarted, &runtime);
    let _ = fs::remove_dir_all(&runtime);
    let _ = fs::remove_dir_all(&project);
}

fn short_temp_path(prefix: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/{prefix}-{}", std::process::id()))
}

/// Does this host actually deliver OS filesystem isolation (seatbelt
/// sandbox-exec on macOS)?  On hosts where the mechanism is degraded, mission
/// dev.test evidence honestly records the degraded pair
/// (requested FilesystemIsolated / achieved ProcessRestricted) instead of the
/// full-isolation proof, and the isolation-evidence assertions below accept
/// that honest pair rather than fabricating a pass.
fn host_supports_filesystem_isolation() -> bool {
    let probe = short_temp_path("iso-probe");
    let _ = fs::remove_dir_all(&probe);
    fs::create_dir_all(&probe).unwrap();
    let supported =
        ac_sandbox::SandboxManager::new(ac_sandbox::SandboxPolicy::new(vec![probe.clone()]))
            .diagnostics()
            .max_isolation
            >= ac_sandbox::IsolationLevel::FilesystemIsolated;
    let _ = fs::remove_dir_all(&probe);
    supported
}

/// dev.test evidence is honest about isolation: either full filesystem
/// isolation on capable hosts, or the recorded degraded pair
/// (requested FilesystemIsolated, achieved ProcessRestricted) where the OS
/// backend cannot deliver it.  Both prove the tool executed under the
/// sandbox-evidence contract; anything else is a failure.
fn evidence_proves_sandboxed_test_execution(records: &[ac_evidence::EvidenceRecord]) -> bool {
    records.iter().any(|record| {
        let Some(content) = record.raw_content.as_deref() else {
            return false;
        };
        if record.provenance.tool.as_deref() != Some("dev.test") {
            return false;
        }
        content.contains("achieved_isolation:FilesystemIsolated")
            || (content.contains("requested_isolation:FilesystemIsolated")
                && content.contains("achieved_isolation:ProcessRestricted"))
    })
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

/// A fixture whose verification step spawns a deterministic long-running
/// subprocess (`sleep 300`) and records its PID in a worktree file named
/// `agentcode_probe.pid`.  The mission runs through the real daemon path; the
/// test reads the PID to prove the subprocess is alive, then after
/// cancellation proves it was terminated (no orphan) and the mission became
/// Cancelled rather than Failed.
fn create_probe_fixture_project(root: &Path) {
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
    // The fixture test spawns a subprocess that lives long enough to observe
    // cancellation, writes the child PID to the worktree root, then waits.
    fs::write(
        root.join("tests/fixture.rs"),
        r#"use std::io::Write;

#[test]
fn probe_runs_long_subprocess() {
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("300")
        .spawn()
        .expect("spawn probe");
    let mut marker = std::fs::File::create(concat!(env!("CARGO_MANIFEST_DIR"), "/agentcode_probe.pid")).unwrap();
    writeln!(marker, "{}", child.id()).unwrap();
    let _ = child.wait();
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

/// Poll the worktree for the probe subprocess PID file and return the PID.
/// `dev.test` runs `cargo test` inside the sandboxed worktree, so the marker
/// file appears at `<project>/.agentcode-worktrees/<session>/agentcode_probe.pid`.
fn poll_probe_pid(project: &Path, session_id: &str, child: &mut Child, runtime: &Path) -> u32 {
    let marker = project
        .join(".agentcode-worktrees")
        .join(session_id)
        .join("agentcode_probe.pid");
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        assert_process_running(child, runtime);
        if let Ok(content) = fs::read_to_string(&marker) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                return pid;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!(
        "probe PID file was never written: {}\n{}",
        marker.display(),
        daemon_log(runtime)
    );
}

fn process_is_alive(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
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

/// Poll GetMission until `state` starts with `expected_prefix`.  The
/// coordinator renders terminal failures as `failed: <CODE>`, so a prefix
/// match is required to accept the typed failure while still rejecting
/// completed/cancelled/silent outcomes.
fn poll_mission_state_prefix(
    socket: &Path,
    mission_id: &str,
    expected_prefix: &str,
    child: &mut Child,
    runtime: &Path,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = json!(null);
    while Instant::now() < deadline {
        assert_process_running(child, runtime);
        last = UnixIpcClient::new(socket)
            .request(json!({
                "id": "poll-state-prefix",
                "command": "GetMission",
                "mission_id": mission_id
            }))
            .unwrap();
        if last["state"]
            .as_str()
            .is_some_and(|state| state.starts_with(expected_prefix))
        {
            return last;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "mission did not reach {expected_prefix}*; last={last}\n{}",
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
