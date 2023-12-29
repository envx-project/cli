use serde::{Deserialize, Serialize};

use super::kvpair::KVPair;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PartialVariable {
    pub id: String,
    pub value: String,
    pub project_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedPartialVariable {
    pub id: String,
    pub value: KVPair,
    pub project_id: String,
}
