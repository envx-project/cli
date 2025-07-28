use super::*;
use crate::utils::config::Config;
use crate::{sdk::SDK, utils::choice::Choice};

/// Get all environment variables for a project
#[derive(Parser)]
pub struct Args {
    /// Partial fingerprint of key to use
    #[clap(short, long)]
    key: Option<String>,

    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,

    /// Output in JSON format
    #[clap(long)]
    json: bool,
}

// TODO: Pretty print project info (in a table?)
pub async fn command(args: Args) -> Result<()> {
    let config = Config::get().await;
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);
    let project_id = Choice::try_project(args.project_id, &key).await?;
    let project_info = SDK::get_project_info(&project_id, &key).await?;
    println!("{:?}", project_info);
    Ok(())
}
