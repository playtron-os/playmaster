use tracing_subscriber::{EnvFilter, fmt};

pub struct LoggerUtils {}

impl LoggerUtils {
    pub fn init() {
        fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_target(false) // Optional: hide module names
            .with_level(true) // Show level (INFO, DEBUG, etc.)
            .compact() // Compact single-line format for CLI tools
            .init();
    }
}
