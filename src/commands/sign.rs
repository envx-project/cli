use crate::utils::{config::get_local_or_global_config, key::VecKeyTrait};
use crypto_hash::{hex_digest, Algorithm};

use super::*;
use anyhow::Ok;
use pgp::{composed, crypto, Deserializable, SignedSecretKey};

/// Sign a message with a key
#[derive(Parser)]
pub struct Args {
    /// Key to sign with
    #[clap(short, long)]
    key: String,

    /// Message to sign
    message: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_local_or_global_config().context("Failed to get config")?;

    let key = config
        .keys
        .iter()
        .find(|k| k.fingerprint.contains(&args.key))
        .ok_or_else(|| anyhow!("Key not found"))?;

    let key = SignedSecretKey::from_string(&key.secret_key()?)?.0;

    let message = composed::message::Message::new_literal("none", &args.message);

    println!("{}", message.to_armored_string(None)?);

    let pw = || "asdf".to_string();

    let signature = message.sign(&key, pw, crypto::hash::HashAlgorithm::SHA3_512)?;

    println!("{}", signature.to_armored_string(None)?);

    Ok(())
}
