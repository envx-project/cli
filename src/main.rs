use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::*;

mod commands;
mod sdk;
mod types;
mod utils;

#[macro_use]
mod macros;

/// Interact with env-store/envs via CLI
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
    add_user_to_project,
    auth,
    config,
    debug,
    decrypt,
    delete_key,
    encrypt,
    export,
    gen,
    get_config,
    get_project,
    import,
    init,
    link,
    list_keys,
    project,
    read_local,
    run,
    set_local,
    set,
    shell,
    sign,
    unlink,
    unset,
    upload,
    variables,
    version,
    // commands with subcommands
    delete,
    new
);

#[tokio::main]
async fn main() -> Result<()> {
    let config = crate::utils::config::get_config()?;
    let global_silent = config.silent.unwrap_or_default();

    let cli = Args::parse();
    let command = std::env::args().nth(1).unwrap_or_default();

    if command != "export" && !cli.silent && !global_silent {
        println!(
            "{} {} {} {}",
            "env-cli".cyan(),
            env!("CARGO_PKG_VERSION").magenta(),
            "by".blue(),
            "alexng353".yellow()
        );
    }

    match Commands::exec(cli).await {
        Ok(_) => {}
        Err(e) => {
            // If the user cancels the operation, we want to exit successfully
            // This can happen if Ctrl+C is pressed during a prompt
            if e.root_cause().to_string() == inquire::InquireError::OperationInterrupted.to_string()
            {
                return Ok(());
            }

            eprintln!("{:?}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
