use super::*;
use crate::utils::{config::Config, rpgp::decrypt_full};
use anyhow::{Context, Result};

/// Decrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    message: String,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get().context("Failed to get config")?;

    let decrypted = decrypt_full(args.message, &config)?;

    println!("{}", decrypted);

    Ok(())
}
