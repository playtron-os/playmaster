use std::env;

use crate::{config::Config, run::Run, utils::errors::EmptyResult};

mod config;
mod hooks;
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

    let config = Config::from_curr_dir()?;
    let run = Run::new(config);
    run.execute()?;

    Ok(())
}
