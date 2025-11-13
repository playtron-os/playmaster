use clap::Parser as _;
use rustls::crypto::aws_lc_rs;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::mpsc;
use std::thread;
use tracing::{debug, error, info, warn};

use crate::{
    code_gen::r#gen::CodeGen,
    code_run::run::CodeRun,
    gmail::client::GmailClient,
    models::{args::AppArgs, config::Config, vars::Vars},
    schemas::schema_gen::SchemaGen,
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, ResultTrait},
        execution::ExecutionUtils,
        logger::LoggerUtils,
    },
};

mod code_gen;
mod code_run;
mod gmail;
mod hooks;
#[cfg(target_os = "linux")]
mod linux;
mod models;
mod schemas;
mod utils;

#[tokio::main]
async fn main() -> EmptyResult {
    let mut signals = Signals::new([SIGINT, SIGTERM]).auto_err("Failed to init signal handler")?;
    let args = AppArgs::parse();

    #[cfg(target_os = "linux")]
    {
        use crate::utils::command::CommandUtils;
        CommandUtils::set_death_signal();
    }

    LoggerUtils::init();

    aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install AWS-LC crypto provider");

    let version = env!("CARGO_PKG_VERSION");
    info!("🔧 PlayMaster, Version: {version}");

    // Channel to notify when to exit
    let (tx, rx) = mpsc::channel::<&'static str>();

    // 🧩 Spawn your worker thread
    let tx_worker = tx.clone();
    tokio::spawn(async move {
        debug!(
            "Worker thread started with args: {:?}, command: {:?}",
            args, args.command
        );

        let result = process_command(args).await;

        match result {
            Ok(_) => {
                let _ = tx_worker.send("done");
            }
            Err(e) => {
                error!("❌ Error: {e}");
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
                    warn!("Received SIGINT (Ctrl+C)");
                    let _ = tx_signal.send("signal");
                    break;
                }
                SIGTERM => {
                    warn!("Received SIGTERM (system kill)");
                    let _ = tx_signal.send("signal");
                    break;
                }
                _ => {}
            }
        }
    });

    // 🧩 Wait for either thread to finish
    match rx.recv() {
        Ok("done") => info!("✅ Execution completed successfully."),
        Ok("error") => error!("❌ Execution ended with error."),
        Ok("signal") => {
            warn!("⚠️ Termination signal received.");
            if let Err(err) = ExecutionUtils::set_running(false) {
                error!("Failed to set running to false: {}", err);
            }

            if let Err(err) = CommandUtils::terminate_all_cmds("") {
                error!("Failed to terminate running commands: {}", err);
            }
        }
        _ => error!("Unknown exit reason."),
    }

    Ok(())
}

async fn process_command(args: AppArgs) -> EmptyResult {
    match args.command {
        models::args::Command::Run { .. } => {
            let config = Config::from_curr_dir()?;
            let vars = Vars::all_from_curr_dir()?;
            let run = CodeRun::new(args, config, vars);
            run.execute().await
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
        models::args::Command::Gmail => {
            let config = Config::from_curr_dir()?;

            let gmail_client = if config.gmail.enabled
                && let Some(creds) = config.gmail.credentials.s3
            {
                GmailClient::new(Some(creds.bucket), Some(creds.key_prefix))
            } else {
                GmailClient::new(None, None)
            };

            gmail_client.generate_refresh_token().await
        }
    }
}
