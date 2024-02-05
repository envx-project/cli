use crate::utils::{
    config::get_config, keyring::clear_password, prompt::prompt_select,
};

use super::*;

#[derive(Parser)]
pub struct Args {
    /// Partial fingerprint of the key to set
    #[clap(short, long)]
    key: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;

    let fingerprint = match args.key {
        Some(key) => config.get_key(&key)?.fingerprint,
        None => {
            prompt_select("Select key to clear password", config.keys.clone())?
                .fingerprint
        }
    };

    clear_password(&fingerprint)?;

    Ok(())
}
