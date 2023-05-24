use anyhow::{Context, Ok};
use base64::Engine;
use pgp::composed::PublicOrSecret;
use pgp::composed::{key::SecretKeyParamsBuilder, KeyType};
use pgp::crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm};
use pgp::types::{CompressionAlgorithm, KeyTrait, PublicKeyTrait, SecretKeyTrait};
use pgp::{SecretKey, SignedPublicKey, SignedSecretKey};
use smallvec::*;
use std::fmt::Display;
use std::io::{Cursor, Read};

use rand::CryptoRng;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[derive(Debug, Clone)]
pub(crate) struct SignedRsaKeyPair {
    pub secret_key: SignedSecretKey,
    pub public_key: SignedPublicKey,
}

impl SignedRsaKeyPair {
    /// Get the hex encoded fingerprint of the public key
    pub fn fingerprint(&self) -> String {
        let fingerprint = self.public_key.fingerprint();
        let hex = hex::encode(fingerprint);
        hex.to_uppercase()
    }

    /// Get the armored ascii string of the public key
    pub fn public_key(&self) -> String {
        self.public_key.to_armored_string(None).unwrap()
    }

    /// Get the armored ascii string of the private key
    pub fn secret_key(&self) -> String {
        self.secret_key.to_armored_string(None).unwrap()
    }
}

impl Display for SignedRsaKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.public_key.to_armored_string(None).unwrap())
    }
}

pub(crate) fn get_vault_location() -> anyhow::Result<std::path::PathBuf, anyhow::Error> {
    let path = home::home_dir()
        .context("Failed to get home directory")?
        .join(".config")
        .join("envcli")
        .join("keys");

    Ok(path)
}

pub(crate) fn rsa_gen_key(
    name: &str,
    description: &str,
    email: &str,
    password: &str,
) -> anyhow::Result<SignedRsaKeyPair, anyhow::Error> {
    let user_id = format!("{} ({}) <{}>", name, description, email);

    let mut key_params = SecretKeyParamsBuilder::default();
    key_params
        .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256,])
        .preferred_hash_algorithms(smallvec![HashAlgorithm::SHA2_256,])
        .preferred_compression_algorithms(smallvec![CompressionAlgorithm::ZLIB,])
        .key_type(KeyType::Rsa(4096))
        .can_create_certificates(true)
        .can_sign(true)
        .can_encrypt(true)
        .passphrase(Some(password.clone().into()))
        .primary_user_id(user_id);

    let secret_key_params = key_params
        .build()
        .context("Failed to build secret key params")?;

    let secret_key = secret_key_params
        .generate()
        .context("Failed to generate secret key")?;

    let passwd_fn = || password.to_string();
    let signed_secret_key = secret_key
        .clone()
        .sign(passwd_fn)
        .context("Failed to sign secret key")?;

    let public_key = signed_secret_key.public_key();
    let signed_public_key = public_key
        .clone()
        .sign(&signed_secret_key, passwd_fn)
        .context("Failed to sign public key")?;

    Ok(SignedRsaKeyPair {
        secret_key: signed_secret_key,
        public_key: signed_public_key,
    })
}

pub(crate) fn read_vault(
    fingerprint: &str,
    password: &str,
) -> anyhow::Result<SignedRsaKeyPair, anyhow::Error> {
    let vault_location = get_vault_location()?.join(fingerprint.to_uppercase());
    let priv_key = std::fs::read(vault_location.clone().join("priv.key"))?;

    let cursor_wrapped_priv_key = Cursor::new(priv_key);

    // make a keypair from the private key
    let mut dearmored = pgp::composed::from_armor_many(cursor_wrapped_priv_key)?;

    let unknown_key = dearmored.0.next().context("Failed to get key")??;
    let passwd_fn = || password.to_string();

    let signed_secret_key = unknown_key.into_secret();
    let signed_public_key = signed_secret_key
        .public_key()
        .sign(&signed_secret_key, passwd_fn)
        .context("Failed to sign public key")?;

    let keypair = SignedRsaKeyPair {
        secret_key: signed_secret_key,
        public_key: signed_public_key,
    };

    anyhow::Ok(keypair)
}

pub(crate) fn read_pub_key(fingerprint: &str) -> anyhow::Result<String, anyhow::Error> {
    Ok("".to_string())
}

pub(crate) fn encrypt(
    keypair: SignedRsaKeyPair,
    fingerprint: &str,
    message: &str,
) -> anyhow::Result<String, anyhow::Error> {
    let vault_location = get_vault_location()?.join(fingerprint.to_uppercase());
    let pub_key = std::fs::read(vault_location.clone().join("pub.key"))?;

    let cursor_wrapped_pub_key = Cursor::new(pub_key);

    // make a keypair from the public key
    let mut dearmored = pgp::composed::from_armor_many(cursor_wrapped_pub_key)?;

    let unknown_key = dearmored.0.next().context("Failed to get key")??;
    let public_key = unknown_key.into_public();

    let mut rng = ChaCha20Rng::from_entropy();

    let encrypted_message = public_key.encrypt(&mut rng, message.as_bytes())?;

    // let encrypted_message = encrypted_message

    // .first()
    // .iter()
    // .map(|x| {
    //     let y = x.as_bytes();
    //     // println!("{:?}", y);
    //     y.to_owned()
    // })
    // .collect::<Vec<_>>();

    let utf8 = base64::engine::general_purpose::STANDARD.encode(encrypted_message.first().unwrap());

    println!("{}", utf8);

    // println!("{:?}", encrypted_message.clone());

    // pgp::crypto::rsa::encrypt(rng, n, e, plaintext)

    Ok("".to_string())
}
