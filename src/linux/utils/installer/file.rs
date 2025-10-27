use tracing::{error, info};

use crate::{
    hooks::iface::HookContext,
    linux::utils::installer::{common::Installer, package::PackageInstaller},
    models::app_state::{AppState, RemoteInfo},
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, ResultWithError},
        os::InstallType,
    },
};

pub struct FileInstaller;

impl Installer for FileInstaller {
    fn get_type(&self) -> InstallType {
        InstallType::File
    }

    fn run(&self, ctx: &HookContext<'_, AppState>, file_path: &str) -> EmptyResult {
        // Proxy to package installer if it is an RPM file
        if file_path.ends_with(".rpm") {
            let package_installer = PackageInstaller;
            package_installer.run(ctx, file_path)?;
            return Ok(());
        }

        info!("Installing file for linux...");
        let state = ctx.read_state()?;
        let remote = state.remote.as_ref();
        self.install_file(file_path, remote, &state.root_dir)?;
        Ok(())
    }
}

impl FileInstaller {
    /// Install a file using either distrobox or dnf, depending on host type.
    pub fn install_file(
        &self,
        file_path: &str,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> EmptyResult {
        let lower = file_path.to_lowercase();

        if lower.ends_with(".tar.xz") || lower.ends_with(".tar.gz") || lower.ends_with(".zip") {
            self.install_archive(file_path, remote, root_dir)
        } else {
            Err(format!("Unknown file type for installation: '{}'", file_path).into())
        }
    }

    /// Install an archive (tar.xz, tar.gz, zip)
    fn install_archive(
        &self,
        file_path: &str,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> EmptyResult {
        let extract_cmd = self.get_base_install_cmd(file_path, root_dir)?;
        let conn_type = if remote.is_some() { "remote" } else { "local" };
        info!("Executing {} extraction: {}", conn_type, extract_cmd);

        let res = CommandUtils::run_command_str(&extract_cmd, remote, root_dir)?;

        if res.status != 0 {
            error!(
                "❌ {} extraction of '{}' failed\nstdout: {}\nstderr: {}",
                conn_type, file_path, res.stdout, res.stderr
            );
            return Err(format!("{} extraction of '{}' failed", conn_type, file_path).into());
        }

        info!("✅ {} extraction of '{}' succeeded", conn_type, file_path);
        Ok(())
    }

    fn get_base_install_cmd(&self, file_path: &str, root_dir: &str) -> ResultWithError<String> {
        // Determine extraction command with progress
        if file_path.ends_with(".tar.xz") {
            Ok(format!(
                "mkdir -p {root_dir} && tar -xJvf {file_path} -C {root_dir}"
            ))
        } else if file_path.ends_with(".tar.gz") {
            Ok(format!(
                "mkdir -p {root_dir} && tar -xvzf {file_path} -C {root_dir}"
            ))
        } else if file_path.ends_with(".zip") {
            Ok(format!(
                "mkdir -p {root_dir} && unzip -v -o {file_path} -d {root_dir}"
            ))
        } else {
            Err(format!("Unsupported archive type: {}", file_path).into())
        }
    }
}
