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

impl EncryptedVariable {
    // pub fn to_parsed(&self) -> Result<DecryptedVariable> {
    //     Ok(DecryptedVariable {
    //         id: self.id.clone(),
    //         value: KVPair::from_json(&self.value)?,
    //         project_id: self.project_id.clone(),
    //         created_at: self.created_at.clone(),
    //     })
    // }
    //
    // pub fn zip_to_parsed(&self, kvpair: KVPair) -> DecryptedVariable {
    //     DecryptedVariable {
    //         id: self.id.clone(),
    //         value: kvpair,
    //         project_id: self.project_id.clone(),
    //         created_at: self.created_at.clone(),
    //     }
    // }
}

// pub trait ToParsed {
//     fn to_parsed(&self) -> Result<Vec<DecryptedVariable>>;
//     fn zip_to_parsed(&self, kvpair: Vec<KVPair>) -> Vec<DecryptedVariable>;
// }
//
// impl ToParsed for Vec<EncryptedVariable> {
//     fn to_parsed(&self) -> Result<Vec<DecryptedVariable>> {
//         Ok(self
//             .iter()
//             .map(|p| p.to_parsed())
//             .collect::<Result<Vec<DecryptedVariable>>>()?)
//     }
//
//     fn zip_to_parsed(
//         &self,
//         kvpairs: Vec<KVPair>,
//     ) -> Vec<DecryptedVariable> {
//         self.iter()
//             .zip(kvpairs)
//             .map(|(p, k)| p.zip_to_parsed(k))
//             .collect()
//     }
// }

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
