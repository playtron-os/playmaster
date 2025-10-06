use tracing_subscriber::{EnvFilter, fmt};

pub struct LoggerUtils {}

impl LoggerUtils {
    pub fn init() {
        fmt()
            .with_env_filter(
                EnvFilter::from_default_env() // Enables RUST_LOG=debug or crate=trace
                    .add_directive("warn".parse().unwrap()), // Default level if not set
            )
            .with_target(false) // Optional: hide module names
            .with_level(true) // Show level (INFO, DEBUG, etc.)
            .compact() // Compact single-line format for CLI tools
            .init();
    }
}
