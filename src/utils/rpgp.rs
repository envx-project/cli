use anyhow::{Context, Ok, Result};
use hex::ToHex;
use pgp::{composed, composed::signed_key::*, crypto, types::SecretKeyTrait, Deserializable};
use rand::prelude::*;
use smallvec::*;
use std::io::Cursor;

use super::config::{get_config, Config};
use crypto_hash::{hex_digest, Algorithm};

#[derive(Debug)]
pub struct KeyPair {
    pub secret_key: pgp::SignedSecretKey,
    pub public_key: pgp::SignedPublicKey,
}

pub(crate) fn get_vault_location() -> anyhow::Result<std::path::PathBuf, anyhow::Error> {
    let path = home::home_dir()
        .context("Failed to get home directory")?
        .join(".config")
        .join("envcli")
        .join("keys");

    Ok(path)
}

pub fn generate_key_pair(
    name: String,
    email: String,
    password: String,
) -> Result<KeyPair, anyhow::Error> {
    let mut key_params = composed::key::SecretKeyParamsBuilder::default();

    // name email mix, + salt and hash as the primary_user_id
    key_params
        // change to 4096 later
        .key_type(composed::KeyType::Rsa(2048))
        .can_create_certificates(false)
        .can_sign(true)
        .can_encrypt(true)
        .passphrase(Some(password.clone()))
        .primary_user_id(generate_primary_user_id(name.clone(), email.clone()))
        .preferred_symmetric_algorithms(smallvec![crypto::sym::SymmetricKeyAlgorithm::AES256]);

    let secret_key_params = key_params
        .build()
        .expect("Must be able to create secret key params");

    let secret_key = secret_key_params
        .generate()
        .expect("Failed to generate a plain key.");

    let passwd_fn = || password.clone();

    let signed_secret_key = secret_key
        .sign(passwd_fn)
        .expect("Secret Key must be able to sign its own metadata");

    let public_key = signed_secret_key.public_key();
    let signed_public_key = public_key
        .sign(&signed_secret_key, passwd_fn)
        .expect("Public key must be able to sign its own metadata");

    let key_pair = KeyPair {
        secret_key: signed_secret_key,
        public_key: signed_public_key,
    };

    Ok(key_pair)
}

pub fn encrypt(msg: &str, pubkey_str: &str) -> Result<String, anyhow::Error> {
    let (pubkey, _) = SignedPublicKey::from_string(pubkey_str)?;
    // Requires a file name as the first arg, in this case I pass "none", as it's not used
    let msg = composed::message::Message::new_literal("none", msg);

    let mut rng = StdRng::from_entropy();
    let new_msg = msg.encrypt_to_keys(
        &mut rng,
        crypto::sym::SymmetricKeyAlgorithm::AES128,
        &[&pubkey],
    )?;
    Ok(new_msg.to_armored_string(None)?)
}

pub fn decrypt(
    armored: &str,
    seckey: &SignedSecretKey,
    password: String,
) -> Result<String, anyhow::Error> {
    let buf = Cursor::new(armored);
    let (msg, _) = composed::message::Message::from_armor_single(buf)
        .context("Failed to convert &str to armored message")?;
    let (decryptor, _) = msg
        .decrypt(|| String::from(password), &[seckey])
        .context("Decrypting the message")?;

    for msg in decryptor {
        let bytes = msg?.get_content()?.unwrap();
        let clear_text = String::from_utf8(bytes)?;
        return Ok(clear_text);
    }

    Err(anyhow::Error::msg("Failed to find message"))
}

pub fn hash_string(input: &str) -> String {
    let hash = hex_digest(Algorithm::SHA512, input.as_bytes());
    hash.to_string()
}

pub fn generate_primary_user_id(name: String, email: String) -> String {
    hash_string(&format!("{}{}{}", name, email, &get_config().unwrap().salt)).to_uppercase()
}

pub fn get_primary_key() -> Result<String> {
    let config = get_config().context("Failed to get config")?;

    let primary_key = config.primary_key.clone();

    let primary_key_location = get_vault_location()?.join(primary_key).join("public.key");

    let primary_public_key =
        std::fs::read_to_string(primary_key_location).context("Failed to read primary key")?;

    Ok(primary_public_key)
}

pub fn decrypt_full(message: String, config: &Config) -> Result<String, anyhow::Error> {
    let buf = Cursor::new(message.clone());
    let (msg, _) = composed::message::Message::from_armor_single(buf)
        .context("Failed to convert &str to armored message")?;

    let recipients = msg
        .get_recipients()
        .iter()
        .map(|e| e.encode_hex_upper())
        .collect::<Vec<String>>();

    let keys = config.keys.clone();

    let mut available_keys: Vec<String> = vec![];

    for key in keys.iter().map(|k| k.fingerprint.clone()) {
        let it_fits = recipients.iter().any(|r| key.contains(r));
        if it_fits {
            available_keys.push(key);
        }
    }

    if available_keys.len() == 0 {
        eprintln!("No keys available to decrypt this message");
        return Err(anyhow::anyhow!("No keys available to decrypt this message"));
    }

    let primary_key = config.primary_key.clone();

    let (key, fingerprint) = if available_keys.iter().any(|k| k.contains(&primary_key)) {
        println!("Using primary key");
        get_key(primary_key)
    } else {
        println!("Using first available key: {}", available_keys[0].clone());
        get_key(available_keys[0].clone())
    };

    println!("Using key: {}", fingerprint);

    let decrypted = decrypt(message.as_str(), &key, String::from("asdf"))?;

    Ok(decrypted)
}

fn get_key(fingerprint: String) -> (SignedSecretKey, String) {
    let key_dir = get_vault_location().unwrap().join(fingerprint.clone());
    let priv_key = std::fs::read_to_string(key_dir.join("private.key")).unwrap();
    let (seckey, _) = SignedSecretKey::from_string(priv_key.as_str()).unwrap();

    (seckey, fingerprint)
}
