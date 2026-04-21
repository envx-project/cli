use anyhow::bail;

use super::*;
use crate::utils::{
    config::{get_config_file_path, Config},
    prompt::{is_interactive, prompt_confirm, prompt_password},
};

/// Set the primary key password in plain text
///
/// This command is VERY insecure. It will store your password in PLAIN TEXT in the config file.
/// Enter "" to unset the password.
#[derive(Parser)]
pub struct Args {
    /// UNSAFE: Set the primary key password in plain text. Enter "" to unset the password.
    #[arg(short, long)]
    password: Option<String>,

    /// Skip the "are you sure" confirmation prompt
    #[arg(short, long)]
    yes: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    println!("This command is VERY insecure. It will store your password in PLAIN TEXT in the config file.");

    if !args.yes {
        if !is_interactive() {
            bail!(
                "Refusing to store an unsafe password without confirmation in a non-interactive terminal.\n\
                 Re-run with --yes (-y) to confirm.",
            );
        }

        let confirmed = prompt_confirm("Are you sure you want to continue?")?;
        if !confirmed {
            println!("Aborting...");
            return Ok(());
        }
    }

    let password = match args.password {
        Some(k) => k,
        None => {
            if !is_interactive() {
                bail!(
                    "Cannot prompt for a password in a non-interactive terminal.\n\
                     Pass --password <value> (or --password \"\" to clear).",
                );
            }
            prompt_password("Enter the password to set")?
        }
    };

    if password.is_empty() {
        config.primary_key_password = None;
        println!("Primary key password removed");
    } else {
        config.primary_key_password = Some(password);
        println!("Primary key password set");
    }

    println!(
        "The config file is located at {}",
        get_config_file_path()?.to_str().unwrap_or("INVALID PATH")
    );

    Ok(())
}
