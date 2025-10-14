use std::io::{Read as _, Write as _};

use reqwest::blocking::Client;
use tracing::{error, info};

use crate::{
    hooks::iface::{Hook, HookContext, HookType},
    models::{
        app_state::RemoteInfo,
        args::{AppArgs, Command},
        config::{Dependency, InstallSource, InstallSpec},
    },
    utils::{
        command::CommandUtils,
        dir::DirUtils,
        downloader_def::{
            downloader::Downloader, providers::bitbucket::BitbucketSourceProvider,
            r#trait::SourceProvider,
        },
        errors::{EmptyResult, ResultTrait as _, ResultWithError},
        os::OsUtils,
        semver::SemverUtils,
    },
};

/// Hook to check system dependencies as specified in the configuration.
pub struct HookCheckDependency {}

impl HookCheckDependency {
    pub fn new() -> Self {
        HookCheckDependency {}
    }

    fn prompt_install(&self, args: &AppArgs, install: &InstallSpec) -> EmptyResult {
        if let Command::Run { yes, .. } = args.command
            && yes
        {
            info!(
                "Auto-accepting installation of {} due to --yes flag",
                install.tool
            );
            return Ok(());
        }

        let res = inquire::Select::new(
            format!("Do you want to install {} now?", install.tool).as_str(),
            vec!["Yes", "No"],
        )
        .prompt()?;

        match res {
            "Yes" => {
                info!("Proceeding with installation of {}", install.tool);
            }
            "No" => {
                error!(
                    "Cannot proceed without installing required tool: {}",
                    install.tool
                );
                return Err("Dependency installation declined".into());
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn download_tool(
        &self,
        ctx: &HookContext,
        source: &InstallSource,
        version: Option<String>,
    ) -> ResultWithError<String> {
        let state = ctx.read_state()?;
        let remote = state.remote.as_ref();

        match source {
            crate::models::config::InstallSource::Bitbucket { repo, token } => {
                info!("Downloading tool from Bitbucket repo: {}", repo);
                let (org, repo_name) = if let Some((o, r)) = repo.split_once('/') {
                    (o.to_string(), r.to_string())
                } else {
                    return Err("Invalid Bitbucket repo format, expected org/repo".into());
                };
                let provider = BitbucketSourceProvider::new(org, repo_name, Some(token.to_owned()));
                self.download_with_provider(provider, version)
            }
            crate::models::config::InstallSource::Url { url } => {
                self.download_url(url, remote, version)
            }
        }
    }

    fn download_with_provider<P>(
        &self,
        provider: P,
        version: Option<String>,
    ) -> ResultWithError<String>
    where
        P: SourceProvider,
    {
        let downloader = Downloader::new(provider);
        let Some(artifact) = downloader.get_versioned_artifact(version)? else {
            return Err("No suitable artifact found for download".into());
        };

        info!("Found latest artifact: {}", artifact.name);
        let dest_path = std::path::Path::new("/tmp").join(&artifact.name);
        downloader.download(&artifact.name, &dest_path)?;
        info!("Downloaded artifact to {:?}", dest_path);

        Ok(dest_path.to_string_lossy().to_string())
    }

    fn download_url(
        &self,
        url: &str,
        remote: Option<&RemoteInfo>,
        version: Option<String>,
    ) -> ResultWithError<String> {
        let mut url = url.to_string();
        if let Some(version) = version {
            url = url.replace("{{version}}", &version);
        }

        let filename = url.split('/').next_back().ok_or("Invalid URL")?;
        let dest_path = format!("/tmp/{}", filename);

        if let Some(remote) = remote {
            let curl_cmd = format!(
                "stdbuf -oL curl -L --progress-bar -o {} '{}'",
                dest_path, url
            );
            CommandUtils::run_command_str(&curl_cmd, Some(remote))?;
        } else {
            self.download_url_local_with_progress(&url, &dest_path)?;
            info!("Downloaded artifact locally to {}", dest_path);
        }

        Ok(dest_path)
    }

    fn download_url_local_with_progress(&self, url: &str, dest_path: &str) -> EmptyResult {
        let client = Client::new();
        let mut response = client.get(url).send()?;
        let total_size = response.content_length().unwrap_or(0);

        let mut file = std::fs::File::create(dest_path)?;
        let mut downloaded: u64 = 0;
        let mut buffer = [0u8; 8192];

        loop {
            let n = match response.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    error!("Error reading from response: {}", e);
                    break;
                }
            };
            file.write_all(&buffer[..n])?;
            downloaded += n as u64;

            if total_size > 0 {
                let progress = downloaded as f64 / total_size as f64 * 100.0;
                print!(
                    "\rDownloading... {:.2}% ({:.1}/{:.1} MB)",
                    progress,
                    downloaded as f64 / 1_000_000.0,
                    total_size as f64 / 1_000_000.0
                );
                std::io::stdout().flush().unwrap();
            }
        }

        println!("\nDownload complete: {}", dest_path);
        Ok(())
    }

    fn install_tool(&self, ctx: &HookContext, install: &InstallSpec) -> EmptyResult {
        self.prompt_install(ctx.args, install)?;

        let install_file = if let Some(source) = &install.source {
            self.download_tool(ctx, source, install.version.clone())?
        } else {
            install.tool.clone()
        };

        let state = ctx.read_state()?;
        let remote = state.remote.as_ref();
        let password = remote.map(|r| r.password.clone()).unwrap_or_default();
        OsUtils::install_file(&install_file, &password, remote)?;

        if let Some(bin_path) = &install.bin_path {
            let root_dir = DirUtils::root_dir(remote)?;

            let full_path = if bin_path.starts_with('/') || bin_path.starts_with("~") {
                bin_path.to_string()
            } else {
                format!("~/{}", bin_path)
            }
            .replace("~", root_dir.to_string_lossy().to_string().as_ref());

            OsUtils::add_bin(&full_path, remote)?;
        }

        info!("Tool {} installed successfully", install_file);

        Ok(())
    }

    fn validate_dependency(
        &self,
        ctx: &HookContext,
        dep: &Dependency,
        can_install: bool,
    ) -> ResultWithError<bool> {
        let state = ctx.read_state()?;
        let remote = state.remote.as_ref();
        let res = CommandUtils::run_command_str(dep.version_command.as_str(), remote)
            .auto_err(format!("Failed to execute command: {}", dep.version_command).as_str())?;
        let output = res.stdout.trim().to_owned();

        // Validate output is a version string
        if !SemverUtils::is_valid_version(&output) {
            error!(
                "❌ {} version command did not return a valid version: {}",
                dep.name, &output
            );

            if can_install && let Some(install_spec) = &dep.install {
                self.install_tool(ctx, install_spec)?;
                return Ok(true);
            } else {
                error!("No install specification provided for {}", dep.name);
                return Err("Some dependencies are not met".into());
            }
        }

        if SemverUtils::is_version_greater_or_equal(&dep.min_version, &output)? {
            info!("✅ {} OK ({} ≥ {})", dep.name, &output, dep.min_version);
        } else {
            info!(
                "❌ {} too old ({} < {})",
                dep.name, &output, dep.min_version
            );

            if can_install && let Some(install_spec) = &dep.install {
                self.install_tool(ctx, install_spec)?;
                return Ok(true);
            } else {
                error!("No install specification provided for {}", dep.name);
                return Err("Some dependencies are not met".into());
            }
        }

        Ok(false)
    }
}

impl Hook for HookCheckDependency {
    fn get_type(&self) -> HookType {
        HookType::VerifySystem
    }

    fn run(&self, ctx: &HookContext) -> EmptyResult {
        info!("Checking dependencies...");

        for dep in ctx.config.dependencies.iter() {
            let was_installed = self.validate_dependency(ctx, dep, true)?;

            if was_installed {
                self.validate_dependency(ctx, dep, false)?;
            }
        }

        Ok(())
    }
}
