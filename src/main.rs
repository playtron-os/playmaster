use clap::Parser as _;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::mpsc;
use std::thread;
use tracing::info;

use crate::{
    code_gen::r#gen::CodeGen,
    code_run::run::CodeRun,
    models::{args::AppArgs, config::Config, vars::Vars},
    schemas::schema_gen::SchemaGen,
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, ResultTrait},
        logger::LoggerUtils,
    },
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
    let mut signals = Signals::new([SIGINT, SIGTERM]).auto_err("Failed to init signal handler")?;
    let args = AppArgs::parse();

    #[cfg(target_os = "linux")]
    {
        use crate::utils::command::CommandUtils;
        CommandUtils::set_death_signal();
    }

    LoggerUtils::init();

    let version = env!("CARGO_PKG_VERSION");
    info!("🔧 PlayMaster, Version: {version}");

    // Channel to notify when to exit
    let (tx, rx) = mpsc::channel::<&'static str>();

    // 🧩 Spawn your worker thread
    let tx_worker = tx.clone();
    thread::spawn(move || {
        let result = match args.command {
            models::args::Command::Run { .. } => {
                let config = Config::from_curr_dir()?;
                let vars = Vars::all_from_curr_dir()?;
                let run = CodeRun::new(args, config, vars);
                run.execute()
            }
            models::args::Command::Gen => {
                let config = Config::from_curr_dir()?;
                let vars = Vars::all_from_curr_dir()?;
                let code_gen = CodeGen::new(args, config, vars);
                code_gen.execute()
            }
            models::args::Command::Schema => {
                let schema_gen = SchemaGen::new();
                schema_gen.execute()
            }
        };

        match result {
            Ok(_) => {
                let _ = tx_worker.send("done");
            }
            Err(e) => {
                eprintln!("❌ Error: {e}");
                let _ = tx_worker.send("error");
            }
        }

        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });

    // 🧩 Spawn signal handler thread
    let tx_signal = tx.clone();
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    println!("Received SIGINT (Ctrl+C)");
                    let _ = tx_signal.send("signal");
                    break;
                }
                SIGTERM => {
                    println!("Received SIGTERM (system kill)");
                    let _ = tx_signal.send("signal");
                    break;
                }
                _ => {}
            }
        }
    });

    // 🧩 Wait for either thread to finish
    match rx.recv() {
        Ok("done") => println!("✅ Execution completed successfully."),
        Ok("error") => println!("❌ Execution ended with error."),
        Ok("signal") => {
            println!("⚠️ Termination signal received.");
            let _ = CommandUtils::terminate_all_cmds();
        }
        _ => println!("Unknown exit reason."),
    }

    Ok(())
}
