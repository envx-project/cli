use std::{collections::HashSet, fmt::Display};

use anyhow::{bail, Result};
use clap::Parser;
use envx_sdk::models::RemoveUserBody;
use pgp::composed::{Deserializable, SignedPublicKey};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::header;
use serde_json::json;

use crate::{
    sdk::{api_url, SDK},
    utils::{
        choice::Choice,
        config::Config,
        prompt::{
            is_interactive, prompt_confirm_with_default, prompt_multi_options,
        },
        rpgp::encrypt,
        variable::{EncryptedVariable, ToKVPair},
    },
};

/// Remove a user from a project
#[derive(Parser)]
pub struct Args {
    /// Project ID to add user to
    #[arg(short, long)]
    project_id: Option<String>,

    /// User ID to add to project
    #[arg(short, long)]
    user_id: Option<String>,

    /// Skip confirmation prompt
    #[arg(short, long)]
    yes: bool,

    /// Output result as JSON
    #[arg(long)]
    json: bool,
}

struct DisplayUser(envx_sdk::models::User);
impl Display for DisplayUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.0.username, self.0.id)
    }
}

pub async fn command(args: Args, config: &mut Config) -> anyhow::Result<()> {
    let key = config.unlocked_primary_key()?;
    let sdk_config = config.sdk_configuration(&key)?;

    let project_id = Choice::try_project(args.project_id, &key).await?;
    let project_info = envx_sdk::apis::project_api::get_project_info_v2(
        &sdk_config,
        &project_id,
    )
    .await?;

    let (selected, selected_ids) = match args.user_id {
        Some(uid) => {
            let user = project_info
                .users
                .clone()
                .into_iter()
                .find(|u| u.id == uid)
                .ok_or(anyhow::anyhow!("User not found"))?;
            (HashSet::from([user.public_key]), vec![uid])
        }
        None => {
            if !is_interactive() {
                bail!(
                    "No --user-id given and stdin is not a terminal.\n\
                     Pass --user-id <uuid> to remove a user non-interactively.",
                );
            }
            let users = prompt_multi_options(
                "Users to Remove",
                project_info
                    .users
                    .clone()
                    .into_iter()
                    .map(DisplayUser)
                    .collect(),
            )?;
            users
                .into_iter()
                .map(|u| (u.0.public_key, u.0.id))
                .collect::<Vec<_>>()
                .into_iter()
                .unzip()
        }
    };

    if selected_ids.is_empty() {
        println!("No users selected. Aborting.");
        return Ok(());
    }

    if !args.yes {
        if !is_interactive() {
            bail!(
                "Refusing to remove users without confirmation in a non-interactive terminal.\n\
                 Re-run with --yes (-y) to confirm.",
            );
        }
        let confirmed = prompt_confirm_with_default(
            &format!(
                "Remove {} user(s) from project {}? All variables will be re-encrypted.",
                selected_ids.len(),
                &project_id,
            ),
            false,
        )?;
        if !confirmed {
            println!("Aborting...");
            return Ok(());
        }
    }

    let variables = SDK::get_variables(&project_id, &key).await?;
    let kvpairs = variables.to_kvpair();

    let pubkeys = project_info
        .users
        .into_iter()
        .map(|e| e.public_key)
        .filter(|e| !selected.contains(e))
        .map(|k| Ok(SignedPublicKey::from_string(&k)?.0))
        .collect::<Result<Vec<SignedPublicKey>>>()?;

    let messages = kvpairs
        .par_iter()
        .map(|k| encrypt(&k.to_json()?, &pubkeys))
        .collect::<Result<Vec<String>>>()?;

    let encrypted = messages
        .into_iter()
        .zip(variables.into_iter())
        .map(|(m, k)| EncryptedVariable {
            id: k.id,
            value: m,
            project_id: k.project_id,
            created_at: k.created_at,
        })
        .collect::<Vec<_>>();

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

    envx_sdk::apis::project_api::remove_users(
        &sdk_config,
        &project_id,
        RemoveUserBody {
            user_ids: selected_ids.clone(),
        },
    )
    .await?;

    if args.json {
        println!(
            "{}",
            json!({
                "project_id": project_id,
                "removed_user_ids": selected_ids,
                "updated_variable_ids": res,
            })
        );
    } else {
        println!("Updated {} variables", res.len());
        println!("IDs: {:?}", res);
        println!("Successfully removed users from project");
        println!("Users removed: {:?}", selected);
    }

    Ok(())
}
