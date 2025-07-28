use super::*;
use crate::utils::choice::Choice;
use crate::utils::prompt::prompt_text;
use crate::{sdk::SDK, utils::config::Config};

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

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get().await;
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(args.project_id, &key).await?;
    let new_name = args.name.unwrap_or_else(|| {
        prompt_text("New name for project").expect("Failed to prompt")
    });

    SDK::rename_project(&project_id, &new_name, &key).await
}
