use super::*;
use crate::types::PartialUser;
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

    /// Show all info
    #[clap(short, long)]
    all: bool,
}

// TODO: Pretty print project info (in a table?)
pub async fn command(args: Args, config: Config) -> Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);
    let project_id = Choice::try_project(args.project_id, &key).await?;
    let project_info = SDK::get_project_info(&project_id, &key).await?;

    if args.json && args.all {
        println!("{}", serde_json::to_string(&project_info.users)?);
        return Ok(());
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string(
                &project_info
                    .users
                    .iter()
                    .map(|u| u.clone().into())
                    .collect::<Vec<PartialUser>>()
            )?
        );
        return Ok(());
    }

    if args.all {
        for user in project_info.users.iter() {
            println!(
                "{} - {} - {} - {}",
                user.username, user.id, user.created_at, user.public_key
            );
        }
        println!("{:?}", &project_info.users);
        return Ok(());
    }

    for user in project_info.users.iter() {
        println!("{}", user);
    }

    Ok(())
}
