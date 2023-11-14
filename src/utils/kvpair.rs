use std::str::FromStr;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct KVPair {
    key: String,
    value: String,
}

impl KVPair {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }

    pub fn from_json(json: String) -> anyhow::Result<Self> {
        Ok(serde_json::from_str::<KVPair>(&json).context("Failed to parse KVPair")?)
    }

    pub fn to_string(&self) -> String {
        format!("{}={}", self.key, self.value)
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&self).context("Failed to serialize KVPair")?)
    }
}

impl FromStr for KVPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = &s.splitn(2, "=").collect::<Vec<&str>>();
        if split.len() != 2 {
            anyhow::bail!("Invalid key=value pair");
        }

        let key = split[0].to_uppercase().to_string();
        let value = split[1].to_string();

        Ok(Self::new(key, value))
    }
}
