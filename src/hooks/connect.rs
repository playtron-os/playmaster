use inquire::Password;
use tracing::info;

use crate::{
    hooks::iface::{Hook, HookContext, HookType},
    models::{self, args::AppMode},
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

    fn run(&self, ctx: &HookContext) -> EmptyResult {
        if let models::args::Command::Run {
            mode: Some(mode), ..
        } = &ctx.args.command
        {
            info!("Connection mode specified via command line: {:?}", mode);

            if *mode == AppMode::Local {
                return Ok(());
            }
        } else if !self.prompt_for_remote_conn()? {
            return Ok(());
        }

        let _password = Password::new("Enter your remote device's password:")
            .without_confirmation()
            .prompt()?;

        Ok(())
    }
}
