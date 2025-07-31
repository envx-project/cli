use super::*;
use crate::{
    sdk::{api_url, SDK},
    utils::{
        choice::Choice,
        config::Config,
        prompt::prompt_text,
        rpgp::encrypt,
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

pub async fn command(args: Args, config: Config) -> Result<()> {
    let user_id = match args.user_id {
        Some(u) => u,
        None => prompt_text("User ID: ")?,
    };
    let user_id = user_id.trim().to_string();

    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);

    let project_id = Choice::try_project(args.project_id, &key).await?;

    let project_info = SDK::get_project_info(&project_id, &key).await?;

    let variables = SDK::get_variables(&project_id, &key, &config).await?;
    let kvpairs = variables.to_kvpair();

    let recipients = project_info
        .users
        .iter()
        .map(|e| e.public_key.clone())
        .collect::<HashSet<String>>();

    let mut pubkeys = recipients
        .iter()
        .map(|k| Ok(SignedPublicKey::from_string(k)?.0))
        .collect::<Result<Vec<SignedPublicKey>>>()?;
    pubkeys.push(pgp::SignedPublicKey::try_from(&key.key)?);

    let messages = kvpairs
        .par_iter()
        .map(|k| encrypt(&k.to_json()?, &pubkeys))
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
    let auth_token = key.auth_token()?.bearer();

    let url = api_url().join("/variables/update-many")?;

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

    SDK::add_user_to_project(&key, &user_id, &project_id).await?;

    Ok(())
}
