use anyhow::bail;

use super::*;
use crate::utils::{
    config::Config,
    key::VecKeyTrait,
    prompt::{is_interactive, prompt_options},
};

/// Export a public or secret key
#[derive(Parser)]
pub struct Args {
    /// The fingerprint of the key to export
    #[arg(short = 'k', long = "key")]
    fingerprint: Option<String>,

    /// Export the secret key
    #[arg(short, long = "secret-key")]
    secret_key: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let keys: Vec<&str> = config.keys.all_fingerprints();

    let fingerprint = match args.fingerprint {
        Some(fingerprint) => fingerprint.to_uppercase(),
        None => {
            if !is_interactive() {
                bail!(
                    "No key selected and stdin is not a terminal.\n\
                     Pass --key <fingerprint> to pick a key non-interactively.",
                );
            }
            prompt_options(
                "Select key to export",
                keys.iter().map(|e| e[..8].to_string()).collect(),
            )?
            .to_string()
        }
    };

    let key = config
        .keys
        .iter()
        .find(|k| {
            k.fingerprint
                .to_uppercase()
                .starts_with(&fingerprint.to_uppercase())
        })
        .context("Failed to find key".red())?;

    let key = if args.secret_key {
        key.secret_key_str()?
    } else {
        key.public_key_str()?
    };

    println!("{}", key);

    Ok(())
}
