use std::process;

use tracing::{error, info};

use crate::code_gen::flutter::GenFlutter;

impl GenFlutter {
    pub fn run_dart_fix(&self) {
        let path = self.out_dir.clone().to_string_lossy().to_string();
        info!("Running dart fix on {}", path);

        let status = process::Command::new("dart")
            .args(["fix", path.as_str(), "--apply"])
            .status();

        match status {
            Ok(s) if s.success() => info!("✅ Dart fix completed successfully."),
            Ok(s) => error!("❌ Dart fix failed with status: {}", s),
            Err(e) => error!("❌ Failed to run dart fix: {}", e),
        }
    }

    pub fn run_dart_format(&self) {
        let path = self.out_dir.clone().to_string_lossy().to_string();
        info!("Running dart format on {}", path);

        let status = process::Command::new("dart")
            .args(["format", path.as_str()])
            .status();

        match status {
            Ok(s) if s.success() => info!("✅ Dart format completed successfully."),
            Ok(s) => error!("❌ Dart format failed with status: {}", s),
            Err(e) => error!("❌ Failed to run dart format: {}", e),
        }
    }
}
