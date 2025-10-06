use clap::{Parser, ValueEnum};

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
