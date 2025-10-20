use std::{collections::HashMap, fs};

use schemars::JsonSchema;
use serde::Deserialize;

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
    pub hooks: Vec<HookConfig>,
}

impl Config {
    pub fn from_curr_dir() -> ResultWithError<Self> {
        let config_path = DirUtils::curr_dir()?.join("playmaster.yaml");
        let content = fs::read_to_string(config_path).auto_err("Could not read config file")?;
        let expanded = VariablesUtils::expand_env_vars(&content);
        let mut config: Config =
            serde_yaml::from_str(&expanded).auto_err("Invalid config format")?;
        config.load_default_configs();
        Ok(config)
    }

    fn load_default_configs(&mut self) {
        if self.project_type == ProjectType::Flutter {
            self.add_flutter_defaults();
        }
    }
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
    pub is_async: bool,
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}
