use std::{
    io::Read as _,
    path::Path,
    process::{Child, Command},
    sync::Mutex,
    time::Duration,
};

use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use tracing::{error, info};

use crate::{
    models::app_state::{CommandOutput, RemoteInfo},
    utils::{
        errors::{EmptyResult, ResultTrait, ResultWithError},
        file_logger::FileLogger,
    },
};

struct CmdInfo {
    name: String,
    child: Child,
}

struct RemoteCmdInfo {
    command: String,
    remote: RemoteInfo,
}

lazy_static::lazy_static! {
    static ref RUNNING_CMDS: Mutex<Vec<CmdInfo>> = Mutex::new(Vec::new());
    static ref RUNNING_REMOTE_CMDS: Mutex<Vec<RemoteCmdInfo>> = Mutex::new(Vec::new());
}

pub struct CommandUtils {}

impl CommandUtils {
    pub fn track_cmd(name: &str, child: Child) -> EmptyResult {
        let mut vec = RUNNING_CMDS
            .lock()
            .auto_err("Failed to lock RUNNING_CMDS")?;
        vec.push(CmdInfo {
            name: name.to_string(),
            child,
        });
        Ok(())
    }

    pub fn track_remote_cmd(command: String, remote: RemoteInfo) -> EmptyResult {
        let mut vec = RUNNING_REMOTE_CMDS
            .lock()
            .auto_err("Failed to lock RUNNING_REMOTE_CMDS")?;
        vec.push(RemoteCmdInfo { command, remote });
        Ok(())
    }

    pub fn terminate_all_cmds(root_dir: &str) -> EmptyResult {
        Self::terminate_local_cmds()?;
        Self::terminate_remote_cmds(root_dir)?;
        Ok(())
    }

    fn terminate_local_cmds() -> EmptyResult {
        let mut vec = RUNNING_CMDS
            .lock()
            .auto_err("Failed to lock RUNNING_CMDS")?;

        for mut cmd in vec.drain(..) {
            let _ = cmd.child.kill();
            let output = cmd.child.wait_with_output();

            match output {
                Ok(output) => {
                    info!(
                        "Terminated command '{}' with status: {}",
                        cmd.name, output.status
                    );

                    let stdout_file_logger =
                        FileLogger::new(&format!("cmd_{}_stdout.log", cmd.name));
                    let stderr_file_logger =
                        FileLogger::new(&format!("cmd_{}_stderr.log", cmd.name));

                    stdout_file_logger.log(&String::from_utf8_lossy(&output.stdout));
                    stderr_file_logger.log(&String::from_utf8_lossy(&output.stderr));
                }
                Err(e) => {
                    error!("Failed to wait for command '{}': {}", cmd.name, e);
                }
            }
        }

        Ok(())
    }

    fn terminate_remote_cmds(root_dir: &str) -> EmptyResult {
        let mut vec = RUNNING_REMOTE_CMDS
            .lock()
            .auto_err("Failed to lock RUNNING_REMOTE_CMDS")?;

        for command in vec.drain(..) {
            info!("Terminating remote command: {}", command.command);
            Self::run_command_str(
                &format!("pkill -f \"{}\"", command.command),
                Some(&command.remote),
                root_dir,
            )?;
        }

        Ok(())
    }

    pub fn run_command_str(
        cmd: &str,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> ResultWithError<CommandOutput> {
        let cmd = CommandUtils::with_env_source(root_dir, cmd)?;

        if let Some(remote) = remote {
            let res = remote.exec(&cmd)?;
            Ok(res)
        } else {
            let output = std::process::Command::new("bash")
                .arg("-c")
                .arg(cmd)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
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
        root_dir: &str,
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

        if let Err(err) = Self::run_command_str(
            &format!("chmod +x {}", remote_path.to_string_lossy()),
            Some(remote),
            root_dir,
        ) {
            error!(
                "Failed to set execute permissions on remote file '{:?}': {}",
                remote_path, err
            );
        }

        Ok(())
    }

    pub fn sync_dir_to_remote(
        remote: &RemoteInfo,
        root_dir: &str,
        local_path: &str,
        remote_path: &str,
    ) -> EmptyResult {
        CommandUtils::run_command_str(
            &format!("mkdir -p {}", remote_path),
            Some(remote),
            root_dir,
        )?;

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
    #[allow(dead_code)]
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

    pub fn unescape_ansi(mut s: String) -> String {
        // \u001b → ESC
        s = Regex::new(r#"\\u0*01[bB]"#)
            .unwrap()
            .replace_all(&s, "\x1b")
            .into_owned();

        // \e → ESC (common in some logs)
        s = Regex::new(r#"\\e"#)
            .unwrap()
            .replace_all(&s, "\x1b")
            .into_owned();

        // \033 (octal) → ESC
        s = Regex::new(r#"\\0?33"#)
            .unwrap()
            .replace_all(&s, "\x1b")
            .into_owned();

        // \xNN → corresponding byte
        let re = Regex::new(r#"\\x([0-9a-fA-F]{2})"#).unwrap();
        re.replace_all(&s, |caps: &regex::Captures| {
            let b = u8::from_str_radix(&caps[1], 16).unwrap_or(b'?');
            String::from_utf8_lossy(&[b]).into_owned()
        })
        .into_owned()
    }

    pub fn with_env_source(root_dir: &str, str: &str) -> ResultWithError<String> {
        Ok(format!(
            "source {}/.bashrc > /dev/null 2>&1 || true; {}",
            root_dir, str
        ))
    }
}
