use std::{
    env, fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::Command,
};

use rand::{Rng as _, distr::Alphanumeric};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{debug, trace};

use crate::{
    hooks::iface::HookContext,
    models::app_state::{AppState, RemoteInfo},
    utils::errors::{EmptyResult, ResultTrait, ResultWithError},
};

#[derive(Debug)]
pub enum InstallType {
    File,
    Package,
}

pub struct OsUtils {}

impl OsUtils {
    pub fn setup_state(ctx: &HookContext<'_, AppState>) -> EmptyResult {
        debug!("Setting up OS state");

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
        debug!("Installing {:?} with argument: {}", install_type, arg);

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
        debug!("Setting file permissions for: {:?}", file_path);

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
        debug!(
            "Adding binary '{}' to PATH (remote: {}, root_dir: {})",
            path,
            remote.is_some(),
            root_dir
        );

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
            let display_val = crate::linux::utils::os::OsUtils::get_display();
            debug!("Detected display: {}", display_val);
            display_val
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
        debug!(
            "Adding line to bashrc: '{}' (remote: {}, root_dir: {})",
            line,
            remote.is_some(),
            root_dir
        );

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
            debug!("Detected architecture: {}", s.trim());
            return s.trim().to_string();
        }

        debug!("Defaulting architecture to x86_64");
        "x86_64".to_string()
    }

    pub fn write_temp_script(contents: &str) -> ResultWithError<PathBuf> {
        debug!("Writing temporary script");
        trace!("Script contents:\n{}", contents);

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

    pub fn ask(prompt: &str) -> ResultWithError<String> {
        print!("{}", prompt);
        io::stdout()
            .flush()
            .auto_err("Error flushing stdout during ask")?;

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let _ = tx.send(input.trim().to_string());
            }
        });

        let res = match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(input) => input,
            Err(_) => {
                println!("\nTimeout: no input received within 30 seconds");
                String::new()
            }
        };

        Ok(res)
    }
}
