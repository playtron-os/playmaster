use clap::Parser as _;
use tracing::info;

use crate::{
    models::{args::AppArgs, config::Config},
    run::Run,
    utils::{errors::EmptyResult, logger::LoggerUtils},
};

mod hooks;
#[cfg(target_os = "linux")]
mod linux;
mod models;
mod run;
mod utils;

fn main() -> EmptyResult {
    let args = AppArgs::parse();

    let version = env!("CARGO_PKG_VERSION");
    info!("🔧 Simple Test Controller, Version: {version}");

    #[cfg(target_os = "linux")]
    {
        use crate::utils::command::CommandUtils;
        CommandUtils::set_death_signal();
    }

    LoggerUtils::init();

    let config = Config::from_curr_dir()?;
    let run = Run::new(args, config);
    run.execute()?;

    Ok(())
}
