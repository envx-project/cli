use crate::utils::config::get_config;

use super::*;

/// Set the keyring expiry in days
///
/// 0 for never
#[derive(Parser)]
pub struct Args {
    /// Number of days before the keyring password expires
    #[clap(short, long)]
    days: u32,
}

pub async fn command(args: Args) -> Result<()> {
    if args.days == 0 {
        println!("Keyring will not expire. This is not recommended.");
    } else {
        println!("Setting keyring expiry to {} days", args.days);
    }
    let mut config = get_config()?;
    let mut settings = config.get_settings()?;

    if args.days == 0 {
        settings.set_keyring_expiry_never();
    } else {
        settings.set_keyring_expiry(args.days);
    }

    config.settings = Some(settings);
    config.write()?;
    Ok(())
}
