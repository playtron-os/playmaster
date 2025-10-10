use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum AppMode {
    Local,
    Remote,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Generate Dart integration tests from YAML files
    Gen,

    /// Generate JSON schema for the YAML files
    Schema,

    /// Run tests in either local or remote mode
    Run {
        /// Mode to run the controller in
        ///
        /// When in remote mode, the controller will connect to an IP address in which to run the tests
        /// When in local mode, the controller will run the tests in the local machine
        #[arg(short, long, value_enum)]
        mode: Option<AppMode>,

        /// Auto accept dependency installation prompts
        #[arg(short, long, default_value_t = false)]
        yes: bool,

        /// Whether to perform only setup tasks without executing tests
        #[arg(short, long, default_value_t = false)]
        setup: bool,
    },
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "PlayMaster",
    version,
    about = "A simple test controller application to execute or generate Flutter integration tests.",
    long_about = r#"
The PlayMaster is a command-line tool designed to help automate and
manage test execution across different environments or generate Dart test files
from YAML-based feature definitions.

Use the `gen` subcommand to generate Dart integration tests, or `run` to execute
them locally or remotely.
"#
)]
pub struct AppArgs {
    #[command(subcommand)]
    pub command: Command,
}
