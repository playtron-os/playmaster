use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;

use tracing::error;

use crate::utils::dir::DirUtils;
use crate::utils::errors::EmptyResult;

pub struct FileLogger {
    log_path: PathBuf,
}

impl FileLogger {
    pub fn new(file_name: &str) -> Self {
        let mut log_dir = DirUtils::config_dir().unwrap_or(PathBuf::from("./"));
        log_dir.push("logs");

        if let Err(err) = create_dir_all(&log_dir) {
            error!("Failed to create log directory: {}", err);
        }

        let mut log_path = log_dir;
        log_path.push(file_name);
        FileLogger { log_path }
    }

    pub fn log(&self, message: &str) {
        if let Err(err) = self._log(message) {
            error!(
                "Failed to write to log file {}: {}",
                self.log_path.display(),
                err
            );
        }
    }

    fn _log(&self, message: &str) -> EmptyResult {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(file, "[{}] {}", timestamp, message)?;
        Ok(())
    }
}
