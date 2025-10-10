use std::process::Command;

pub struct OsUtils {}

impl OsUtils {
    #[allow(dead_code)]
    pub fn is_package_installed(package: &str) -> bool {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::is_package_installed(package)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub fn get_rpm_install_command(package: &str) -> Command {
        #[cfg(target_os = "linux")]
        {
            crate::linux::utils::os::OsUtils::get_rpm_install_command(package)
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
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
