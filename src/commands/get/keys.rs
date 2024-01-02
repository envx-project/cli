use super::*;
use crate::utils::config::get_config;

#[derive(Parser)]
pub struct Args {
    /// Use full length fingerprints
    #[clap(short, long)]
    full: bool,
}

pub async fn command(_args: Args) -> Result<()> {
    let config = get_config().context("Failed to get config")?;

    println!("Keys:");
    for key in config.keys.iter() {
        let fingerprint = match _args.full {
            true => &key.fingerprint,
            false => &key.fingerprint[..8],
        };

        let uuid = match &key.uuid {
            Some(uuid) => uuid,
            None => "Not on remote",
        };

        println!("\t{} {} | {}", fingerprint, key.primary_user_id, uuid);
    }

    Ok(())
}
