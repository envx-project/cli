use super::config::{Config, Key};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

pub trait ToBTreeMap {
    fn to_btreemap(&self) -> Result<BTreeMap<String, String>>;
}

impl ToBTreeMap for Config {
    fn to_btreemap(&self) -> Result<BTreeMap<String, String>> {
        // Convert Config to JSON value
        let v: Value =
            serde_json::to_value(&self).context("Failed to convert config to JSON value")?;

        // Convert JSON value into a BTreeMap
        if let Value::Object(map) = v {
            Ok(map.into_iter().map(|(k, v)| (k, v.to_string())).collect())
        } else {
            panic!("Expected an object");
        }
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
