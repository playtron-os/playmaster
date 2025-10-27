use std::{fs, io::Write as _, path::PathBuf, str::FromStr as _};

use tracing::{error, info};

use crate::{
    hooks::iface::HookContext,
    models::app_state::{AppState, RemoteInfo},
    utils::{command::CommandUtils, errors::EmptyResult, os::InstallType},
};

/// Trait that all installer implementations must adhere to.
pub trait Installer {
    #[allow(dead_code)]
    fn get_type(&self) -> InstallType;
    fn run(&self, ctx: &HookContext<'_, AppState>, arg: &str) -> EmptyResult;
}

impl dyn Installer {
    pub fn from_type(install_type: InstallType) -> Box<dyn Installer> {
        match install_type {
            InstallType::File => Box::new(crate::linux::utils::installer::file::FileInstaller),
            InstallType::Package => {
                Box::new(crate::linux::utils::installer::package::PackageInstaller)
            }
        }
    }
}

pub fn add_bin_to_path(bin_path: &str, remote: Option<&RemoteInfo>, root_dir: &str) -> EmptyResult {
    let line = format!("export PATH=\"{}:\\\\$PATH\"", bin_path);
    add_line_to_bashrc(&line, remote, root_dir)
}

pub fn add_line_to_bashrc(line: &str, remote: Option<&RemoteInfo>, root_dir: &str) -> EmptyResult {
    let file_path = PathBuf::from_str(root_dir)?
        .join(".bashrc")
        .to_string_lossy()
        .to_string();

    info!("Adding line '{}' to {}", line, file_path);

    if let Some(remote) = remote {
        // Remote: only append if not already present
        let check_cmd =
            format!("grep -Fxq \"{line}\" {file_path} || echo \"{line}\" >> {file_path}");
        let res = CommandUtils::run_command_str(&check_cmd, Some(remote), root_dir)?;
        if res.status != 0 {
            error!("Failed to add line remotely: {}", res.stderr);
            return Err("Failed to add line remotely".into());
        }
        info!("Ensured line '{}' is defined in remote {}", line, file_path);
    } else {
        // Local: only append if not already present
        let contents = fs::read_to_string(&file_path).unwrap_or_default();
        if !contents.contains(line) {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&file_path)?;
            writeln!(file, "{}", line)?;
            info!("Added line '{}' in local {}", line, file_path);
        } else {
            info!("Line '{}' already exists in {}, skipping", line, file_path);
        }
    }

    Ok(())
}
