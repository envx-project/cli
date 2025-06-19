use super::rpgp::get_vault_location;
use anyhow::Result;
use pgp::Deserializable;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Key {
    pub fingerprint: String,
    pub note: String,
    pub primary_user_id: String,
    pub pubkey_only: Option<bool>,
    pub uuid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KeyType {
    Public,
    Secret,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::Public => write!(f, "public"),
            KeyType::Secret => write!(f, "secret"),
        }
    }
}

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Failed to read {0} key")]
    FailedToRead(KeyType),
    #[error("Failed to parse {0} key")]
    FailedToParse(KeyType),
    #[error("Unknown Error: {0}")]
    Unknown(anyhow::Error),
}

impl From<anyhow::Error> for KeyError {
    fn from(e: anyhow::Error) -> Self {
        KeyError::Unknown(e)
    }
}

impl Key {
    pub fn public_key_str(&self) -> Result<String, KeyError> {
        let key_location = get_vault_location()?
            .join(self.fingerprint.clone())
            .join("public.key");

        let key = fs::read_to_string(key_location)
            .map_err(|_| KeyError::FailedToRead(KeyType::Public))?;

        Ok(key)
    }

    fn signed_public_key(&self) -> Result<pgp::SignedPublicKey, KeyError> {
        let key = self.public_key_str()?;
        let (pubkey, _) = pgp::SignedPublicKey::from_string(key.as_str())
            .map_err(|_| KeyError::FailedToParse(KeyType::Public))?;

        Ok(pubkey)
    }

    pub fn secret_key_str(&self) -> Result<String, KeyError> {
        let key_location = get_vault_location()?
            .join(self.fingerprint.clone())
            .join("private.key");

        let key = fs::read_to_string(key_location)
            .map_err(|_| KeyError::FailedToRead(KeyType::Secret))?;

        Ok(key)
    }

    fn signed_secret_key(&self) -> Result<pgp::SignedSecretKey, KeyError> {
        let key = self.secret_key_str()?;
        let (seckey, _) = pgp::SignedSecretKey::from_string(key.as_str())
            .map_err(|_| KeyError::FailedToParse(KeyType::Secret))?;

        Ok(seckey)
    }
}

impl TryInto<pgp::SignedSecretKey> for Key {
    type Error = KeyError;

    fn try_into(self) -> Result<pgp::SignedSecretKey, Self::Error> {
        self.signed_secret_key()
    }
}

impl TryInto<pgp::SignedPublicKey> for Key {
    type Error = KeyError;

    fn try_into(self) -> Result<pgp::SignedPublicKey, Self::Error> {
        self.signed_public_key()
    }
}

impl TryFrom<&Key> for pgp::SignedSecretKey {
    type Error = KeyError;

    fn try_from(key: &Key) -> Result<Self, Self::Error> {
        key.signed_secret_key()
    }
}

impl TryFrom<&Key> for pgp::SignedPublicKey {
    type Error = KeyError;

    fn try_from(key: &Key) -> Result<Self, Self::Error> {
        key.signed_public_key()
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
