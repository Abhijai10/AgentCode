use ac_config::SettingsBuilder;
use ac_daemon::{default_paths, DaemonService};
use ac_logging::{Redactor, Severity, StructuredLogger};

fn main() {
    let logger = StructuredLogger::new("ac-daemon", Redactor::new());
    let _settings = SettingsBuilder::new()
        .default("daemon.mode", "local")
        .build()
        .expect("compiled defaults must be valid");
    let base = std::env::temp_dir().join("agentcode-daemon");
    std::fs::create_dir_all(&base).expect("daemon temp dir should be creatable");
    let (db, lock) = default_paths(base);
    let mut daemon = DaemonService::open(db, lock).expect("daemon should open");
    daemon.start().expect("daemon should start");
    let record = logger.record(Severity::Info, "AgentCode daemon foundation started");
    println!("{} {:?}", record.component, record.severity);
    daemon.stop().expect("daemon should stop");
}
