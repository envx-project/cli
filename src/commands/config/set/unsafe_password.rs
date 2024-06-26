use super::*;
use crate::utils::{
    config::{get_config, get_config_path},
    prompt::{prompt_confirm, prompt_password},
};

#[derive(Parser)]
pub struct Args {
    /// UNSAFE: Set the primary key password in plain text. Enter "" to unset the password.
    #[clap(short, long)]
    password: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    println!("This command is VERY insecure. It will store your password in PLAIN TEXT in the config file.");
    prompt_confirm("Are you sure you want to continue?")?;

    let mut config = get_config()?;

    let password = match args.password {
        Some(k) => k,
        None => prompt_password("Enter the password to set")?,
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
        get_config_path()?.to_str().unwrap_or("INVALID PATH")
    );

    config.write()?;

    Ok(())
}
