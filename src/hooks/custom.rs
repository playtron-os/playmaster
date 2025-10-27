use std::process::Stdio;
use std::{env, thread};
use std::{path::PathBuf, process::Command};

use tracing::{error, info, trace};

use crate::utils::errors::ResultWithError;
use crate::utils::os::OsUtils;
use crate::{
    hooks::iface::{Hook, HookContext, HookType},
    models::{
        app_state::{AppState, RemoteInfo},
        config::HookConfig,
    },
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, ResultTrait},
        file_logger::FileLogger,
    },
};

struct RunCmd {
    file_path: PathBuf,
    command: String,
}

pub struct HookCustom {
    config: HookConfig,
}

impl HookCustom {
    pub fn new(config: HookConfig) -> Self {
        HookCustom { config }
    }

    /// Build command string with args and environment variables
    fn build_cmd(&self) -> ResultWithError<RunCmd> {
        let mut s = String::new();
        let mut has_display = false;
        let file_path = OsUtils::write_temp_script(&self.config.command)
            .auto_err("Failed to write temporary script")?;

        // Inject environment variables
        if let Some(envs) = &self.config.env {
            for (key, value) in envs {
                s.push_str(&format!("{}='{}' ", key, value));

                if key == "DISPLAY" {
                    has_display = true;
                }
            }
        }

        // Ensure DISPLAY is set
        if !has_display {
            s.push_str(
                format!("DISPLAY='{}' ", env::var("DISPLAY").unwrap_or(":0".into())).as_str(),
            );
        }

        // Append command and args (works for multi-line YAML too)
        s.push_str(&file_path.to_string_lossy());

        Ok(RunCmd {
            file_path,
            command: s,
        })
    }

    /// Local synchronous execution
    fn run_local_sync(&self) -> EmptyResult {
        // Build the full command string using your existing helper
        let cmd = self.build_cmd()?;

        // Always run through bash so it can interpret the full string
        let mut command = Command::new("bash");
        command.arg("-c").arg(&cmd.command);
        trace!(
            "[{}] Running sync command: {:?}",
            self.config.name, &command
        );

        let output = command.output().auto_err(&format!(
            "Failed to start custom hook sync: {}",
            self.config.name
        ))?;

        let stdout_logger = FileLogger::new(&format!("{}.stdout.log", self.config.name));
        let stderr_logger = FileLogger::new(&format!("{}.stderr.log", self.config.name));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.is_empty() {
            stdout_logger.log(&stdout);
        }
        if !stderr.is_empty() {
            stderr_logger.log(&stderr);
        }

        if !output.status.success() {
            error!(
                "[{}] Custom hook sync exited with non-zero status: {}",
                self.config.name, output.status
            );
            error!("[{}] Stdout: {}", self.config.name, stdout);
            error!("[{}] Stderr: {}", self.config.name, stderr);
            return Err(format!(
                "Custom hook '{}' failed with exit code {:?}",
                self.config.name,
                output.status.code()
            )
            .into());
        }

        Ok(())
    }

    /// Local asynchronous execution
    fn run_local_async(&self) -> EmptyResult {
        let cmd = self.build_cmd()?;
        let name = self.config.name.clone();

        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(&cmd.command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        trace!("[{name}] Running async command: {:?}", &command);

        match command.spawn() {
            Ok(child) => CommandUtils::track_cmd(&name, child),
            Err(e) => {
                error!("[{name}] Failed to spawn async custom hook: {}", e);
                Err(format!("Failed to spawn async custom hook '{}': {}", name, e).into())
            }
        }
    }

    /// Remote synchronous execution with proper stdout/stderr logging
    fn run_remote_sync(&self, remote: &RemoteInfo) -> EmptyResult {
        let cmd = self.build_cmd()?;

        // Execute remote command and capture stdout/stderr separately
        CommandUtils::copy_file_to_remote(
            remote,
            &cmd.file_path.to_string_lossy(),
            &cmd.file_path,
        )?;
        let output = CommandUtils::run_command_str(&cmd.command, Some(remote))?;

        // Log stdout/stderr to the same files as local execution
        let stdout_logger = FileLogger::new(&format!("{}.stdout.log", self.config.name));
        let stderr_logger = FileLogger::new(&format!("{}.stderr.log", self.config.name));

        if !output.stdout.is_empty() {
            stdout_logger.log(&output.stdout);
        }
        if !output.stderr.is_empty() {
            stderr_logger.log(&output.stderr);
        }

        if output.status != 0 {
            error!(
                "[{}] Remote custom hook exited with non-zero status: {}",
                self.config.name, output.status
            );
            return Err(format!(
                "Remote custom hook '{}' failed with exit code {}",
                self.config.name, output.status
            )
            .into());
        }

        Ok(())
    }

    /// Remote asynchronous execution with logging
    fn run_remote_async(&self, remote: &RemoteInfo) -> EmptyResult {
        let cmd = self.build_cmd()?;

        CommandUtils::copy_file_to_remote(
            remote,
            &cmd.file_path.to_string_lossy(),
            &cmd.file_path,
        )?;

        // Start the remote command
        let name = self.config.name.clone();
        let remote_clone = remote.clone();

        thread::spawn(move || {
            if let Err(err) = CommandUtils::track_remote_cmd(
                cmd.command
                    .split_whitespace()
                    .last()
                    .unwrap_or_default()
                    .to_owned(),
                remote_clone.clone(),
            ) {
                error!("[{name}] Failed to track remote async command: {}", err);
            }

            if let Err(err) = CommandUtils::run_command_str(&cmd.command, Some(&remote_clone)) {
                error!("[{name}] Failed to start remote async command: {}", err);
                return;
            }

            info!("[{name}] Remote async command ended.");
        });

        Ok(())
    }
}

impl Hook for HookCustom {
    fn get_type(&self) -> HookType {
        self.config.hook_type
    }

    fn continue_on_error(&self) -> bool {
        self.config.continue_on_error
    }

    fn run(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        info!("Executing custom hook: {}", self.config.name);

        if let Some(remote) = &ctx.state.read().unwrap().remote {
            if self.config.is_async {
                self.run_remote_async(remote)
            } else {
                self.run_remote_sync(remote)
            }
        } else if self.config.is_async {
            self.run_local_async()
        } else {
            self.run_local_sync()
        }
    }
}
