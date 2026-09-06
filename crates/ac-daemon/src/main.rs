use ac_config::SettingsBuilder;
use ac_daemon::{
    default_paths, default_runtime_dir, default_socket_path, DaemonService, UnixIpcServer,
};
use ac_logging::{Redactor, Severity, StructuredLogger};

fn main() {
    let logger = StructuredLogger::new("ac-daemon", Redactor::new());
    let _settings = SettingsBuilder::new()
        .default("daemon.mode", "local")
        .build()
        .expect("compiled defaults must be valid");
    let base = std::env::var_os("AGENTCODE_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(default_runtime_dir)
        .unwrap_or_else(|error| {
            eprintln!("daemon runtime initialization failed: {error}");
            std::process::exit(1);
        });
    if let Err(error) = std::fs::create_dir_all(&base) {
        eprintln!("daemon runtime initialization failed: {error}");
        std::process::exit(1);
    }
    let (db, lock) = default_paths(&base);
    let mut daemon = DaemonService::open(db, lock).expect("daemon should open");
    daemon.start().expect("daemon should start");
    let socket = default_socket_path(&base);
    // Stale-socket recovery (F7): if a previous daemon died without cleanup,
    // the socket file may still exist and bind() would fail.  Probe it: if
    // nothing is listening, remove it honestly; if something IS listening,
    // refuse to start (another daemon owns it) with a clear error.
    if socket.exists() {
        let alive = std::os::unix::net::UnixStream::connect(&socket).is_ok();
        if alive {
            eprintln!(
                "another AgentCode daemon is already serving at {}",
                socket.display()
            );
            std::process::exit(1);
        }
        eprintln!("removing stale daemon socket at {}", socket.display());
        let _ = std::fs::remove_file(&socket);
    }
    let (ipc, listener) = UnixIpcServer::bind(socket).expect("daemon IPC should bind");
    // F7: signal handlers AFTER bind (cleanup meaningful only from here).
    // Confined to the ac-signals crate (the workspace's single sanctioned
    // unsafe home — see DEP-ADM-021).
    ac_signals::install_shutdown_handler();
    let record = logger.record(Severity::Info, "AgentCode daemon started");
    println!("{} {:?}", record.component, record.severity);
    loop {
        if ac_signals::shutdown_requested() {
            eprintln!("daemon: shutdown signal received; stopping cleanly");
            break;
        }
        if ipc
            .serve_once(&listener, &mut daemon)
            .expect("daemon IPC should serve")
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // Clean stop: reaps design-preview + terminal children, removes the
    // socket file.  Never skip even on the error path — best effort.
    if let Err(error) = daemon.stop() {
        eprintln!("daemon stop reported: {error}");
    }
    ipc.cleanup();
}
