use super::*;
use crate::utils::config::get_local_or_global_config;

#[derive(Parser)]
pub struct Args {
    /// Use full length fingerprints
    #[clap(short, long)]
    full: bool,
}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let config = get_local_or_global_config().context("Failed to get config")?;

    println!("Keys:");
    for key in config.keys.iter() {
        let fingerprint = match _args.full {
            true => &key.fingerprint,
            false => &key.fingerprint[..8],
        };
        println!("\t{} {}", fingerprint, key.primary_user_id);
    }

    Ok(())
}
