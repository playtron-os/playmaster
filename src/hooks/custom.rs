use std::process::Command;

use tracing::{error, info};

use crate::{
    hooks::iface::{Hook, HookType},
    models::{
        args::AppArgs,
        config::{Config, HookConfig},
    },
    utils::{
        errors::{EmptyResult, ResultTrait},
        file_logger::FileLogger,
    },
};

/// Custom hook that gets implementation based on yaml config.
pub struct HookCustom {
    config: HookConfig,
}

impl HookCustom {
    pub fn new(config: HookConfig) -> Self {
        HookCustom { config }
    }

    fn get_command(&self) -> Command {
        let mut cmd = Command::new(&self.config.command);
        if let Some(args) = &self.config.args {
            cmd.args(args);
        }
        if let Some(env) = &self.config.env {
            cmd.envs(env);
        }

        cmd
    }

    fn run_async(&self) -> EmptyResult {
        let mut cmd = self.get_command();
        let child = cmd.spawn().auto_err(
            format!("Failed to start custom hook async: {}", self.config.name).as_str(),
        )?;
        let name = self.config.name.clone();

        std::thread::spawn(move || {
            let output = child.wait_with_output();
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
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        stdout_logger.log(stdout.as_ref());
                    }
                    if !output.stderr.is_empty() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        stderr_logger.log(stderr.as_ref());
                    }
                }
                Err(e) => {
                    error!("[{name}] Failed to wait for custom hook async: {}", e);
                }
            }
        });

        Ok(())
    }

    fn run_sync(&self) -> EmptyResult {
        let mut cmd = self.get_command();
        cmd.output()
            .auto_err(format!("Failed to start custom hook sync: {}", self.config.name).as_str())?;
        Ok(())
    }
}

impl Hook for HookCustom {
    fn get_type(&self) -> HookType {
        self.config.hook_type
    }

    fn run(&self, _args: &AppArgs, _config: &Config) -> EmptyResult {
        info!("Executing custom hook: {}", self.config.name);

        if self.config.is_async {
            self.run_async()
        } else {
            self.run_sync()
        }
    }
}
