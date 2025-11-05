use envx_sdk::models::UpdateProjectV2;

use super::*;
use crate::utils::choice::Choice;
use crate::utils::config::Config;
use crate::utils::prompt::prompt_text;

/// Rename a project
#[derive(Parser)]
pub struct Args {
    /// New name for the project
    #[clap(short, long)]
    name: Option<String>,

    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(args.project_id, &key).await?;
    let new_name = args.name.unwrap_or_else(|| {
        prompt_text("New name for project").expect("Failed to prompt")
    });

    let sdk_config = config.sdk_configuration(&key)?;
    envx_sdk::apis::project_api::update(
        &sdk_config,
        &project_id,
        UpdateProjectV2 {
            project_name: Some(Some(new_name)),
        },
    )
    .await?;

    Ok(())
}
