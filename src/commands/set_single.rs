use crate::{
    sdk::SDK,
    utils::{config::get_local_or_global_config, key::VecKeyTrait},
};
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
    kvpair: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let (key, value) = args.kvpair.split_once('=').unwrap();

    let id = SDK::set_env(
        &key.to_uppercase(),
        &value,
        &args.key,
        "be38c68e-5003-4493-a7b8-5653e8db26a6",
        "dd2bc69b-cabe-4e35-903e-ef76485bd757",
    )
    .await?;

    dbg!(id);

    Ok(())
}
