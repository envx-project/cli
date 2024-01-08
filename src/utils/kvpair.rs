use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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
        serde_json::from_str::<KVPair>(&json).context("Failed to parse KVPair")
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self).context("Failed to serialize KVPair")
    }
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
