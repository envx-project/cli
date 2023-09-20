use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::*;

mod commands;
mod consts;
mod errors;
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
commands_enum!(variables, set, unset, shell, run, encrypt, decrypt, gen, delete_key);

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();

    println!(
        "{} {} {} {}",
        "env-cli".cyan(),
        env!("CARGO_PKG_VERSION").magenta(),
        "by".blue(),
        "alexng353".yellow()
    );

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
