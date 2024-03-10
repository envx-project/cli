use super::*;
use crate::utils::{config::get_config, keyring::get_password, prompt::prompt_select};

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
        None => prompt_select("Select key to check password", config.keys.clone())?.fingerprint,
    };

    let password = get_password(&fingerprint)?;
    println!("{}", password);

    Ok(())
}
