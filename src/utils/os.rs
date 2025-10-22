use std::{
    env, fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use rand::{Rng as _, distr::Alphanumeric};

use crate::{
    models::app_state::RemoteInfo,
    utils::errors::{EmptyResult, ResultWithError},
};

pub struct OsUtils {}

impl OsUtils {
    #[allow(dead_code)]
    pub fn is_package_installed(
        package: &str,
        remote: Option<&RemoteInfo>,
    ) -> ResultWithError<bool> {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::is_package_installed(package, remote)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub fn install_file(
        file_path: &str,
        sudo_password: &str,
        remote: Option<&RemoteInfo>,
    ) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::install_file(file_path, sudo_password, remote)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn install_package(
        package: &str,
        sudo_password: &str,
        remote: Option<&RemoteInfo>,
    ) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::install_package(package, sudo_password, remote)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn add_bin(path: &str, remote: Option<&RemoteInfo>) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::add_bin(path, remote)
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

    pub fn set_file_permissions(file_path: &Path) -> EmptyResult {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::set_file_permissions(file_path)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(file_path.to_path_buf())
        }
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
