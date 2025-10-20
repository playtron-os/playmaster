use std::process::Command;
use std::thread;

use tracing::{error, info};
use uuid::Uuid;

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

pub struct HookCustom {
    config: HookConfig,
}

impl HookCustom {
    pub fn new(config: HookConfig) -> Self {
        HookCustom { config }
    }

    /// Build command string with args and environment variables
    fn build_cmd_string(&self) -> String {
        let mut s = String::new();

        // Inject environment variables
        if let Some(envs) = &self.config.env {
            for (key, value) in envs {
                s.push_str(&format!("{}='{}' ", key, value));
            }
        }

        // Add command and args
        s.push_str(&self.config.command);
        if let Some(args) = &self.config.args {
            s.push(' ');
            s.push_str(&args.join(" "));
        }

        s
    }

    /// Local synchronous execution
    fn run_local_sync(&self) -> EmptyResult {
        let mut cmd = Command::new(&self.config.command);
        if let Some(args) = &self.config.args {
            cmd.args(args);
        }
        if let Some(env) = &self.config.env {
            cmd.envs(env);
        }

        let output = cmd
            .output()
            .auto_err(format!("Failed to start custom hook sync: {}", self.config.name).as_str())?;

        let stdout_logger = FileLogger::new(&format!("{}.stdout.log", self.config.name));
        let stderr_logger = FileLogger::new(&format!("{}.stderr.log", self.config.name));

        if !output.stdout.is_empty() {
            stdout_logger.log(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            stderr_logger.log(&String::from_utf8_lossy(&output.stderr));
        }
        if !output.status.success() {
            error!(
                "[{}] Custom hook sync exited with non-zero status: {}",
                self.config.name, output.status
            );
        }

        Ok(())
    }

    /// Local asynchronous execution
    fn run_local_async(&self) -> EmptyResult {
        let cmd_str = self.build_cmd_string();
        let name = self.config.name.clone();

        thread::spawn(move || {
            let output = Command::new("sh").arg("-c").arg(&cmd_str).output();

            let stdout_logger = FileLogger::new(&format!("{name}.stdout.log"));
            let stderr_logger = FileLogger::new(&format!("{name}.stderr.log"));

            match output {
                Ok(output) => {
                    if !output.status.success() {
                        error!(
                            "[{name}] Custom hook async exited with non-zero status: {}",
                            output.status
                        );
                    }
                    if !output.stdout.is_empty() {
                        stdout_logger.log(&String::from_utf8_lossy(&output.stdout));
                    }
                    if !output.stderr.is_empty() {
                        stderr_logger.log(&String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    error!("[{name}] Failed to run async custom hook: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Remote synchronous execution with proper stdout/stderr logging
    fn run_remote_sync(&self, remote: &RemoteInfo) -> EmptyResult {
        let cmd_str = self.build_cmd_string();

        // Execute remote command and capture stdout/stderr separately
        let output = CommandUtils::run_command_str(&cmd_str, Some(remote))?;

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
        }

        Ok(())
    }

    /// Remote asynchronous execution with logging
    fn run_remote_async(&self, remote: &RemoteInfo) -> EmptyResult {
        let cmd_str = self.build_cmd_string();

        // Generate temporary remote log files
        let stdout_remote = format!("/tmp/{}_stdout.log", Uuid::new_v4());
        let stderr_remote = format!("/tmp/{}_stderr.log", Uuid::new_v4());
        let pid_remote = format!("/tmp/{}_pid", Uuid::new_v4());

        // Start the remote command and store its PID
        let async_cmd = format!(
            "nohup {} > {} 2> {} & echo $! > {}",
            cmd_str, stdout_remote, stderr_remote, pid_remote
        );
        CommandUtils::run_command_str(&async_cmd, Some(remote))?;

        let name = self.config.name.clone();
        let remote_clone = remote.clone();
        let stdout_remote_clone = stdout_remote.clone();
        let stderr_remote_clone = stderr_remote.clone();
        let pid_remote_clone = pid_remote.clone();

        thread::spawn(move || {
            let stdout_logger = FileLogger::new(&format!("{name}.stdout.log"));
            let stderr_logger = FileLogger::new(&format!("{name}.stderr.log"));

            loop {
                // Fetch logs
                let stdout = CommandUtils::fetch_remote_file(&remote_clone, &stdout_remote_clone)
                    .unwrap_or_default();
                let stderr = CommandUtils::fetch_remote_file(&remote_clone, &stderr_remote_clone)
                    .unwrap_or_default();

                if !stdout.is_empty() {
                    stdout_logger.log(&stdout);
                }
                if !stderr.is_empty() {
                    stderr_logger.log(&stderr);
                }

                // Check if remote process is still running
                let pid_str = CommandUtils::fetch_remote_file(&remote_clone, &pid_remote_clone)
                    .unwrap_or_default();
                let pid = pid_str.trim();
                if pid.is_empty() {
                    // PID not available yet, continue polling
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    continue;
                }

                // Check process with kill -0
                let check_cmd =
                    format!("kill -0 {} 2>/dev/null && echo running || echo exited", pid);
                let res = CommandUtils::run_command_str(&check_cmd, Some(&remote_clone))
                    .unwrap_or_default();
                if res.stdout.trim() == "exited" {
                    break; // remote process finished, exit loop
                }

                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });

        Ok(())
    }
}

impl Hook for HookCustom {
    fn get_type(&self) -> HookType {
        self.config.hook_type
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
