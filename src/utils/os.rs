use std::process::Command;

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
}
