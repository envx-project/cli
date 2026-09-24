use super::kvpair::KVPair;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};

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
        sorted_vec.sort_by_cached_key(|variable| {
            std::cmp::Reverse((
                variable
                    .created_at
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .ok(),
                variable.created_at.clone(),
                variable.id.clone(),
            ))
        });

        let mut seen: BTreeMap<String, DecryptedVariable> = BTreeMap::new();

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

#[cfg(test)]
mod tests {
    use super::*;
    fn variable(id: &str, value: &str, time: &str) -> DecryptedVariable {
        DecryptedVariable {
            id: id.into(),
            project_id: "project".into(),
            value: KVPair {
                key: "KEY".into(),
                value: value.into(),
            },
            created_at: time.into(),
        }
    }
    #[test]
    fn newest_instant_wins_across_timezone_offsets() {
        let old = variable("z", "old", "2026-09-24T10:00:00+02:00");
        let new = variable("a", "new", "2026-09-24T09:00:00Z");
        assert_eq!(vec![old, new].dedupe()[0].value.value, "new");
    }
    #[test]
    fn equal_timestamps_use_stable_id_tiebreaker() {
        let a = variable("a", "a", "2026-09-24T09:00:00Z");
        let b = variable("b", "b", "2026-09-24T09:00:00Z");
        assert_eq!(vec![a.clone(), b.clone()].dedupe()[0].id, "b");
        assert_eq!(vec![b, a].dedupe()[0].id, "b");
    }
}
