use super::*;
use crate::utils::choice::Choice;
use crate::utils::config::get_config;

/// Get all environment variables for a project
#[derive(Parser)]
pub struct Args {
    /// Partial fingerprint of key to use
    #[clap(short, long)]
    key: Option<String>,

    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = get_config()?;
    let key = config.get_key_or_default(args.key)?;

    let project_id = match args.project_id {
        Some(p) => p,
        None => Choice::choose_project(&key.fingerprint).await?,
    };

    config.set_project(&project_id)?;

    Ok(())
}
