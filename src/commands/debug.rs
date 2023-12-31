use pgp::{types::KeyTrait, KeyType};

use crate::utils::{
    rpgp::{generate_key_pair, GenerationOptions},
    vecu8::ToHex,
};

// use crate::utils::vecu8::ToHex;

use super::*;
// use pgp::{composed::message::Message, types::KeyTrait, Deserializable};

/// Unset the current project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    let options = GenerationOptions::default()
        .identity("Testing Person", "test@example.com")
        .build();

    let keypair = generate_key_pair(options)?;

    dbg!(keypair.secret_key.fingerprint().to_hex());

    Ok(())
}
