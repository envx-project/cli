use super::rpgp::get_vault_location;
use anyhow::{Context, Result};
use pgp::Deserializable;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Key {
    pub fingerprint: String,
    pub note: String,
    pub primary_user_id: String,
    pub pubkey_only: Option<bool>,
    pub uuid: Option<String>,
}

#[allow(dead_code)]
impl Key {
    pub fn public_key_str(&self) -> Result<String> {
        let key_location = get_vault_location()?
            .join(self.fingerprint.clone())
            .join("public.key");

        let key = fs::read_to_string(key_location)
            .context("Failed to read public key")?;

        Ok(key)
    }

    pub fn signed_public_key(&self) -> Result<pgp::SignedPublicKey> {
        let key = self.public_key_str()?;
        let (pubkey, _) = pgp::SignedPublicKey::from_string(key.as_str())
            .context("Failed to convert public key to string")?;

        Ok(pubkey)
    }

    pub fn secret_key_str(&self) -> Result<String> {
        let key_location = get_vault_location()?
            .join(self.fingerprint.clone())
            .join("private.key");

        let key = fs::read_to_string(key_location)
            .context("Failed to read secret key")?;

        Ok(key)
    }

    pub fn signed_secret_key(&self) -> Result<pgp::SignedSecretKey> {
        let key = self.secret_key_str()?;
        let (seckey, _) = pgp::SignedSecretKey::from_string(key.as_str())
            .context("Failed to convert private key to string")?;

        Ok(seckey)
    }
}

pub trait VecKeyTrait {
    fn all_fingerprints(&self) -> Vec<&str>;
}

impl VecKeyTrait for Vec<Key> {
    fn all_fingerprints(&self) -> Vec<&str> {
        self.iter().map(|k| k.fingerprint.as_str()).collect()
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - ({})", self.fingerprint, self.primary_user_id)
    }
}
