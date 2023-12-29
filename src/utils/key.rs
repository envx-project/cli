use super::rpgp::get_vault_location;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Key {
    pub fingerprint: String,
    pub note: String,
    pub primary_user_id: String,
    pub hashed_note: String,
    pub pubkey_only: Option<bool>,
    pub uuid: Option<String>,
}

impl Key {
    pub fn public_key(&self) -> Result<String> {
        let key_location = get_vault_location()?
            .join(self.fingerprint.clone())
            .join("public.key");

        let key = fs::read_to_string(key_location).context("Failed to read public key")?;

        Ok(key)
    }

    pub fn secret_key(&self) -> Result<String> {
        let key_location = get_vault_location()?
            .join(self.fingerprint.clone())
            .join("private.key");

        let key = fs::read_to_string(key_location).context("Failed to read secret key")?;

        Ok(key)
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
