use ac_config::SettingsBuilder;
use ac_daemon::{default_paths, default_socket_path, DaemonService, UnixIpcServer};
use ac_logging::{Redactor, Severity, StructuredLogger};

fn main() {
    let logger = StructuredLogger::new("ac-daemon", Redactor::new());
    let _settings = SettingsBuilder::new()
        .default("daemon.mode", "local")
        .build()
        .expect("compiled defaults must be valid");
    let base = std::env::var_os("AGENTCODE_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("agentcode-daemon"));
    std::fs::create_dir_all(&base).expect("daemon temp dir should be creatable");
    let (db, lock) = default_paths(&base);
    let mut daemon = DaemonService::open(db, lock).expect("daemon should open");
    daemon.start().expect("daemon should start");
    let socket = default_socket_path(&base);
    let (ipc, listener) = UnixIpcServer::bind(socket).expect("daemon IPC should bind");
    let record = logger.record(Severity::Info, "AgentCode daemon started");
    println!("{} {:?}", record.component, record.severity);
    loop {
        if ipc
            .serve_once(&listener, &mut daemon)
            .expect("daemon IPC should serve")
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    ipc.cleanup();
}
