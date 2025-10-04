use std::env;

use crate::{config::Config, run::Run, utils::errors::EmptyResult};

mod config;
mod hooks;
#[cfg(target_os = "linux")]
mod linux;
mod run;
mod utils;

fn main() -> EmptyResult {
    let version = env!("CARGO_PKG_VERSION");

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!("{}", version);
        return Ok(());
    }

    println!("🔧 Simple Test Controller, Version: {version}");

    #[cfg(target_os = "linux")]
    {
        use crate::utils::command::CommandUtils;
        CommandUtils::set_death_signal();
    }

    let config = Config::from_curr_dir()?;
    let run = Run::new(config);
    run.execute()?;

    Ok(())
}
