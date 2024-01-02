use crate::{sdk::SDK, utils::config::get_config};

use super::*;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    json: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;
    let key = config.get_key_or_default(None)?;

    let local_projects = config.projects.clone();
    let remote_projects = SDK::list_projects(&key.fingerprint)
        .await
        .context("Failed to get projects from server".red())?;
    let remote_projects = remote_projects
        .iter()
        .filter(|p| !local_projects.iter().any(|lp| lp.project_id == **p))
        .map(|p| format!("{} - {}", p, "Remote".green()));
    let local_projects = local_projects
        .iter()
        .map(|p| format!("{} - {}", p.project_id, p.path.display()));

    let combined = local_projects
        .chain(remote_projects)
        .collect::<Vec<String>>();

    if args.json {
        println!("{}", serde_json::to_string(&combined)?);
        return Ok(());
    } else {
        println!("{}", combined.join("\n"));
    }

    Ok(())
}
