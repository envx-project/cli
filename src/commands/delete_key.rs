// TODO: also delete keys on the server side

use crate::utils::prompt::prompt_multi_options;

use super::*;
use anyhow::{bail, Context};

/// Delete a key (Caution, keys will still stay on the server for now)
#[derive(Debug, Parser)]
pub struct Args {
    /// Fingerprint of the key to delete
    #[clap(short, long)]
    key: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let mut config = crate::utils::config::get_config().context("Failed to get config")?;

    let selected: Vec<String> = match args.key {
        Some(key) => {
            let key = config.get_key(&key).context("Failed to get key")?;
            vec![key.fingerprint.clone()]
        }
        None => prompt_multi_options("Select keys to delete", config.keys.clone())?
            .iter()
            .map(|k| k.fingerprint.clone())
            .collect(),
    };

    let selected = selected
        .iter()
        .map(|s| {
            // split at " - " and take the first part
            s.split(" - ")
                .next()
                .expect("Failed to split fingerprint")
                .to_string()
        })
        .collect::<Vec<_>>();

    println!("Deleting keys: {:?}", selected);

    let vault_location = crate::utils::rpgp::get_vault_location()?;

    for key in selected {
        let key_dir = vault_location.join(&key);
        if !key_dir.exists() {
            bail!("Key {} does not exist", key);
        }
        std::fs::remove_dir_all(key_dir).context("Failed to delete key directory")?;

        config.keys.retain(|k| k.fingerprint != key);
    }

    config.write().context("Failed to write config")?;

    Ok(())
}
