use std::io::Read;

use super::*;
use crate::utils::{config::Config, rpgp::decrypt_full};
use anyhow::{Context, Result};

/// Decrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    message: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get().context("Failed to get config")?;

    let message = match args.message {
        Some(m) => m,
        None => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read from stdin")?;
            buffer
        }
    };

    if message.is_empty() {
        return Err(anyhow::anyhow!(
            "No message provided.\nUsage: envx decrypt [message] or echo [message] | envx decrypt"
        ));
    }

    let decrypted = decrypt_full(message, &config)?;

    println!("{}", decrypted);

    Ok(())
}
