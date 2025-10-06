use std::process::Command;

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
}
