use super::{config::Config, key::Key, kvpair::KVPair, settings::Settings};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

pub trait ToBTreeMap {
    fn to_btreemap(&self) -> Result<BTreeMap<String, String>>;
}

pub trait FromBTreeMap {
    fn from_btreemap(map: &BTreeMap<String, String>) -> Result<Self>
    where
        Self: Sized;
}

impl ToBTreeMap for Config {
    fn to_btreemap(&self) -> Result<BTreeMap<String, String>> {
        // Convert Config to JSON value
        let v: Value = serde_json::to_value(self)
            .context("Failed to convert config to JSON value")?;

        // Convert JSON value into a BTreeMap
        if let Value::Object(map) = v {
            Ok(map.into_iter().map(|(k, v)| (k, v.to_string())).collect())
        } else {
            Err(anyhow::anyhow!("Expected an object"))
        }
    }
}

impl ToBTreeMap for Settings {
    fn to_btreemap(&self) -> Result<BTreeMap<String, String>> {
        // Convert Settings to JSON value
        let v: Value = serde_json::to_value(self)
            .context("Failed to convert settings to JSON value")?;

        // Convert JSON value into a BTreeMap
        if let Value::Object(map) = v {
            Ok(map.into_iter().map(|(k, v)| (k, v.to_string())).collect())
        } else {
            Err(anyhow::anyhow!("Expected an object"))
        }
    }
}

impl FromBTreeMap for Settings {
    fn from_btreemap(map: &BTreeMap<String, String>) -> Result<Self> {
        // Convert BTreeMap to JSON value
        let v: Value = serde_json::to_value(map)
            .context("Failed to convert BTreeMap to JSON value")?;

        // Convert JSON value into Settings
        serde_json::from_value(v)
            .context("Failed to convert JSON value to Settings")
    }
}

impl ToBTreeMap for Vec<Key> {
    fn to_btreemap(&self) -> Result<BTreeMap<String, String>> {
        let mut map = BTreeMap::new();
        for key in self.iter() {
            // Check for duplicate fingerprints
            if map.contains_key(&key.fingerprint) {
                return Err(anyhow::anyhow!(
                    "Duplicate fingerprint found for: {}",
                    key.fingerprint
                ));
            }
            map.insert(
                key.fingerprint.chars().skip(30).collect(),
                key.primary_user_id.clone(),
            );
        }
        Ok(map)
    }
}

impl ToBTreeMap for Vec<KVPair> {
    fn to_btreemap(&self) -> Result<BTreeMap<String, String>> {
        let mut map = BTreeMap::new();
        for kvpair in self.iter() {
            // Check for duplicate keys
            if map.contains_key(&kvpair.key) {
                eprintln!("Duplicate key found: {}", kvpair.key);
            }
            map.insert(kvpair.key.clone(), kvpair.value.clone());
        }
        Ok(map)
    }
}
