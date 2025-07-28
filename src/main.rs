use std::{cmp::Ordering, io::IsTerminal};

use anyhow::Result;
use clap::{error::ErrorKind, Parser, Subcommand};
use commands::*;
use utils::{compare_semver, config::Config};

mod commands;
mod constants;
mod sdk;
mod types;
mod utils;

#[macro_use]
mod macros;

/// Interact with env-store/rusty-api via CLI
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(long)]
    silent: bool,
}

// Generates the commands based on the modules in the commands directory
// Specify the modules you want to include in the commands_enum! macro
commands_enum!(
    auth, export, gen, import, link, list_keys, run, set, shell, unlink, unset,
    update, upload, variables, version, whoami,
    // commands with subcommands
    config, delete, get, keyring, project
);

fn spawn_update_task(
    mut config: Config,
) -> tokio::task::JoinHandle<Result<(), anyhow::Error>> {
    tokio::spawn(async move {
        if !std::io::stdout().is_terminal() {
            return Ok::<(), anyhow::Error>(());
        }

        let result = config.check_update(false).await;
        if let Ok(Some(latest_version)) = result {
            config.new_version_available = Some(latest_version);
        }
        config.write()?;
        Ok::<(), anyhow::Error>(())
    })
}

async fn handle_update_task(
    handle: Option<tokio::task::JoinHandle<Result<(), anyhow::Error>>>,
) {
    if let Some(handle) = handle {
        match handle.await {
            Ok(Ok(_)) => {} // Task completed successfully
            Ok(Err(e)) => {
                if !std::io::stdout().is_terminal() {
                    eprintln!("Failed to check for updates (not fatal)");
                    eprintln!("{}", e);
                }
            }
            Err(e) => {
                eprintln!("Check Updates: Task panicked or failed to execute.");
                eprintln!("{}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let check_updates_handle = if std::io::stdout().is_terminal() {
        let mut config = Config::get()?;

        if let Some(new_version) = &config.new_version_available {
            if matches!(
                compare_semver(env!("CARGO_PKG_VERSION"), &new_version),
                Ordering::Less
            ) {
                println!(
                    "{} {}: v{} -> v{}",
                    "info!".bold(),
                    "Update available".green().bold(),
                    env!("CARGO_PKG_VERSION").yellow(),
                    new_version.bright_yellow(),
                );
                println!(
                    "{} Run `{}` to update\n",
                    "info!".bold(),
                    "curl -fsSL https://get.envx.sh | sh".green()
                );
            } else {
                // TODO: rewrite this to use .config/envx/version instead of the config file
                config.new_version_available = None;
                config.write()?;
            }
        }

        Some(spawn_update_task(config))
    } else {
        None
    };

    // Trace from where Args::parse() bubbles an error to where it gets caught
    // and handled.
    //
    // https://github.com/clap-rs/clap/blob/cb2352f84a7663f32a89e70f01ad24446d5fa1e2/clap_builder/src/derive.rs#L30-L42
    // https://github.com/clap-rs/clap/blob/cb2352f84a7663f32a89e70f01ad24446d5fa1e2/clap_builder/src/error/mod.rs#L233-L237
    //
    // This code tells us what exit code to use:
    // https://github.com/clap-rs/clap/blob/cb2352f84a7663f32a89e70f01ad24446d5fa1e2/clap_builder/src/error/mod.rs#L221-L227
    //
    // https://github.com/clap-rs/clap/blob/cb2352f84a7663f32a89e70f01ad24446d5fa1e2/clap_builder/src/error/mod.rs#L206-L208
    //
    // This code tells us what stream to print the error to:
    // https://github.com/clap-rs/clap/blob/cb2352f84a7663f32a89e70f01ad24446d5fa1e2/clap_builder/src/error/mod.rs#L210-L215
    //
    // pub(crate) fn stream(&self) -> Stream {
    //     match self.kind() {
    //         ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => Stream::Stdout,
    //         _ => Stream::Stderr,
    //     }
    // }

    let cli = match Args::try_parse() {
        Ok(args) => args,
        // Clap's source code specifically says that these errors should be
        // printed to stdout and exit with a status of 0.
        Err(e)
            if e.kind() == ErrorKind::DisplayHelp
                || e.kind() == ErrorKind::DisplayVersion =>
        {
            println!("{}", e);
            handle_update_task(check_updates_handle).await;
            std::process::exit(0); // Exit 0 (because of error kind)
        }
        Err(e) => {
            eprintln!("{}", e);
            handle_update_task(check_updates_handle).await;
            std::process::exit(2); // Exit 2 (default)
        }
    };

    let exec_result = Commands::exec(cli).await;

    if let Err(e) = exec_result {
        if matches!(
            e.root_cause().downcast_ref::<inquire::InquireError>(),
            Some(&inquire::InquireError::OperationInterrupted)
                | Some(&inquire::InquireError::OperationCanceled)
        ) {
            // We don't wait for the update task to finish because we want to
            // immediately exit if the user presses Ctrl+C
            return Ok(()); // Exit gracefully if interrupted
        }

        eprintln!("{:?}", e);
        handle_update_task(check_updates_handle).await;
        std::process::exit(1);
    }

    handle_update_task(check_updates_handle).await;

    Ok(())
}
