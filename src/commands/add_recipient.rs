// TODO: remove this file
use super::*;
use crate::utils::config::get_config;
use crate::utils::rpgp::{decrypt_full, encrypt_multi};

/// Initialize a new env-store
#[derive(Debug, Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let config = get_config()?;

    let mut vault = std::fs::File::open(".envcli.vault")
        .context("Failed to open .envcli.vault, try running `envcli init`")?;

    let variables = match serde_json::from_reader::<_, Vec<String>>(&mut vault) {
        Ok(parsed) => parsed,
        Err(_) => vec![],
    };

    if variables.is_empty() {
        println!("No variables found in vault");
        return Ok(());
    }

    let first = variables.first().unwrap();

    let decrypted = decrypt_full(first.clone(), &config)?;

    let primary_pubkey = config
        .primary_key()
        .context("Failed to get primary key, try generating a new one with `envcli gen`")?;

    let secondary_pubkey = config.keys.first().unwrap().public_key()?;

    let encrypted = encrypt_multi(&decrypted, vec![&primary_pubkey, &secondary_pubkey])?;

    println!("{}", encrypted);

    Ok(())
}
