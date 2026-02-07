use crate::{sdk::SDK, utils::variable::ToKVPair};

use super::{key::UnlockedKey, kvpair::KVPair};

pub async fn get_variables_magic(
    project_id: &str,
    key: &UnlockedKey,
    all: bool,
) -> anyhow::Result<Vec<KVPair>> {
    if all {
        SDK::get_variables(&project_id, &key)
            .await
            .map(|v| v.to_kvpair())
    } else {
        SDK::get_variables_pruned(&project_id, &key).await
    }
}
