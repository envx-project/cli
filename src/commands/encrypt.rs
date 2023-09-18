use anyhow::Context;

use crate::utils::{
    config::get_config,
    e::{encrypt, get_vault_location},
};

use super::*;

/// Encrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    /// recipient's public key fingerprint
    recipient: String,

    /// string to encrypt
    message: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config().context("Failed to get config")?;

    let primary_key = config.primary_key.clone();

    let primary_key_location = get_vault_location()?.join(primary_key).join("public.key");

    let primary_public_key =
        std::fs::read_to_string(primary_key_location).context("Failed to read primary key")?;

    let encrypted = encrypt(&args.message, primary_public_key.as_str())?;

    println!("{}", encrypted);

    Ok(())
}

// pub async fn command(args: Args, _json: bool) -> Result<()> {
//     match encrypt(args.recipient, args.text).await {
//         Ok(encrypted) => println!("{}", encrypted),
//         Err(error) => eprintln!("{}", error),
//     }

//     Ok(())
// }
