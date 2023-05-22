use anyhow::Context;
use pgp::composed::{key::SecretKeyParamsBuilder, KeyType};
use pgp::crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm};
use pgp::types::{CompressionAlgorithm, KeyTrait, SecretKeyTrait};
use pgp::{SignedPublicKey, SignedSecretKey};
use smallvec::*;
use std::fmt::Display;

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
