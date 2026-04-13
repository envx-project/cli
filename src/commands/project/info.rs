use super::*;
use crate::{
    sdk::SDK,
    utils::{choice::Choice, config::Config},
};

#[derive(Parser)]
pub struct Args {
    /// Project ID
    #[args(long, short)]
    project_id: Option<String>,

    /// Output as JSON
    #[args(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(args.project_id, &key).await?;
    let project_info = SDK::get_project_info(&project_id, &key).await?;

    if args.json {
        println!("{}", serde_json::to_string(&project_info)?);
        return Ok(());
    }

    println!("Project Info:\n");
    println!("ID: {}", project_info.project_id);
    println!("Name: {}", project_info.project_name);
    println!("Users:");
    for user in project_info.users {
        println!("    {} - {}", user.id, user.username);
    }

    Ok(())
}
