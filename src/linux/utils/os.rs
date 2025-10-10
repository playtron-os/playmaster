use std::{
    fs,
    process::{Command, Stdio},
};

pub struct OsUtils;

impl OsUtils {
    pub fn is_fedora_silverblue() -> bool {
        fs::metadata("/run/ostree-booted").is_ok()
    }

    #[allow(dead_code)]
    pub fn is_package_installed(package: &str) -> bool {
        if Self::is_fedora_silverblue() {
            let output = std::process::Command::new("rpm-ostree")
                .args(["pkg-status", package])
                .output();

            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    stdout.contains(package)
                }
                Err(_) => false,
            }
        } else {
            let output = std::process::Command::new("rpm")
                .args(["-q", package])
                .output();

            match output {
                Ok(o) => o.status.success(),
                Err(_) => false,
            }
        }
    }

    pub fn get_rpm_install_command(package: &str) -> Command {
        let is_ostree = OsUtils::is_fedora_silverblue();

        if is_ostree {
            let _ = Command::new("sudo").args(["rpm-ostree", "unlock"]).status();

            // Fedora Silverblue or Kinoite etc.
            let mut c = Command::new("rpm-ostree");
            c.args(["override", "install", package]);
            c
        } else {
            // Regular Fedora/RHEL
            let mut c = Command::new("sudo");
            c.args(["dnf", "install", "-y", package])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            c
        }
    }
}
