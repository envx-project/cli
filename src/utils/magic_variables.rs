use std::io::Write;

use anyhow::{bail, Context};
use home::home_dir;
use pgp::composed::SignedPublicKey;

use crate::{
    sdk::SDK,
    utils::{
        kvpair::read_kvpairs_from_file, rpgp::encrypt, variable::ToKVPair,
    },
};

use super::{
    key::{Key, UnlockedKey},
    kvpair::KVPair,
};

/// Magically get variables from the API or default to ~/.envx/<fingerprint>.envx file
pub async fn get_variables_magic(
    project_id: &str,
    key: &UnlockedKey,
    all: bool,
) -> anyhow::Result<Vec<KVPair>> {
    let kvpairs = if all {
        SDK::get_variables(&project_id, &key)
            .await
            .map(|v| v.to_kvpair())
    } else {
        SDK::get_variables_pruned(&project_id, &key).await
    };

    match kvpairs {
        Ok(variables) => {
            write_variables_magic(project_id, &key.key, &variables).await?;
            Ok(variables)
        }
        Err(e) => {
            let home_dir =
                home_dir().context("Failed to get home directory")?;
            let config_dir = home_dir.join(".config/envx");
            let envx_file = config_dir.join(format!("{}.envx", &project_id));
            let envx_file = envx_file
                .to_str()
                .ok_or(anyhow::anyhow!("Failed to convert path to string"))?;

            match read_kvpairs_from_file(envx_file, &key) {
                Ok(kvpairs) => Ok(kvpairs),
                Err(err) => {
                    eprintln!(
                        "Error occurred while getting variables: {}",
                        err
                    );
                    bail!("Failed to read .envx file: {}", e);
                }
            }
        }
    }
}

async fn write_variables_magic(
    project_id: &str,
    key: &Key,
    kvpairs: &Vec<KVPair>,
) -> anyhow::Result<()> {
    let stringified_kvpairs = kvpairs
        .iter()
        .map(|kv| kv.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    let spk = [SignedPublicKey::try_from(key)?];
    let msg = encrypt(&stringified_kvpairs, &spk)?;

    let envx_file = home_dir()
        .context("Failed to get home directory")?
        .join(format!(".config/envx/{}.envx", &project_id));

    let mut file = std::fs::File::create(envx_file)?;
    file.write_all(msg.as_bytes())?;

    Ok(())
}
