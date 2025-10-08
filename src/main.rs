use clap::Parser as _;
use tracing::info;

use crate::{
    code_gen::code_gen::CodeGen,
    code_run::code_run::CodeRun,
    models::{args::AppArgs, config::Config},
    schemas::schema_gen::SchemaGen,
    utils::{errors::EmptyResult, logger::LoggerUtils},
};

mod code_gen;
mod code_run;
mod hooks;
#[cfg(target_os = "linux")]
mod linux;
mod models;
mod schemas;
mod utils;

fn main() -> EmptyResult {
    let args = AppArgs::parse();

    #[cfg(target_os = "linux")]
    {
        use crate::utils::command::CommandUtils;
        CommandUtils::set_death_signal();
    }

    LoggerUtils::init();

    let version = env!("CARGO_PKG_VERSION");
    info!("🔧 Simple Test Controller, Version: {version}");

    let config = Config::from_curr_dir()?;

    match args.command {
        models::args::Command::Run { .. } => {
            let run = CodeRun::new(args, config);
            run.execute()?;
        }
        models::args::Command::Gen {} => {
            let code_gen = CodeGen::new(args, config);
            code_gen.execute()?;
        }
        models::args::Command::Schema {} => {
            let schema_gen = SchemaGen::new();
            schema_gen.execute()?;
        }
    }

    Ok(())
}
