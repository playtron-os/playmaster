use std::{path::PathBuf, str::FromStr};

use tracing::{error, info};

use crate::{
    hooks::iface::HookContext,
    linux::utils::installer::common::Installer,
    models::app_state::{AppState, RemoteInfo},
    utils::{command::CommandUtils, errors::EmptyResult, os::InstallType},
};

pub struct PackageInstaller;

impl Installer for PackageInstaller {
    fn get_type(&self) -> InstallType {
        InstallType::Package
    }

    fn run(&self, ctx: &HookContext<'_, AppState>, package: &str) -> EmptyResult {
        info!("Installing package for linux...");
        let state = ctx.read_state()?;
        let remote = state.remote.as_ref();
        self.install_package(
            package,
            state.os_info.is_ostree,
            &state.sudo_password,
            remote,
            &state.root_dir,
        )?;
        self.add_alias_post_install(package, state.os_info.is_ostree, remote, &state.root_dir)?;
        self.add_local_bin_to_bashrc_path(state.os_info.is_ostree, remote, &state.root_dir)?;
        Ok(())
    }
}

impl PackageInstaller {
    /// Install a package by name using either distrobox or dnf, depending on host type.
    pub fn install_package(
        &self,
        package: &str,
        is_ostree: bool,
        sudo_password: &str,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> EmptyResult {
        let cmd_str = self.get_base_install_cmd(is_ostree, package, sudo_password);
        let conn_type = if remote.is_some() { "remote" } else { "local" };

        info!("Running {} install: {}", conn_type, cmd_str);
        let result = CommandUtils::run_command_str(&cmd_str, remote, root_dir)?;

        if result.status != 0 {
            error!(
                "❌ {} installation of '{}' failed: {}",
                conn_type, package, result.stderr
            );
            return Err(format!("{} installation of '{}' failed", conn_type, package).into());
        }

        info!("✅ {} installation of '{}' succeeded", conn_type, package);
        Ok(())
    }

    fn add_alias_post_install(
        &self,
        cmd: &str,
        is_ostree: bool,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> EmptyResult {
        if !is_ostree {
            return Ok(());
        }

        let cmd = PathBuf::from_str(cmd)?;
        let cmd = cmd.file_name().unwrap_or_default().to_string_lossy();
        let cmd = cmd.split(".").next().unwrap_or_default();
        let full_cmd = format!(
            "distrobox enter --name dev -- distrobox-export --bin /usr/bin/{}",
            cmd
        );

        let conn_type = if remote.is_some() { "remote" } else { "local" };
        let result = CommandUtils::run_command_str(&full_cmd, remote, root_dir)?;

        if result.status != 0 {
            error!(
                "❌ {} export of '{}' failed: {}",
                conn_type, cmd, result.stderr
            );
            return Err(format!("{} export of '{}' failed", conn_type, cmd).into());
        }

        info!("✅ {} export of '{}' succeeded", conn_type, cmd);
        Ok(())
    }

    fn add_local_bin_to_bashrc_path(
        &self,
        is_ostree: bool,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> EmptyResult {
        if !is_ostree {
            return Ok(());
        }

        let full_cmd = "grep -qxF 'export PATH=\"$HOME/.local/bin:$PATH\"' ~/.bashrc || \
                        echo 'export PATH=\"$HOME/.local/bin:$PATH\"' >> ~/.bashrc";

        let conn_type = if remote.is_some() { "remote" } else { "local" };
        let result = CommandUtils::run_command_str(full_cmd, remote, root_dir)?;

        if result.status != 0 {
            error!(
                "❌ {} adding ~/.local/bin to PATH failed: {}",
                conn_type, result.stderr
            );
            return Err(format!("{} adding ~/.local/bin to PATH failed", conn_type).into());
        }

        info!("✅ {} added ~/.local/bin to PATH", conn_type);
        Ok(())
    }

    fn get_base_install_cmd(&self, is_ostree: bool, package: &str, sudo_password: &str) -> String {
        if is_ostree {
            // Immutable host → install inside Distrobox
            format!(
                "distrobox create --name dev --image fedora:41 --yes || true && \
                 distrobox enter --name dev -- sudo dnf install -y {}",
                package
            )
        } else {
            let cmd = format!("dnf install -y {}", package);
            format!("echo '{}' | sudo -S {}", sudo_password, cmd)
        }
    }
}
