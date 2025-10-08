use std::{collections::HashMap, fs};

use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    hooks::iface::HookType,
    utils::{
        dir::DirUtils,
        errors::{ResultTrait, ResultWithError},
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
    pub dependencies: Vec<Dependency>,
    pub hooks: Vec<HookConfig>,
}

impl Config {
    pub fn from_curr_dir() -> ResultWithError<Self> {
        let config_path = DirUtils::exec_dir()?.join("test_controller.yaml");
        let content = fs::read_to_string(config_path).auto_err("Could not read config file")?;
        serde_yaml::from_str(&content).auto_err("Invalid config format")
    }
}

#[derive(Debug, Deserialize, Clone, JsonSchema)]
pub struct Dependency {
    pub name: String,
    pub min_version: String,
    pub version_command: String,
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
