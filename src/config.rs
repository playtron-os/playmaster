use clap::{Parser, ValueEnum};
use std::{collections::HashMap, fs};

use serde::Deserialize;

use crate::{
    hooks::iface::HookType,
    utils::errors::{ResultTrait, ResultWithError},
};

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
    pub hooks: Vec<HookConfig>,
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

#[derive(Debug, Deserialize, Clone)]
pub struct HookConfig {
    pub name: String,
    pub hook_type: HookType,
    #[serde(rename = "async")]
    pub is_async: bool,
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum AppMode {
    Local,
    Remote,
}

#[derive(Parser, Debug)]
#[command(
    name = "Simple Test Controller",
    version,
    about = "A simple test controller application to execute tests in local or remote mode.",
    long_about = r#"
The Simple Test Controller is a command-line tool designed to help automate and
manage test execution across different environments.

You can run tests locally for quick validation or remotely to target devices
and servers on your network. It supports defining test dependencies, running
pre-execution hooks, and dynamically loading configuration files for flexible
test workflows.

Common use cases include:
  • Running integration or functional tests on a local machine
  • Executing test suites remotely over LAN-connected devices
  • Managing test configurations via key=value arguments or YAML files
  • Automating setup steps before test execution
  • Extending functionality with custom hooks and commands

Use the '--local' or '--remote' flags to select the desired execution mode.
"#
)]
pub struct AppArgs {
    /// Mode to run the controller in
    ///
    /// When in remote mode, the controller will connect to an IP address in which to run the tests
    /// When in local mode, the controller will run the tests in the local machine
    #[arg(short, long, value_enum)]
    pub mode: Option<AppMode>,
}
