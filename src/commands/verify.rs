use std::env::args;

use crate::utils::config::get_local_or_global_config;

use super::*;
use anyhow::Ok;
use pgp::{composed, Deserializable, SignedPublicKey, SignedSecretKey};

/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {
    /// Key to sign with
    #[clap(short, long)]
    key: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let msg = "-----BEGIN PGP MESSAGE-----

xA0DAQ4BQfaaFRiCYR8ByxZ1BG5vbmVljUMnSGVsbG8gV29ybGQhwsBzBAEBDgAd
FiEEIdVgFYnAyYyeORJhQfaaFRiCYR8FAmWNQycACgkQQfaaFRiCYR8Gzgf/ZUub
IE9Ju2k2Xy7HUT203vQ2V7mMued5Jek8XA/apZ/cvQu5ejItW+67xVmWKIm4TNis
e0IUjW9PccwVWDG+kVDHdt4QOXMZsUmppZK7CylGAAr0gwnevMg79fRYftGaJzFW
a9oKSkmT9y/QJWVDrJQC1c3asGJv9neV/AMmo3v1TXaJ+g6yObPy4VP1WxEaNFd6
LexUXAga4PAU7+oDqhMaQFNEvaSAomM/5CHaDT4AQ7MaJ/T0VTr6VyPncKBofx5b
hPXTjkXi3oX3rENjU9w8uuzzTmrHMdtOgYz0mvgY7GtWxL3/evCSjZW5Nitd3DEj
xS01Yh2Y03he2rIJCQ==
=NzL+
-----END PGP MESSAGE-----";

    let config = get_local_or_global_config().context("Failed to get config")?;

    let key = config
        .keys
        .iter()
        .find(|k| k.fingerprint.contains(&args.key))
        .ok_or_else(|| anyhow!("Key not found"))?;

    let key = SignedPublicKey::from_string(&key.public_key()?)?.0;

    let message = composed::message::Message::new_literal("none", msg);

    let verified = message.verify(&key);

    println!("{}", message.to_armored_string(None)?);
    println!("{:?}", verified.is_ok());

    Ok(())
}
