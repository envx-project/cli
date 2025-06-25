use crate::{
    sdk::SDK,
    utils::{choice::Choice, config::Config},
};

use super::*;

#[derive(Parser)]
pub struct Args {
    /// Project ID that you want do delete
    project: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = Config::get()?;
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;

    let key = key.unlock(&password);

    let project_id = Choice::try_project(args.project, &key).await?;

    SDK::delete_project(&key, &project_id).await?;
    config.delete_project(&project_id)?;
    println!("Project {} deleted", &project_id);

    Ok(())
}
