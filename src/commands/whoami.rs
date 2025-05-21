use super::*;
use crate::utils::config::Config;

/// Print the primary key fingerprint and uuid
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    let config = Config::get().context("Failed to get config")?;
    let primary_key = config.get_key(&config.primary_key)?;
    println!(
        "{} - {}",
        &primary_key.fingerprint[..8],
        primary_key.uuid.unwrap_or("Not on remote".into())
    );
    Ok(())
}
