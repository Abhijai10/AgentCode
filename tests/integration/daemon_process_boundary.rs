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

    shutdown_daemon(&socket, &mut second, &runtime);
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

fn short_temp_path(prefix: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/{prefix}-{}", std::process::id()))
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
    let deadline = Instant::now() + Duration::from_secs(90);
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
        assert_ne!(
            last["state"],
            "failed",
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
