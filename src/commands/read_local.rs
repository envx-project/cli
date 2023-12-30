use super::*;
use crate::utils::{config::get_config, kvpair::KVPair, rpgp::decrypt_full};
use anyhow::Context;
use std::vec;

/// Read from local store
#[derive(Debug, Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    let config = get_config()?;

    let mut file = std::fs::File::open(".envcli.vault")
        .context("Failed to open .envcli.vault, try running `envcli init`")?;

    let variables = match serde_json::from_reader::<_, Vec<String>>(&mut file) {
        Ok(parsed) => parsed,
        Err(_) => vec![],
    };

    let decrypted_variables = variables
        .iter()
        .map(|x| {
            let decrypted = decrypt_full(x.clone(), &config)?;
            KVPair::from_json(decrypted)
        })
        .collect::<Result<Vec<KVPair>>>()?;

    let decrypted_variables = decrypted_variables
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>();

    println!("{}", decrypted_variables.join("\n"));

    Ok(())
}
