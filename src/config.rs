use std::fs;

use serde::Deserialize;

use crate::utils::errors::{ResultTrait, ResultWithError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Flutter,
}

/// Configuration structure for the test controller application.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub project_type: ProjectType,
    pub dependencies: Vec<Dependency>,
}

impl Config {
    pub fn from_curr_dir() -> ResultWithError<Self> {
        let mut cwd = std::env::current_dir().auto_err("Could not read current directory")?;
        if cfg!(debug_assertions) {
            cwd.push("sample_app");
        }

        let config_path = cwd.join("test_controller.yaml");
        let content = fs::read_to_string(config_path).auto_err("Could not read config file")?;
        serde_yaml::from_str(&content).auto_err("Invalid config format")
    }
}

#[derive(Debug, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub min_version: String,
    pub version_command: String,
}
