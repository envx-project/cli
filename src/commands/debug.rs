use crate::utils::keyring::get_password;

use super::*;

/// Unset the current project
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    key: Option<String>,
}

pub async fn command(_args: Args) -> Result<()> {
    if let Some(key) = _args.key {
        let password = get_password(&key)?;
        println!("Password: {}", password);
    }

    Ok(())
}
