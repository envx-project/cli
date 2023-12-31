use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::*;

mod commands;
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
    add_user_to_project,
    auth,
    debug,
    decrypt,
    encrypt,
    export,
    gen,
    import,
    link,
    run,
    shell,
    sign,
    unlink,
    unset,
    upload,
    variables,
    version,
    // commands with subcommands
    delete,
    get,
    new,
    secrets,
    set
);

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();
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
