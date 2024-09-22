use crate::utils::{
    config::get_config,
    keyring::get_password,
    prompt::{prompt_confirm_with_default, prompt_select},
};

use super::*;

/// View the saved passphrase for a key
///
/// This command is interactive
#[derive(Parser)]
pub struct Args {
    /// Partial fingerprint of the key to set
    #[clap(short, long)]
    key: Option<String>,

    /// Force, don't prompt for confirmation
    #[clap(short, long)]
    force: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;

    let fingerprint = match args.key {
        Some(key) => config.get_key(&key)?.fingerprint,
        None => {
            prompt_select("Select key to view password", config.keys)?
                .fingerprint
        }
    };

    let password = get_password(&fingerprint)?;

    if args.force {
        println!("{}", password);
        return Ok(());
    }

    println!("This will print the saved password in PLAIN TEXT");
    match prompt_confirm_with_default(
        "Are you sure you want to continue? (y/N)",
        false,
    ) {
        Ok(true) => {
            println!("Password:");
            println!("{}", password)
        }
        _ => println!("Aborting"),
    }

    Ok(())
}
