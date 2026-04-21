use std::collections::HashSet;

use anyhow::bail;
use envx_sdk::models::{AcceptInviteBody, UpdateManyBody, Variable};
use uuid::Uuid;

use crate::utils::{
    symmetric::password_decrypt_from_armor,
    variable::{DecryptedVariable, ToKVPair},
};

use super::*;

use crate::utils::{config::Config, rpgp::encrypt};
use pgp::composed::{Deserializable, SignedPublicKey};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

#[derive(Parser)]
pub struct Args {
    invite_code: String,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);
    let sdk_config = config.sdk_configuration(&key)?;

    let [sym, code, verifier]: [Uuid; 3] = args
        .invite_code
        .split(':')
        .map(Uuid::parse_str)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| anyhow!("invalid invite code"))?;

    println!("sym: {sym}");
    println!("code: {code}");
    println!("ver: {verifier}");

    let response = envx_sdk::apis::invite_api::accept_invite(
        &sdk_config,
        AcceptInviteBody { verifier, code },
    )
    .await
    .context("Failed to accept invite")?;

    let decrypted =
        password_decrypt_from_armor(&response.ciphertext, &sym.to_string())?;
    let variables: Vec<DecryptedVariable> = serde_json::from_str(&decrypted)?;
    let kvpairs = variables.to_kvpair();

    if variables.is_empty() {
        bail!("No variables found");
    }

    let project_info = envx_sdk::apis::project_api::get_project_info_v2(
        &sdk_config,
        &response.project_id,
    )
    .await
    .context("Failed to get project info")?;

    let mut recipients = project_info
        .users
        .iter()
        .map(|e| e.public_key.clone())
        .collect::<HashSet<String>>();
    recipients.insert(key.key.public_key_str()?);

    let pubkeys = recipients
        .par_iter()
        .map(|k| Ok(SignedPublicKey::from_string(k)?.0))
        .collect::<Result<Vec<SignedPublicKey>>>()?;

    let messages = kvpairs
        .par_iter()
        .map(|k| encrypt(&k.to_json()?, &pubkeys))
        .collect::<Result<Vec<String>>>()?;

    let encrypted = messages
        .into_iter()
        .zip(variables.into_iter())
        .map(|(m, k)| Variable {
            id: k.id,
            value: m,
            project_id: k.project_id,
        })
        .collect::<Vec<_>>();

    let res = envx_sdk::apis::variables_api::update_many(
        &sdk_config,
        UpdateManyBody {
            variables: encrypted,
        },
    )
    .await
    .context("Failed to update variables")?;

    println!("Updated {} variables", res.len());
    println!("IDs: {:?}", res);

    println!("\nSuccessfully joined project {}", response.project_id);

    Ok(())
}
