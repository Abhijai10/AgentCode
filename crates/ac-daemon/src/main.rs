use ac_config::SettingsBuilder;
use ac_kernel::{AllowAllPolicy, Kernel};
use ac_logging::{Redactor, Severity, StructuredLogger};

fn main() {
    let logger = StructuredLogger::new("ac-daemon", Redactor::new());
    let _settings = SettingsBuilder::new()
        .default("daemon.mode", "foundation")
        .build()
        .expect("compiled defaults must be valid");
    let mut kernel = Kernel::new(AllowAllPolicy);
    kernel.start().expect("kernel should start");
    let record = logger.record(Severity::Info, "AgentCode daemon foundation started");
    println!("{} {:?}", record.component, record.severity);
}
