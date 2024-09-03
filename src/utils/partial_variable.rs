use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::kvpair::KVPair;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EncryptedVariable {
    pub id: String,
    pub value: String,
    pub project_id: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedVariable {
    pub id: String,
    pub value: KVPair,
    pub project_id: String,
    pub created_at: String,
}

pub trait DeDupe {
    fn dedupe(&self) -> Self;
}

use std::{collections::HashMap, fmt::Display};

impl DeDupe for Vec<DecryptedVariable> {
    fn dedupe(&self) -> Self {
        // Sort the vector based on the `created_at` timestamp in descending order
        let mut sorted_vec = self.clone();
        sorted_vec.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // HashMap to track encountered keys
        let mut seen: HashMap<String, DecryptedVariable> = HashMap::new();

        // Iterate and populate the HashMap
        for variable in sorted_vec {
            let key = variable.value.key.clone();
            seen.entry(key).or_insert(variable);
        }

        // Collect the deduplicated variables into a new Vec
        seen.into_values().collect()
    }
}

pub trait ToKVPair {
    fn to_kvpair(&self) -> Vec<KVPair>;
}

impl ToKVPair for Vec<DecryptedVariable> {
    fn to_kvpair(&self) -> Vec<KVPair> {
        self.iter().map(|p| p.value.clone()).collect()
    }
}

impl Display for DecryptedVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{} - ({}) - {}",
            self.id, self.value, self.project_id
        ))?;
        Ok(())
    }
}
