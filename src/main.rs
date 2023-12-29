use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::*;

mod commands;
mod errors;
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

    /// Output in JSON format
    #[clap(global = true, long)]
    json: bool,
}

// Generates the commands based on the modules in the commands directory
// Specify the modules you want to include in the commands_enum! macro
commands_enum!(
    add_recipient,
    auth,
    behaviour,
    config,
    decrypt,
    delete_key,
    encrypt,
    export,
    gen,
    get_config,
    get_project,
    import,
    init,
    list_keys,
    read_local,
    run,
    set_local,
    set,
    settings,
    shell,
    sign,
    unset,
    variables,
    verify,
    add_user_to_project,
    upload
);

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();

    let command = std::env::args().nth(1).unwrap_or_default();
    if command != "export" {
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
