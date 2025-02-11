use super::*;
use crate::{sdk::SDK, utils::config::get_config};
use crate::utils::prompt::{prompt_text};

/// Create a new project
#[derive(Parser)]
pub struct Args {
    /// Key
    #[clap(short, long)]
    key: Option<String>,

    /// Project name
    #[clap(short, long)]
    name: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;
    let key = config.get_key_or_default(args.key)?;

    let name = args
        .name
        .unwrap_or_else(|| prompt_text("What is the name of this project?").unwrap());

    let new_project_id = SDK::new_project(&key.fingerprint, &name).await?;
    println!("Created new project with ID: {}", new_project_id);
    Ok(())
}
