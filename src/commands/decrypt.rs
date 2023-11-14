use super::*;
use crate::utils::{config::get_local_or_global_config, rpgp::decrypt_full};
use anyhow::{Context, Result};

/// Decrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    message: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_local_or_global_config().context("Failed to get config")?;

    let decrypted = decrypt_full(args.message, &config)?;

    println!("{}", decrypted);

    Ok(())
}
