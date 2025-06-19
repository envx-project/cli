use crate::utils::{
    config::Config, keyring::get_password, prompt::prompt_confirm_with_default,
};

use super::*;

/// View the saved passphrase for a key
///
/// This command is interactive
#[derive(Parser)]
pub struct Args {
    /// Don't prompt for confirmation
    #[clap(short, long)]
    yes: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get()?;
    let password = get_password(&config)?;

    if args.yes {
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
