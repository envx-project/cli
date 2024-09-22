use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::*;
use home::home_dir;

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
    auth, decrypt, encrypt, export, gen, import, link, list_keys, run, set,
    shell, sign, unlink, unset, upload, variables, version,
    // commands with subcommands
    config, delete, get, keyring, new, project
);

#[tokio::main]
async fn main() -> Result<()> {
    // check if config file exists at ~/.config/envcli/config.json
    let mut config_path = home_dir().unwrap();
    config_path.push(".config/envcli/config.json");

    if config_path.exists() {
        let args = std::env::args().collect::<Vec<String>>();

        if !(args.iter().any(|a| a == "config")
            && args.iter().any(|a| a == "migrate"))
        {
            eprintln!("The version 1 config file has been detected. Please run `{}` to migrate your config file to the new format.", "envx config migrate".green());
            eprintln!("If you have already migrated, please delete the old config file at {}", config_path.to_str().unwrap_or("INVALID PATH"));
            return Ok(());
        }
    }

    let cli = Args::parse();
    match Commands::exec(cli).await {
        Ok(_) => {}
        Err(e) => {
            // If the user cancels the operation, we want to exit successfully
            // This can happen if Ctrl+C is pressed during a prompt
            if e.root_cause().to_string()
                == inquire::InquireError::OperationInterrupted.to_string()
            {
                return Ok(());
            }

            eprintln!("{:?}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
