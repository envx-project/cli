use anyhow::{Context, Result};
use pgp::composed::{Message, SignedSecretKey};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::key::UnlockedKey;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KVPair {
    pub key: String,
    pub value: String,
}

impl KVPair {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str::<KVPair>(json).context("Failed to parse KVPair")
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self).context("Failed to serialize KVPair")
    }
}

pub fn read_kvpairs_from_file(
    file_name: &str,
    key: &UnlockedKey,
) -> Result<Vec<KVPair>> {
    let msg = Message::from_file(&file_name)?;

    let ssk = SignedSecretKey::try_from(key)?;
    let mut decrypted = msg
        .decrypt(&key.password.clone().into(), &ssk)
        .context("Decrypting the message")?;

    if decrypted.is_compressed() {
        decrypted = decrypted
            .decompress()
            .context("Failed to decompress message")?;
    }

    decrypted
        .as_data_string()?
        .split("\n")
        .map(|s| KVPair::from_str(s))
        .collect::<Result<Vec<KVPair>>>()
}

impl fmt::Display for KVPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

impl FromStr for KVPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = &s.splitn(2, '=').collect::<Vec<&str>>();
        if split.len() != 2 {
            anyhow::bail!("Invalid key=value pair");
        }

        let key = split[0].to_uppercase().to_string();
        let value = split[1].to_string();

        Ok(Self::new(key, value))
    }
}
