use super::*;
use crate::utils::{
    choice::Choice,
    config::Config,
    project_snapshot::{self, Client, VERSION},
    prompt::{is_interactive, prompt_text},
};
use anyhow::bail;
use serde_json::json;
use uuid::Uuid;

/// Add a user to a project
#[derive(Parser)]
pub struct Args {
    /// Project ID to add user to
    #[arg(short, long)]
    project_id: Option<String>,

    /// User ID to add to project
    #[arg(trailing_var_arg = true)]
    user_ids: Vec<Uuid>,

    /// Output result as JSON
    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let user_ids = if args.user_ids.is_empty() {
        if !is_interactive() {
            bail!(
                "No user IDs given and stdin is not a terminal.\n\
                 Pass user IDs as positional args: `envx project add-users <uuid>...`",
            );
        }
        vec![prompt_text("User ID: ")?.parse::<Uuid>()?]
    } else {
        args.user_ids
    };

    let key = config.unlocked_primary_key()?;
    let client = Client::new(config, &key)?;
    let project_id = Choice::try_project(config, args.project_id, &key).await?;
    let snapshot = client.snapshot(&project_id, &user_ids).await?;
    let variables = project_snapshot::decrypt(&snapshot, &key)?;
    let encrypted = project_snapshot::rewrap(
        &variables,
        &project_id,
        &snapshot.users,
        &key,
    )?;
    client.post(&format!("/v2/project/{project_id}/rewrap"),&json!({"protocol_version":VERSION,"snapshot":snapshot.snapshot,"variables":encrypted,"add_user_ids":user_ids})).await?;
    let updated: Vec<_> = variables.iter().map(|v| &v.id).collect();
    if args.json {
        println!(
            "{}",
            json!({"project_id":project_id,"added_user_ids":user_ids,"updated_variable_ids":updated})
        );
    } else {
        println!(
            "Added {} users and rewrapped {} variables",
            user_ids.len(),
            updated.len()
        );
    }
    Ok(())
}
