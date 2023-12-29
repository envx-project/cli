use super::*;
use crate::utils::{config::get_config, key::VecKeyTrait, prompt::prompt_options};

/// Export a public or secret key
#[derive(Parser)]
pub struct Args {
    /// The fingerprint of the key to export
    #[clap(short, long)]
    fingerprint: Option<String>,

    /// Export the secret key
    #[clap(short, long = "secret-key")]
    secret_key: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config().context("Failed to get config")?;

    let keys: Vec<&str> = config.keys.all_fingerprints();

    let fingerprint = match args.fingerprint {
        Some(fingerprint) => fingerprint.to_uppercase(),
        None => prompt_options(
            "Select key to export",
            keys.iter().map(|e| e[..8].to_string()).collect(),
        )?
        .to_string(),
    };

    let key = config
        .keys
        .iter()
        .find(|k| k.fingerprint.starts_with(&fingerprint))
        .context("Failed to find key".red())?;

    let key = if args.secret_key {
        key.secret_key()?
    } else {
        key.public_key()?
    };

    println!("{}", key);

    Ok(())
}
