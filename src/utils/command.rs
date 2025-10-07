use std::{process::Command, time::Duration};

use indicatif::{ProgressBar, ProgressStyle};

use crate::utils::errors::{ResultTrait, ResultWithError};

#[allow(dead_code)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

pub struct CommandUtils {}

impl CommandUtils {
    pub fn run_command_str(cmd: &str) -> ResultWithError<CommandOutput> {
        let res = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .auto_err("failed to run command")?;

        let output = String::from_utf8_lossy(&res.stdout);
        let stderr = String::from_utf8_lossy(&res.stderr);

        Ok(CommandOutput {
            stdout: output.trim().to_string(),
            stderr: stderr.trim().to_string(),
            status: res.status.code().unwrap_or(-1),
        })
    }

    pub fn display_loader(msg: String) -> ProgressBar {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠸", "⠴", "⠦", "⠇", "✔"]),
        );
        spinner.set_message(msg);
        spinner.enable_steady_tick(Duration::from_millis(80));
        spinner
    }
}
