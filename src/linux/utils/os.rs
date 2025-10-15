use std::{
    fs,
    io::Write as _,
    process::{Command, Stdio},
};

use tracing::{error, info};

use crate::{
    models::app_state::RemoteInfo,
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, OptionResultTrait, ResultTrait as _, ResultWithError},
    },
};

pub struct OsUtils;

impl OsUtils {
    pub fn is_fedora_silverblue(remote: Option<&RemoteInfo>) -> ResultWithError<bool> {
        if let Some(remote) = remote {
            // Remote check: see if /run/ostree-booted exists
            let cmd = "test -e /run/ostree-booted && echo true || echo false";
            let result = CommandUtils::run_command_str(cmd, Some(remote))?;
            Ok(result.stdout.trim() == "true")
        } else {
            // Local check
            Ok(fs::metadata("/run/ostree-booted").is_ok())
        }
    }

    #[allow(dead_code)]
    pub fn is_package_installed(
        package: &str,
        remote: Option<&RemoteInfo>,
    ) -> ResultWithError<bool> {
        let is_ostree = Self::is_fedora_silverblue(remote)?;

        let cmd = if is_ostree {
            format!("rpm-ostree pkg-status {}", package)
        } else {
            format!("rpm -q {}", package)
        };

        let result = CommandUtils::run_command_str(&cmd, remote)?;

        if is_ostree {
            Ok(result.stdout.contains(package))
        } else {
            Ok(result.stdout.trim() != "")
        }
    }

    pub fn add_bin(path: &str, remote: Option<&RemoteInfo>) -> EmptyResult {
        let export_cmd = format!("export PATH=\"{}:$PATH\"", path);

        if let Some(remote) = remote {
            // Remote: only append if not already present
            let check_cmd = format!(
                "grep -Fxq '{}' ~/.bashrc || echo '{}' >> ~/.bashrc",
                export_cmd, export_cmd
            );
            let source_cmd = format!("{} && source ~/.bashrc", check_cmd);
            let res = CommandUtils::run_command_str(&source_cmd, Some(remote))?;
            if res.status != 0 {
                error!("Failed to add bin path remotely: {}", res.stderr);
                return Err("Failed to add bin path remotely".into());
            }
            info!(
                "Ensured '{}' is in PATH in remote ~/.bashrc and refreshed session",
                path
            );
        } else {
            // Local: only append if not already present
            let bashrc_path = std::path::Path::new(&std::env::var("HOME")?).join(".bashrc");
            let contents = std::fs::read_to_string(&bashrc_path).unwrap_or_default();
            if !contents.contains(&export_cmd) {
                let mut file = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&bashrc_path)?;
                writeln!(file, "{}", export_cmd)?;
                info!("Added '{}' to PATH in local ~/.bashrc", path);
            } else {
                info!("PATH already contains '{}', skipping", path);
            }
        }

        Ok(())
    }

    pub fn install_file(
        file_path: &str,
        sudo_password: &str,
        remote: Option<&RemoteInfo>,
    ) -> EmptyResult {
        let lower = file_path.to_lowercase();
        if lower.ends_with(".rpm") {
            Self::install_rpm(file_path, sudo_password, remote)
        } else if lower.ends_with(".tar.xz")
            || lower.ends_with(".tar.gz")
            || lower.ends_with(".zip")
        {
            Self::install_archive(file_path, remote)
        } else {
            Err(format!("Unknown file type for installation: '{}'", file_path).into())
        }
    }

    /// Install RPM package, local or remote
    fn install_rpm(
        file_path: &str,
        sudo_password: &str,
        remote: Option<&RemoteInfo>,
    ) -> EmptyResult {
        let is_ostree = OsUtils::is_fedora_silverblue(remote)?;

        if let Some(remote) = remote {
            Self::run_remote_install(file_path, remote, is_ostree, sudo_password)
        } else {
            Self::run_local_install(file_path, is_ostree, sudo_password)
        }
    }

    /// Install an archive (tar.xz, tar.gz, zip)
    fn install_archive(file_path: &str, remote: Option<&RemoteInfo>) -> EmptyResult {
        if let Some(remote) = remote {
            Self::extract_archive_remote(file_path, remote)
        } else {
            Self::extract_archive_local(file_path)
        }
    }

    /// Local archive extraction
    fn extract_archive_local(file_path: &str) -> EmptyResult {
        let install_dir = "~/playmaster";
        std::fs::create_dir_all(install_dir)?;

        // Build extraction command with progress
        let mut cmd = if file_path.ends_with(".tar.xz") {
            let mut c = Command::new("tar");
            c.args(["-xJvf", file_path, "-C", install_dir]);
            c
        } else if file_path.ends_with(".tar.gz") {
            let mut c = Command::new("tar");
            c.args(["-xvzf", file_path, "-C", install_dir]);
            c
        } else if file_path.ends_with(".zip") {
            let mut c = Command::new("unzip");
            c.args(["-v", "-o", file_path, "-d", install_dir]);
            c
        } else {
            return Err(format!("Unsupported archive type: {}", file_path).into());
        };

        info!("Extracting '{}' to '{}'", file_path, install_dir);

        // Stream stdout/stderr to console
        cmd.stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd
            .status()
            .auto_err(format!("Failed to extract archive: {}", file_path).as_ref())?;

        if !status.success() {
            return Err(format!("Extraction of '{}' failed", file_path).into());
        }

        info!("✅ Local extraction of '{}' succeeded", file_path);
        Ok(())
    }

    /// Remote archive extraction
    fn extract_archive_remote(file_path: &str, remote: &RemoteInfo) -> EmptyResult {
        // Install directory in user's home
        let install_dir = "~/playmaster";

        // Determine extraction command with progress
        let extract_cmd = if file_path.ends_with(".tar.xz") {
            format!("mkdir -p {install_dir} && tar -xJvf {file_path} -C {install_dir}")
        } else if file_path.ends_with(".tar.gz") {
            format!("mkdir -p {install_dir} && tar -xvzf {file_path} -C {install_dir}")
        } else if file_path.ends_with(".zip") {
            format!("mkdir -p {install_dir} && unzip -v -o {file_path} -d {install_dir}")
        } else {
            return Err(format!("Unsupported archive type: {}", file_path).into());
        };

        info!("Executing remote extraction: {}", extract_cmd);

        // Run command on remote without sudo (assuming home dir is writable)
        let res = CommandUtils::run_command_str(&extract_cmd, Some(remote))?;

        if res.status != 0 {
            error!(
                "❌ Remote extraction of '{}' failed: {}",
                file_path, res.stderr
            );
            return Err(format!("Remote extraction of '{}' failed", file_path).into());
        }

        info!("✅ Remote extraction of '{}' succeeded", file_path);
        Ok(())
    }

    fn run_local_install(package: &str, is_ostree: bool, sudo_password: &str) -> EmptyResult {
        if is_ostree {
            Self::run_local_rpm_ostree_install(package)
        } else {
            Self::run_local_dnf_install(package, sudo_password)
        }
    }

    fn run_local_rpm_ostree_install(package: &str) -> EmptyResult {
        info!("Detected Fedora Silverblue/Kinoite — using rpm-ostree");

        // Unlock for temporary write
        let _ = Command::new("sudo").args(["rpm-ostree", "unlock"]).status();

        // Perform installation
        let status = Command::new("rpm-ostree")
            .args(["install", package])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .auto_err("Failed to run rpm-ostree")?;

        if !status.success() {
            return Err(format!("rpm-ostree install of '{}' failed", package).into());
        }

        info!("✅ rpm-ostree install of '{}' succeeded", package);
        Ok(())
    }

    fn run_local_dnf_install(package: &str, sudo_password: &str) -> EmptyResult {
        info!("Running local DNF install for '{}'", package);

        let mut cmd = Command::new("sudo");
        cmd.args(["-S", "dnf", "install", "-y", package])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let mut child = cmd
            .stdin(Stdio::piped())
            .spawn()
            .auto_err("Failed to spawn sudo")?;
        child
            .stdin
            .as_mut()
            .auto_err("Failed to open stdin")?
            .write_all(format!("{}\n", sudo_password).as_bytes())?;
        let status = child.wait().auto_err("Failed to wait for sudo child")?;
        if !status.success() {
            return Err(format!("DNF install of '{}' failed", package).into());
        }

        info!("✅ DNF install of '{}' completed successfully", package);
        Ok(())
    }

    fn run_remote_install(
        package: &str,
        remote: &RemoteInfo,
        is_ostree: bool,
        sudo_password: &str,
    ) -> EmptyResult {
        let base_cmd = if is_ostree {
            format!(
                "(rpm-ostree unlock || true) && rpm-ostree install {}",
                package
            )
        } else {
            format!("dnf install -y {}", package)
        };

        let cmd_str = format!("echo '{}' | sudo -S {}", sudo_password, base_cmd);

        info!("Running remote install: {}", base_cmd);
        let result = CommandUtils::run_command_str(&cmd_str, Some(remote))?;

        if result.status != 0 {
            error!(
                "❌ Remote installation of '{}' failed: {}",
                package, result.stderr
            );
            return Err(format!("Remote installation of '{}' failed", package).into());
        }

        info!("✅ Remote installation of '{}' succeeded", package);
        Ok(())
    }

    /// Install a package by name using either rpm-ostree or dnf, depending on host type.
    pub fn install_package(
        package: &str,
        sudo_password: &str,
        remote: Option<&RemoteInfo>,
    ) -> EmptyResult {
        let is_ostree = OsUtils::is_fedora_silverblue(remote)?;

        if let Some(remote) = remote {
            // Remote install
            let base_cmd = if is_ostree {
                // rpm-ostree requires unlock and override install
                format!("rpm-ostree install {} --apply-live", package)
            } else {
                format!("dnf install -y {}", package)
            };

            let cmd_str = format!("echo '{}' | sudo -S {}", sudo_password, base_cmd);
            info!(
                "Installing '{}' remotely with {}",
                package,
                if is_ostree { "rpm-ostree" } else { "dnf" }
            );

            let result = CommandUtils::run_command_str(&cmd_str, Some(remote))?;

            if result.status != 0 {
                error!(
                    "❌ Remote install of '{}' failed: {}",
                    package, result.stderr
                );
                return Err(format!("Remote installation of '{}' failed", package).into());
            }

            info!("✅ Remote installation of '{}' succeeded", package);
        } else {
            // Local install
            if is_ostree {
                info!("Detected Fedora Silverblue/Kinoite — using rpm-ostree");

                // Unlock just in case and apply live if possible
                let _ = Command::new("sudo").args(["rpm-ostree", "unlock"]).status();

                let status = Command::new("sudo")
                    .args([
                        "-S",
                        "rpm-ostree",
                        "override",
                        "install",
                        package,
                        "--apply-live",
                        "--allow-replacement",
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .and_then(|mut child| {
                        child.stdin.as_mut().map(|stdin| {
                            stdin.write_all(format!("{}\n", sudo_password).as_bytes())
                        });
                        child.wait()
                    })
                    .auto_err("Failed to execute rpm-ostree install")?;

                if !status.success() {
                    return Err(format!("rpm-ostree install of '{}' failed", package).into());
                }
                info!("✅ rpm-ostree install of '{}' succeeded", package);
            } else {
                info!("Running local DNF install for '{}'", package);

                let mut cmd = Command::new("sudo");
                cmd.args(["-S", "dnf", "install", "-y", package])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit());

                let mut child = cmd
                    .stdin(Stdio::piped())
                    .spawn()
                    .auto_err("Failed to spawn sudo")?;
                child
                    .stdin
                    .as_mut()
                    .auto_err("Failed to open stdin")?
                    .write_all(format!("{}\n", sudo_password).as_bytes())?;
                let status = child.wait().auto_err("Failed to wait for sudo child")?;
                if !status.success() {
                    return Err(format!("DNF install of '{}' failed", package).into());
                }

                info!("✅ DNF install of '{}' completed successfully", package);
            }
        }

        Ok(())
    }
}
