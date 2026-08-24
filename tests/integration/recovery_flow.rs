use ac_common::StableId;
use ac_daemon::{default_paths, DaemonLifecycle, DaemonLifecycleRuntime, DaemonService};

#[test]
fn recovery_flow_reconstructs_interrupted_session_after_reload() {
    let dir =
        std::env::temp_dir().join(format!("agentcode-recovery-flow-{}", StableId::new("tmp")));
    std::fs::create_dir_all(&dir).unwrap();
    let (db, lock) = default_paths(&dir);
    let session_id;
    {
        let mut daemon = DaemonService::open(&db, &lock).unwrap();
        DaemonLifecycleRuntime::start(&mut daemon).unwrap();
        let response = daemon
            .handle(ac_daemon::DaemonCommand::CreateSession {
                goal: "Recover integration task".to_string(),
            })
            .unwrap();
        session_id = match response {
            ac_daemon::DaemonResponse::SessionCreated { session_id, .. } => session_id,
            _ => panic!("expected session creation"),
        };
        daemon
            .handle(ac_daemon::DaemonCommand::CheckpointSession {
                session_id,
                next_step: 2,
            })
            .unwrap();
    }

    let mut recovered = DaemonService::open(&db, &lock).unwrap();
    DaemonLifecycleRuntime::start(&mut recovered).unwrap();
    assert_eq!(recovered.health().lifecycle, DaemonLifecycle::Running);
    assert!(!recovered.recovered_sessions().is_empty());
    DaemonLifecycleRuntime::shutdown(&mut recovered).unwrap();

    let _ = std::fs::remove_dir_all(dir);
}
