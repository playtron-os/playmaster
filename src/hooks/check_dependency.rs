use tracing::info;

use crate::{
    hooks::iface::{Hook, HookType},
    models::{args::AppArgs, config::Config},
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, ResultTrait as _},
        semver::SemverUtils,
    },
};

/// Hook to check system dependencies as specified in the configuration.
pub struct HookCheckDependency {}

impl HookCheckDependency {
    pub fn new() -> Self {
        HookCheckDependency {}
    }
}

impl Hook for HookCheckDependency {
    fn get_type(&self) -> HookType {
        HookType::VerifySystem
    }

    fn run(&self, _args: &AppArgs, config: &Config) -> EmptyResult {
        info!("Checking dependencies...");

        let mut invalid_dep = false;

        for dep in config.dependencies.iter() {
            let output = CommandUtils::run_command_str(dep.version_command.as_str())
                .auto_err(format!("Failed to execute command: {}", dep.version_command).as_str())?;

            if SemverUtils::is_version_greater_or_equal(&dep.min_version, &output.stdout)? {
                info!(
                    "✅ {} OK ({} ≥ {})",
                    dep.name, &output.stdout, dep.min_version
                );
            } else {
                info!(
                    "❌ {} too old ({} < {})",
                    dep.name, &output.stdout, dep.min_version
                );
                invalid_dep = true;
            }
        }

        if invalid_dep {
            return Err("Some dependencies are not met".into());
        }

        Ok(())
    }
}
