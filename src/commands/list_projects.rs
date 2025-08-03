use super::*;
use crate::utils::config::Config;

#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, config: Config) -> Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);
    let sdk_config = config.sdk_configuration(&key)?;
    let projects =
        envx_sdk::apis::projects_api::list_projects_v2(&sdk_config).await?;

    println!("Projects:");
    for project in projects {
        println!("\t{} - {}", project.project_id, project.project_name);
    }
    Ok(())
}
