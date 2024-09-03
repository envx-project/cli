use super::kvpair::KVPair;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

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

impl DeDupe for Vec<DecryptedVariable> {
    fn dedupe(&self) -> Self {
        let mut sorted_vec = self.clone();
        sorted_vec.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let mut seen: HashMap<String, DecryptedVariable> = HashMap::new();

        for variable in sorted_vec {
            let key = variable.value.key.clone();
            seen.entry(key).or_insert(variable);
        }

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
