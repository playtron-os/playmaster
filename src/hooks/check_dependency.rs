use std::{fs::File, path::Path};

use reqwest::blocking::get;
use tracing::{error, info};

use crate::{
    hooks::iface::{Hook, HookContext, HookType},
    models::{
        args::{AppArgs, Command},
        config::{Dependency, InstallSource, InstallSpec},
    },
    utils::{
        command::CommandUtils,
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
        source: &InstallSource,
        version: Option<String>,
    ) -> ResultWithError<String> {
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
            crate::models::config::InstallSource::Url { url } => self.download_url(url, version),
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

    fn download_url(&self, url: &str, version: Option<String>) -> ResultWithError<String> {
        let mut url = url.to_owned();
        if let Some(version) = version {
            url = url.replace("{{version}}", &version);
        }

        info!("Downloading tool from URL: {}", url);

        let filename = url.split('/').next_back().ok_or("Invalid URL")?;
        let dest_path = Path::new("/tmp").join(filename);

        let mut response = get(url).map_err(|e| format!("Failed to download URL: {}", e))?;
        let mut file = File::create(&dest_path)?;
        std::io::copy(&mut response, &mut file)?;

        info!("Downloaded artifact to {:?}", dest_path);
        Ok(dest_path.to_string_lossy().to_string())
    }

    fn install_tool(&self, args: &AppArgs, install: &InstallSpec) -> EmptyResult {
        self.prompt_install(args, install)?;

        let install_file = if let Some(source) = &install.source {
            self.download_tool(source, install.version.clone())?
        } else {
            install.tool.clone()
        };

        let mut cmd = OsUtils::get_rpm_install_command(&install_file);
        let output = cmd
            .output()
            .auto_err(format!("Failed to install tool: {}", install_file).as_str())?;

        if !output.status.success() {
            error!(
                "Installation command exited with non-zero status: {}",
                output.status
            );
            if !output.stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Installation stderr: {}", stderr);
            }
            return Err("Tool installation failed".into());
        }

        info!("Tool {} installed successfully", install_file);

        Ok(())
    }

    fn validate_dependency(
        &self,
        args: &AppArgs,
        dep: &Dependency,
        can_install: bool,
    ) -> ResultWithError<bool> {
        let output = CommandUtils::run_command_str(dep.version_command.as_str())
            .auto_err(format!("Failed to execute command: {}", dep.version_command).as_str())?;

        // Validate output is a version string
        if !SemverUtils::is_valid_version(&output.stdout) {
            error!(
                "❌ {} version command did not return a valid version: {}",
                dep.name, &output.stdout
            );

            if can_install && let Some(install_spec) = &dep.install {
                self.install_tool(args, install_spec)?;
                return Ok(true);
            } else {
                error!("No install specification provided for {}", dep.name);
                return Err("Some dependencies are not met".into());
            }
        }

        if SemverUtils::is_version_greater_or_equal(&dep.min_version, &output.stdout)? {
            info!(
                "✅ {} OK ({} ≥ {})",
                dep.name, &output.stdout, dep.min_version
            );
        } else {
            info!(
                "❌ {} too old ({} < {})",
                dep.name, &output.stdout, dep.min_version
            );

            if can_install && let Some(install_spec) = &dep.install {
                self.install_tool(args, install_spec)?;
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
            let was_installed = self.validate_dependency(ctx.args, dep, true)?;

            if was_installed {
                self.validate_dependency(ctx.args, dep, false)?;
            }
        }

        Ok(())
    }
}
