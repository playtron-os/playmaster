use std::{env, fs, path::Path};

use crate::{
    hooks::iface::HookContext,
    linux::utils::installer::common::{Installer, add_bin_to_path, add_line_to_bashrc},
    models::app_state::{AppState, RemoteInfo},
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, ResultWithError},
        os::InstallType,
    },
};

pub struct OsUtils;

impl OsUtils {
    pub fn setup_state(ctx: &HookContext<'_, AppState>) -> EmptyResult {
        let is_ostree = {
            let state = ctx.read_state()?;
            let remote = state.remote.as_ref();
            crate::linux::utils::os::OsUtils::is_fedora_silverblue(remote, &state.root_dir)
                .unwrap_or(false)
        };

        let mut state = ctx.write_state()?;
        state.os_info.is_ostree = state.os_info.is_ostree || is_ostree;

        Ok(())
    }

    pub fn is_fedora_silverblue(
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> ResultWithError<bool> {
        if let Some(remote) = remote {
            // Remote check: see if /run/ostree-booted exists
            let cmd = "test -e /run/ostree-booted && echo true || echo false";
            let result = CommandUtils::run_command_str(cmd, Some(remote), root_dir)?;
            Ok(result.stdout.trim() == "true")
        } else {
            // Local check
            Ok(fs::metadata("/run/ostree-booted").is_ok())
        }
    }

    pub fn set_file_permissions(file_path: &Path) -> EmptyResult {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(file_path, fs::Permissions::from_mode(0o755))?;
        Ok(())
    }

    pub fn install_package(
        install_type: InstallType,
        ctx: &HookContext<'_, AppState>,
        package: &str,
    ) -> EmptyResult {
        let installer = <dyn Installer>::from_type(install_type);
        installer.run(ctx, package)
    }

    pub fn add_bin(path: &str, remote: Option<&RemoteInfo>, root_dir: &str) -> EmptyResult {
        add_bin_to_path(path, remote, root_dir)
    }

    pub fn add_line_to_bashrc(
        line: &str,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> EmptyResult {
        add_line_to_bashrc(line, remote, root_dir)
    }

    pub fn get_display() -> String {
        env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string())
    }
}
