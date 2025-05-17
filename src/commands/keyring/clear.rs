use crate::utils::{
    config::Config, keyring::clear_password, prompt::prompt_select,
};

use super::*;

/// Clear the saved passphrase for a key
///
/// This command is interactive
#[derive(Parser)]
pub struct Args {
    /// Partial fingerprint of the key to set
    #[clap(short, long)]
    key: Option<String>,

    /// Clear all saved keys
    #[clap(short, long)]
    all: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get()?;

    if args.all {
        config.keys.iter().for_each(|key| {
            clear_password(&key.fingerprint).unwrap();
        });
        return Ok(());
    }

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
