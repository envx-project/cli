use std::{cmp::Ordering, io::IsTerminal};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use commands::*;
use utils::state::StateStore;
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
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(long)]
    silent: bool,
}

// Generates the commands based on the modules in the commands directory
// Specify the modules you want to include in the commands_enum! macro
commands_enum!(
    auth,
    friends,
    add_friend,
    friend_link,
    send,
    inbox,
    read,
    completion,
    export,
    gen,
    import,
    link,
    list_projects,
    run,
    set,
    shell,
    unlink,
    unset,
    update,
    upload,
    variables,
    version,
    whoami,
    // commands with subcommands
    config,
    get,
    keyring,
    project,
    invite
);

fn spawn_update_task() -> tokio::task::JoinHandle<anyhow::Result<String>> {
    tokio::spawn(async move {
        // outputtng would break json output on CI
        if !std::io::stdout().is_terminal() {
            bail!("Stdout is not a terminal");
        }
        let latest_version = update::check_update(false).await?;

        Ok(latest_version)
    })
}

async fn handle_update_task(
    handle: Option<tokio::task::JoinHandle<anyhow::Result<String>>>,
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
                eprintln!("Check Updates: Task failed to execute.");
                eprintln!("{}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Help, version and usage errors must work without a writable or valid profile.
    let cli = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => error.exit(),
    };
    let mut config = Config::load()?;

    let check_updates_handle = if std::io::stdout().is_terminal() {
        let update = StateStore::open(&config)
            .and_then(|store| store.update_check())
            .unwrap_or_default();

        if let Some(latest_version) = update.latest_version {
            if matches!(
                compare_semver(env!("CARGO_PKG_VERSION"), &latest_version),
                Ordering::Less
            ) {
                println!(
                    "{} {}: v{} -> v{}",
                    "info!".bold(),
                    "Update available".green().bold(),
                    env!("CARGO_PKG_VERSION").yellow(),
                    latest_version.bright_yellow(),
                );
                println!(
                    "{} Run `{}` to update\n",
                    "info!".bold(),
                    "curl -fsSL https://get.envx.sh | sh".green()
                );
            }
        }

        Some(spawn_update_task())
    } else {
        None
    };

    let exec_result = {
        let exec_result = Commands::exec(cli, &mut config).await;
        config.write()?;
        exec_result
    };

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
