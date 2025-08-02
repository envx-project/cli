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
use uuid::Uuid;

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
    #[clap(trailing_var_arg = true)]
    user_ids: Vec<Uuid>,
}

pub async fn command(args: Args, config: Config) -> Result<()> {
    let user_ids = if args.user_ids.is_empty() {
        vec![prompt_text("User ID: ")?.parse::<Uuid>()?]
    } else {
        args.user_ids
    };
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);
    let sdk_config = config.sdk_configuration(&key)?;

    let project_id = Choice::try_project(args.project_id, &key).await?;
    let project_info = envx_sdk::apis::project_api::get_project_info_v2(
        &sdk_config,
        &project_id,
    )
    .await?;

    let variables = SDK::get_variables(&project_id, &key).await?;
    let kvpairs = variables.to_kvpair();

    let users =
        envx_sdk::apis::user_api::get_many_users(&sdk_config, user_ids.clone())
            .await?;

    let mut recipients = project_info
        .users
        .iter()
        .map(|e| e.public_key.clone())
        .collect::<HashSet<String>>();
    recipients.extend(users.into_iter().map(|u| u.public_key));
    recipients.insert(key.key.public_key_str()?);

    let pubkeys = recipients
        .par_iter()
        .map(|k| Ok(SignedPublicKey::from_string(k)?.0))
        .collect::<Result<Vec<SignedPublicKey>>>()?;

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
    envx_sdk::apis::project_api::add_user(&sdk_config, &project_id, user_ids)
        .await?;

    Ok(())
}
