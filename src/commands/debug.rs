// use crate::utils::vecu8::ToHex;

use crate::{sdk::get_api_url, utils::keyring::get_password};

use super::*;
// use pgp::{composed::message::Message, types::KeyTrait, Deserializable};

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
