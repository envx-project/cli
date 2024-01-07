use crate::utils::{config::get_config, prompt::prompt_select};

use super::*;

#[derive(Parser)]
pub struct Args {
    /// Partial fingerprint of the key to set
    #[clap(short, long)]
    key: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = get_config()?;

    let fingerprint = match args.key {
        Some(k) => k,
        None => prompt_select("Select key to set as primary", config.keys.clone())?.fingerprint,
    };

    config.set_primary_key(&fingerprint)?;
    config.write()?;

    Ok(())
}
