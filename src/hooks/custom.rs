use std::{os::unix::process::CommandExt as _, process::Command};

use crate::{
    config::{Config, HookConfig},
    hooks::iface::{Hook, HookType},
    utils::errors::{EmptyResult, ResultTrait},
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

    fn run(&self, _config: &Config) -> EmptyResult {
        println!("Executing custom hook: {}", self.config.name);

        if self.config.is_async {
            self.run_async()
        } else {
            self.run_sync()
        }
    }
}
