use std::{
    env, fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use rand::{Rng as _, distr::Alphanumeric};

use crate::{
    hooks::iface::HookContext,
    models::app_state::{AppState, RemoteInfo},
    utils::errors::{EmptyResult, ResultWithError},
};

pub enum InstallType {
    File,
    Package,
}

pub struct OsUtils {}

impl OsUtils {
    pub fn setup_state(ctx: &HookContext<'_, AppState>) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::setup_state(ctx)?;
        }

        Ok(())
    }

    pub fn install(
        install_type: InstallType,
        ctx: &HookContext<'_, AppState>,
        arg: &str,
    ) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::install_package(install_type, ctx, arg)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn set_file_permissions(file_path: &Path) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::set_file_permissions(file_path)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn add_bin(path: &str, remote: Option<&RemoteInfo>, root_dir: &str) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::add_bin(path, remote, root_dir)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn get_display() -> String {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::get_display()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn add_line_to_bashrc(
        line: &str,
        remote: Option<&RemoteInfo>,
        root_dir: &str,
    ) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::add_line_to_bashrc(line, remote, root_dir)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn detect_arch() -> String {
        if let Ok(output) = Command::new("uname").arg("-m").output()
            && let Ok(s) = String::from_utf8(output.stdout)
        {
            return s.trim().to_string();
        }
        "x86_64".to_string()
    }

    pub fn write_temp_script(contents: &str) -> ResultWithError<PathBuf> {
        // Generate a random filename like "hook-ABC123.sh"
        let random: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        let filename = format!("{}.sh", random);

        // Get system temp directory
        let mut path = env::temp_dir();
        path.push(filename);

        // Write the contents
        let mut file = fs::File::create(&path)?;

        writeln!(
            file,
            "#!/usr/bin/env bash\n\
             set -Eeuo pipefail\n\
             # Uncomment for debug tracing\n\
             # set -x\n"
        )?;

        file.write_all(contents.as_bytes())?;
        file.flush()?;

        Self::set_file_permissions(&path)?;

        Ok(path)
    }
}
