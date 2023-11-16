use super::*;
use crate::utils::config::get_local_or_global_config;
use crate::utils::vecu8::ToHex;
use pgp::{types::KeyTrait, Deserializable};
use std::io::Cursor;

/// Import ascii armored keys from a file
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    path: String,

    #[clap(long)]
    pubkey: bool,

    #[clap(long)]
    secret_key: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    println!("Importing keys...");

    let _config = get_local_or_global_config().context("Failed to get config")?;

    let file = std::fs::read_to_string(&args.path).context("Failed to read file")?;

    if args.pubkey {
        let buf = Cursor::new(file.clone());
        let (key, _) = pgp::composed::SignedPublicKey::from_armor_single(buf)
            .context("Failed to parse armored key")?;

        let fingerprint = key.fingerprint().to_hex().to_uppercase();

        println!("Importing key: {}", fingerprint);

        return Ok(());
    }

    Ok(())
}
