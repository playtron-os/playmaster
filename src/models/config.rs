use std::{collections::HashMap, fs};

use schemars::JsonSchema;
use serde::Deserialize;
use tracing::debug;

use crate::{
    hooks::iface::HookType,
    utils::{
        dir::DirUtils,
        errors::{ResultTrait, ResultWithError},
        variables::VariablesUtils,
    },
};

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Flutter,
}

/// Configuration structure for the test controller application.
#[derive(Debug, Deserialize, Clone, JsonSchema)]
pub struct Config {
    pub project_type: ProjectType,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub state_set: StateSet,
    #[serde(default)]
    pub hooks: Vec<HookConfig>,
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,
    #[serde(default)]
    pub gmail: GmailConfig,
}

impl Config {
    pub fn from_curr_dir() -> ResultWithError<Self> {
        debug!("Loading configuration from current directory...");

        let config_path = DirUtils::curr_dir()?.join("playmaster.yaml");
        debug!("Loading config from {:?}", config_path);

        let content = fs::read_to_string(config_path).auto_err("Could not read config file")?;
        debug!("Config loaded");

        let expanded = VariablesUtils::expand_env_vars(&content);
        debug!("Config expanded");

        let mut config: Config =
            serde_yaml::from_str(&expanded).auto_err("Invalid config format")?;
        debug!("Config deserialized");

        config.load_default_configs();
        debug!("Config default values loaded");

        Ok(config)
    }

    fn load_default_configs(&mut self) {
        if self.project_type == ProjectType::Flutter {
            self.add_flutter_defaults();
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookType {
    #[serde(alias = "Results", alias = "RESULTS")]
    Results,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
pub struct WebhookConfig {
    pub webhook_type: WebhookType,
    pub url: String,
    #[serde(default)]
    pub message_template: String,
    #[serde(default)]
    pub ignore_error: bool,
    #[serde(default)]
    pub s3_config: Option<S3Config>,
}

#[derive(Debug, Deserialize, Clone, JsonSchema, Default)]
pub struct S3Config {
    #[serde(default)]
    pub key_prefix: String,
    pub bucket: String,
}

#[derive(Debug, Deserialize, Clone, JsonSchema, Default)]
pub struct StateSet {
    pub command: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
pub struct Dependency {
    pub name: String,
    pub min_version: String,
    pub version_command: String,
    pub install: Option<InstallSpec>,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct InstallSpec {
    /// The tool name (used to find in Bitbucket/GitHub, etc.)
    pub tool: String,

    /// Optional version, e.g. "1.2.3"
    pub version: Option<String>,

    /// Optional binary path inside the archive, e.g. "flutter/bin"
    pub bin_path: Option<String>,

    /// Optional setup command to run after installation
    pub setup: Option<String>,

    /// Source information, if left empty will default to DNF install
    pub source: Option<InstallSource>,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallSource {
    Bitbucket { repo: String, token: String },
    Url { url: String },
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
pub struct HookConfig {
    pub name: String,
    pub hook_type: HookType,
    #[serde(rename = "async")]
    #[serde(default)]
    pub is_async: bool,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default)]
    pub r#if: Option<String>,
    pub command: String,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Default, Deserialize, Clone, JsonSchema)]
pub struct GmailConfig {
    pub enabled: bool,
    pub credentials: GmailCredentialsConfig,
}

#[derive(Debug, Default, Deserialize, Clone, JsonSchema)]
pub struct GmailCredentialsConfig {
    pub s3: Option<S3Config>,
    /// IMAP configuration for Gmail access using App Password
    pub imap: Option<ImapConfig>,
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
pub struct ImapConfig {
    /// Gmail address (e.g., "your@gmail.com")
    pub email: String,
    /// Google App Password (not your regular password)
    /// Can use environment variable syntax like ${GMAIL_APP_PASSWORD}
    pub app_password: String,
}
