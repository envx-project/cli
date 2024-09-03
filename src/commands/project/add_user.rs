use super::*;
use crate::{
    sdk::{get_api_url, SDK},
    utils::{
        auth::get_token,
        choice::Choice,
        config::get_config,
        prompt::prompt_text,
        rpgp::encrypt_multi,
        variable::{EncryptedVariable, ToKVPair},
    },
};
use pgp::{Deserializable, SignedPublicKey};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::header;
use serde_json::json;
use std::collections::HashSet;

/// Add a user to a project
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

pub async fn command(args: Args) -> Result<()> {
    let user_id = match args.user_id {
        Some(u) => u,
        None => prompt_text("User ID: ")?,
    };
    let user_id = user_id.trim().to_string();

    let config = get_config()?;
    let key = config.get_key_or_default(args.key)?;

    let uuid = key
        .uuid
        .context("Key does not have a UUID, try `envx upload`")?;
    let (_, public_key) = SDK::get_user(&key.fingerprint, &user_id)
        .await
        .context("Failed to get user, is the user ID correct?")?;

    let project_id =
        Choice::try_project(args.project_id, &key.fingerprint).await?;

    let project_info =
        SDK::get_project_info(&project_id, &key.fingerprint).await?;

    let variables = SDK::get_variables(&project_id, &key.fingerprint).await?;
    let kvpairs = variables.to_kvpair();

    let mut recipients = project_info
        .users
        .iter()
        .map(|e| e.public_key.clone())
        .collect::<Vec<String>>();

    recipients.push(public_key);

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

    SDK::add_user_to_project(&key.fingerprint, &user_id, &project_id).await?;

    Ok(())
}
