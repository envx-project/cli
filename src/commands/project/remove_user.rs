use std::collections::HashSet;

use anyhow::{Context, Result};
use clap::Parser;
use pgp::{Deserializable, SignedPublicKey};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::header;
use serde_json::json;

use crate::{
    sdk::{get_api_url, SDK},
    types::User,
    utils::{
        auth::get_token,
        choice::Choice,
        config::get_config,
        partial_variable::{EncryptedVariable, ToKVPair},
        prompt::prompt_multi_options,
        rpgp::encrypt_multi,
    },
};

/// Remove a user from a project
#[derive(Parser)]
pub struct Args {
    /// Key to sign with
    #[clap(short, long)]
    key: Option<String>,

    /// Project ID to add user to
    #[clap(short, long)]
    project_id: Option<String>,

    /// User ID to add to project
    #[clap(short, long)]
    user_id: Option<String>,
}

pub async fn command(args: Args) -> anyhow::Result<()> {
    let config = get_config()?;
    let key = config.get_key_or_default(args.key)?;

    let uuid = key
        .uuid
        .context("Key does not have a UUID, try `envx upload`")?;

    let project_id =
        Choice::try_project(args.project_id, &key.fingerprint).await?;
    let project_info =
        SDK::get_project_info(&project_id, &key.fingerprint).await?;

    let users_to_remove = match args.user_id {
        Some(u) => vec![u],
        None => {
            let users = prompt_multi_options(
                "Users to Remove",
                project_info.users.clone(),
            )?;
            users.into_iter().map(|u| u.id).collect()
        }
    };

    let variables = SDK::get_variables(&project_id, &key.fingerprint).await?;
    let kvpairs = variables.to_kvpair();

    let users_without_users_to_remove = project_info
        .users
        .into_iter()
        .filter(|u| !users_to_remove.contains(&u.id))
        .collect::<Vec<User>>();

    let recipients = users_without_users_to_remove
        .iter()
        .map(|e| e.public_key.clone())
        .collect::<Vec<String>>();

    let recipients = recipients
        .into_iter()
        .collect::<HashSet<String>>()
        .into_iter()
        .collect::<Vec<String>>();

    let pubkeys = recipients
        .iter()
        .map(|k| Ok(SignedPublicKey::from_string(k)?.0))
        .collect::<Result<Vec<SignedPublicKey>>>()?;

    let messages = kvpairs
        .par_iter()
        .map(|k| encrypt_multi(&k.to_json()?, &pubkeys))
        .collect::<Result<Vec<String>>>()?;

    let encrypted: Vec<EncryptedVariable> = messages
        .into_iter()
        .zip(variables.into_iter())
        .map(|(m, k)| EncryptedVariable {
            id: k.id,
            value: m,
            project_id: k.project_id,
            created_at: k.created_at,
        })
        .collect();

    let body = json!({
        "variables": encrypted,
    });

    let client = reqwest::Client::new();
    let auth_token = get_token(&key.fingerprint, &uuid).await?;

    let url = get_api_url().join("/variables/update-many")?;

    let res = client
        .post(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", auth_token))
        .json(&body)
        .send()
        .await?
        .json::<Vec<String>>()
        .await?;

    println!("Updated {} variables", res.len());
    println!("IDs: {:?}", res);

    SDK::remove_users_from_project(
        &key.fingerprint,
        users_to_remove.clone(),
        &project_info.project_id,
    )
    .await?;

    println!("Successfully removed users from project");
    println!("Users removed: {:?}", users_to_remove);

    Ok(())
}
