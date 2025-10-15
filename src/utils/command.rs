use std::{io::Read as _, path::Path, process::Command, time::Duration};

use indicatif::{ProgressBar, ProgressStyle};

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

    pub fn copy_file_to_remote(
        remote: &RemoteInfo,
        local_path: &str,
        remote_path: &Path,
    ) -> EmptyResult {
        let sess = remote.get_sess()?;

        // Open SFTP session
        let sftp = sess
            .sftp()
            .map_err(|e| format!("Failed to open SFTP session: {}", e))?;

        // Open the local file for reading
        let mut local_file = std::fs::File::open(local_path)
            .map_err(|e| format!("Failed to open local file '{}': {}", local_path, e))?;

        // Create or truncate the remote file for writing
        let mut remote_file = sftp
            .create(remote_path)
            .map_err(|e| format!("Failed to create remote file '{:?}': {}", remote_path, e))?;

        // Copy the contents from the local file to the remote file
        std::io::copy(&mut local_file, &mut remote_file)
            .map_err(|e| format!("Failed to copy to remote file '{:?}': {}", remote_path, e))?;

        Ok(())
    }

    pub fn sync_dir_to_remote(
        remote: &RemoteInfo,
        local_path: &str,
        remote_path: &str,
    ) -> EmptyResult {
        CommandUtils::run_command_str(&format!("mkdir -p {}", remote_path), Some(remote))?;

        let ssh_target = format!("{}@{}", remote.user, remote.host);
        let ssh_cmd = format!("ssh -p {}", remote.port);

        let mut command = Command::new("sshpass");
        command.args([
            "-p",
            &remote.password,
            "rsync",
            "-azP",
            "--delete",
            "--exclude",
            "build/",
            "--exclude",
            ".dart_tool/",
            "--exclude",
            ".git/",
            "-e",
            &ssh_cmd,
            &format!("{}/", local_path.trim_end_matches('/')),
            &format!("{}:{}/", ssh_target, remote_path.trim_end_matches('/')),
        ]);

        let output = command.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("rsync failed: {}", stderr).into());
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
