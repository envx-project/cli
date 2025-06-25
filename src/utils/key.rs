use super::{auth::AuthToken, rpgp::get_vault_location};
use anyhow::{bail, Context, Result};
use pgp::{crypto, ArmorOptions, Deserializable, Message};
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

pub struct UnlockedKey {
    pub password: String,
    pub key: Key,
}

impl UnlockedKey {
    pub fn new(password: String, key: Key) -> Self {
        Self { password, key }
    }

    pub fn auth_token(&self) -> Result<crate::utils::auth::AuthToken> {
        let key = self
            .key
            .signed_secret_key()
            .context("Failed to get secret key")?;

        let msg = Message::new_literal("none", &chrono::Utc::now().to_string());

        let pw = || self.password.to_string();

        let rng = rand::rngs::OsRng;
        let signature =
            msg.sign(rng, &key, pw, crypto::hash::HashAlgorithm::SHA3_512);

        let signature = match signature {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to sign API authentication challenge: {}", e);
                if let pgp::errors::Error::Incomplete(_) = e {
                    eprintln!("This is most likely due to a missing or incorrect passphrase.");
                    println!(
                    "You can view the saved passphrase with 'envx keyring view [fingerprint]'"
                );
                    println!("This command is interactive");
                    // println!("Or you can check against the saved passphrase with 'envx keyring check -k <fingerprint> -p <passphrase>'");
                    // println!("Both of these commands are interactive")
                }

                bail!("Failed to sign API authentication challenge");
            }
        };

        let signature = signature
            .to_armored_string(ArmorOptions::default())
            .context("Failed to convert signature to armored string")?;

        let auth_token =
            AuthToken::new(self.key.uuid.clone().unwrap().into(), signature);

        Ok(auth_token)
    }
}

impl Into<Key> for UnlockedKey {
    fn into(self) -> Key {
        self.key
    }
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
    pub fn unlock(self, password: &str) -> UnlockedKey {
        UnlockedKey {
            password: password.to_string(),
            key: self,
        }
    }
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
