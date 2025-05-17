use std::io::Read;

use anyhow::Context;

use crate::utils::{
    config::Config,
    rpgp::{encrypt, get_vault_location},
};

use super::*;

/// Encrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    /// recipient's public key fingerprint
    #[clap(long, short)]
    recipient: String,

    /// string to encrypt
    message: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get().context("Failed to get config")?;

    let primary_key = config.primary_key.clone();

    let primary_key_location =
        get_vault_location()?.join(primary_key).join("public.key");

    let primary_public_key = std::fs::read_to_string(primary_key_location)
        .context("Failed to read primary key")?;

    let message = match args.message {
        Some(message) => message,
        None => {
            let mut message = String::new();
            std::io::stdin()
                .read_to_string(&mut message)
                .context("Failed to read message")?;
            message
        }
    };

    if message.is_empty() {
        anyhow::bail!("Message is empty");
    }

    let encrypted = encrypt(&message, primary_public_key.as_str())?;

    println!("{}", encrypted);

    Ok(())
}
