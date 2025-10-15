use std::{fs, io::Read as _, path::Path, time::Duration};

use indicatif::{ProgressBar, ProgressStyle};
use ssh2::Sftp;
use tracing::info;

use crate::{
    models::app_state::{CommandOutput, RemoteInfo},
    utils::errors::{EmptyResult, ResultWithError},
};

pub struct CommandUtils {}

impl CommandUtils {
    pub fn run_command_str(
        cmd: &str,
        remote: Option<&RemoteInfo>,
    ) -> ResultWithError<CommandOutput> {
        if let Some(remote) = remote {
            let res = remote.exec(cmd)?;
            Ok(res)
        } else {
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()?;
            Ok(CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                status: output.status.code().unwrap_or(-1),
            })
        }
    }

    pub fn copy_dir_to_remote(
        remote: &RemoteInfo,
        local_dir: &Path,
        remote_dir: &Path,
    ) -> EmptyResult {
        info!(
            "Copying directory {:?} to remote {:?}",
            local_dir, remote_dir
        );

        let sess = remote.get_sess()?;
        let sftp = sess.sftp()?;
        Self::upload_recursive(&sftp, local_dir, remote_dir)?;

        Ok(())
    }

    fn upload_recursive(sftp: &Sftp, local_path: &Path, remote_path: &Path) -> EmptyResult {
        // Ensure remote dir exists
        let _ = sftp.mkdir(remote_path, 0o755);

        for entry in fs::read_dir(local_path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let local_item = entry.path();
            let remote_item = remote_path.join(entry.file_name());

            if file_type.is_dir() {
                Self::upload_recursive(sftp, &local_item, &remote_item)?;
            } else if file_type.is_file() {
                let mut remote_file = sftp.create(&remote_item)?;
                let mut local_file = fs::File::open(&local_item)?;
                std::io::copy(&mut local_file, &mut remote_file)?;
            }
        }

        Ok(())
    }

    /// Fetches the contents of a file from a remote host over SSH
    pub fn fetch_remote_file(remote: &RemoteInfo, remote_path: &str) -> ResultWithError<String> {
        let sess = remote.get_sess()?;

        // Open SFTP session
        let sftp = sess
            .sftp()
            .map_err(|e| format!("Failed to open SFTP session: {}", e))?;

        // Open the remote file for reading
        let mut remote_file = sftp
            .open(remote_path)
            .map_err(|e| format!("Failed to open remote file '{}': {}", remote_path, e))?;

        // Read the file contents
        let mut contents = String::new();
        remote_file
            .read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read remote file '{}': {}", remote_path, e))?;

        Ok(contents)
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
