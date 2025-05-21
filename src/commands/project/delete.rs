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

    let project_id =
        Choice::try_project(args.project, &key.fingerprint).await?;

    SDK::delete_project(&project_id, &key.fingerprint).await?;
    config.delete_project(&project_id)?;
    println!("Project {} deleted", &project_id);

    Ok(())
}
