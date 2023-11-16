use crate::utils::prompt::prompt_multi_options;

use super::*;
use anyhow::{bail, Context};

/// Delete a key
#[derive(Debug, Parser)]
pub struct Args {
    /// Args to pass to the command
    #[clap(trailing_var_arg = true)]
    args: Vec<String>,
}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let mut config = crate::utils::config::get_config().context("Failed to get config")?;

    let selected = prompt_multi_options(
        "Select keys to delete",
        config
            .keys
            .clone()
            .iter()
            .map(|k| k.fingerprint.clone())
            .collect(),
    )
    .context("Failed to prompt for options")?;

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

    crate::utils::config::write_config(&config).context("Failed to write config")?;

    Ok(())
}
