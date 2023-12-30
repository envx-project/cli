use super::*;
use crate::{sdk::SDK, utils::config::get_config};

/// Create a new project
#[derive(Parser)]
pub struct Args {
    /// Key
    #[clap(short, long)]
    key: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;
    let key = config.get_key_or_default(args.key)?;
    let new_project_id = SDK::new_project(&key.fingerprint).await?;
    println!("Created new project with ID: {}", new_project_id);
    Ok(())
}
