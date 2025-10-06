use clap::Parser as _;
use tracing::info;

use crate::{
    code_gen::code_gen::CodeGen,
    models::{args::AppArgs, config::Config},
    run::Run,
    utils::{errors::EmptyResult, logger::LoggerUtils},
};

mod code_gen;
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

    match args.command {
        models::args::Command::Run { .. } => {
            let run = Run::new(args, config);
            run.execute()?;
        }
        models::args::Command::Gen {} => {
            let code_gen = CodeGen::new(args, config);
            code_gen.execute()?;
        }
    }

    Ok(())
}
