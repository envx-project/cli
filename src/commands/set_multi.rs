use crate::{
    sdk::SDK,
    utils::{config::get_local_or_global_config, key::VecKeyTrait, kvpair::KVPair},
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

    /// KVPairs
    #[clap(trailing_var_arg = true)]
    kvpairs: Vec<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let kvpairs = args
        .kvpairs
        .iter()
        .map(|k| {
            let (key, value) = k.split_once('=').unwrap();
            KVPair::new(key.to_uppercase(), value.to_string())
        })
        .collect::<Vec<KVPair>>();

    let ids = SDK::set_many(
        kvpairs,
        &args.key,
        "be38c68e-5003-4493-a7b8-5653e8db26a6",
        "dd2bc69b-cabe-4e35-903e-ef76485bd757",
    )
    .await?;

    println!("Uploaded {} variables", ids.len());
    println!("IDs: {:?}", ids);

    Ok(())
}
