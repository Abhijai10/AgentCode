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
