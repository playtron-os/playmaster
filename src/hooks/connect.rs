use tracing::info;

use crate::{
    hooks::iface::{Hook, HookType},
    models::{args::AppArgs, config::Config},
    utils::errors::{EmptyResult, ResultWithError},
};

/// Hook to establish connection to remote host if needed.
pub struct HookConnect {}

impl HookConnect {
    pub fn new() -> Self {
        HookConnect {}
    }

    fn prompt_for_remote_conn(&self) -> ResultWithError<bool> {
        let res = inquire::Select::new(
            "Do you want to connect to a remote host?",
            vec!["Yes", "No"],
        )
        .prompt()?;

        if res == "Yes" {
            return Ok(true);
        }

        info!("Proceeding with local connection.");
        Ok(false)
    }
}

impl Hook for HookConnect {
    fn get_type(&self) -> HookType {
        HookType::Connect
    }

    fn run(&self, args: &AppArgs, _config: &Config) -> EmptyResult {
        match args.mode.as_ref() {
            Some(mode) => {
                info!("Connection mode specified via command line: {:?}", mode);
                return Ok(());
            }
            None => {}
        }

        if !self.prompt_for_remote_conn()? {
            return Ok(());
        }

        // TODO: Remote connection logic here
        info!(
            "Remote connection feature is not implemented yet, so we will proceed with local connection."
        );

        Ok(())
    }
}
