use super::*;
use crate::utils::config::Config;

#[derive(Parser)]
pub struct Args {
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);
    let sdk_config = config.sdk_configuration(&key)?;
    let projects =
        envx_sdk::apis::projects_api::list_projects_v2(&sdk_config).await?;

    if args.json {
        println!("{}", serde_json::to_string(&projects)?);
        return Ok(());
    }

    println!("Projects:");
    for project in projects {
        println!("\t{} - {}", project.project_id, project.project_name);
    }
    Ok(())
}
