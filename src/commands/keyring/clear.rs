use crate::utils::{config::Config, keyring::clear_password};

use super::*;

/// Clear the saved passphrase for a key
///
/// This command is interactive
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, config: Config) -> Result<()> {
    let fingerprint = config.primary_key()?.fingerprint;
    clear_password(&fingerprint)?;

    Ok(())
}
