use std::{path::PathBuf, str::FromStr};

use tracing::info;

use crate::{
    hooks::iface::{Hook, HookContext, HookType},
    models::app_state::AppState,
    utils::{command::CommandUtils, dir::DirUtils, errors::EmptyResult, os::OsUtils},
};

/// Hook to establish connection to remote host if needed.
pub struct HookSetupState {}
impl HookSetupState {
    pub fn new() -> Self {
        Self {}
    }
}

impl Hook for HookSetupState {
    fn get_type(&self) -> HookType {
        HookType::Connect
    }

    fn continue_on_error(&self) -> bool {
        false
    }

    fn run(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        info!("Setting up OS-specific state information...");
        OsUtils::setup_state(ctx)?;
        self.set_root_dir(ctx)?;
        self.create_bashrc_if_not_existing(ctx)?;
        self.add_display_to_bashrc(ctx)?;

        Ok(())
    }
}

impl HookSetupState {
    fn set_root_dir(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        let root_dir = {
            let state = ctx.read_state()?;
            DirUtils::root_dir(state.remote.as_ref())?
        };

        let mut state = ctx.write_state()?;
        state.root_dir = root_dir.to_string_lossy().to_string();

        Ok(())
    }

    fn create_bashrc_if_not_existing(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        let state = ctx.read_state().unwrap();
        let remote = state.remote.as_ref();

        let root_dir = ctx.get_root_dir()?;
        let file_path = PathBuf::from_str(&root_dir)?.join(".bashrc");
        CommandUtils::run_command_str(
            &format!(
                "mkdir -p {} && touch {}",
                root_dir,
                file_path.to_string_lossy()
            ),
            remote,
            &root_dir,
        )?;

        Ok(())
    }

    fn add_display_to_bashrc(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        let state = ctx.read_state().unwrap();
        let remote = state.remote.as_ref();

        let root_dir = ctx.get_root_dir()?;
        let display = OsUtils::get_display();
        let line = format!("export DISPLAY={}", display);
        OsUtils::add_line_to_bashrc(&line, remote, &root_dir)?;

        Ok(())
    }
}
