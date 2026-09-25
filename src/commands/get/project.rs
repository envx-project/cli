use super::*;
use crate::utils::config::Config;
use crate::{sdk::SDK, utils::choice::Choice};

/// Get project info
#[derive(Parser)]
pub struct Args {
    /// Show full IDs and diagnostic details
    #[arg(long)]
    pub verbose: bool,

    /// Partial fingerprint of key to use
    #[arg(short, long)]
    key: Option<String>,

    /// Project ID
    #[arg(short, long)]
    project_id: Option<String>,

    /// Output in JSON format
    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);
    let project_id = Choice::try_project(config, args.project_id, &key).await?;
    let project_info = SDK::get_project_info(config, &project_id, &key).await?;

    if args.json {
        println!("{}", serde_json::to_string(&project_info)?);
        return Ok(());
    }

    let names = crate::utils::user_display::UserDisplay::new(config)?;
    println!("Project Info:\n");
    println!("ID: {}", project_info.project_id);
    println!("Name: {}", project_info.project_name);
    println!("Users:");
    for user in project_info.users {
        println!("    {}", names.row(&user.id, &user.username, args.verbose));
    }

    Ok(())
}
